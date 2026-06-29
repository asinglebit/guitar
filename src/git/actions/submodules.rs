use crate::{
    git::actions::gix_support::{checkout_tree, edit_repo_config, load_config_file, open_repo, set_head_to_object, to_git2_error, write_index},
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    helpers::localisation::network,
};
use git2::Repository;
use gix::bstr::ByteSlice;
use gix::sec::trust::DefaultForLevel;
use std::{sync::atomic::AtomicBool, thread};

fn find_submodule<'repo>(repo: &'repo gix::Repository, name: &str) -> Result<Option<gix::Submodule<'repo>>, git2::Error> {
    let Some(mut submodules) = repo.submodules().map_err(to_git2_error)? else {
        return Ok(None);
    };
    let wanted = name.as_bytes().as_bstr();
    Ok(submodules.find(|submodule| submodule.name() == wanted || submodule.path().is_ok_and(|path| path.as_ref() == wanted)))
}

fn submodule_path(submodule: &gix::Submodule<'_>) -> Result<gix::bstr::BString, git2::Error> {
    submodule.path().map(|path| path.into_owned()).map_err(to_git2_error)
}

fn stage_commit_pointer(index: &mut gix::index::File, path: &gix::bstr::BStr, oid: gix::ObjectId) {
    if let Some(entry) = index.entry_mut_by_path_and_stage(path, gix::index::entry::Stage::Unconflicted) {
        entry.id = oid;
        entry.mode = gix::index::entry::Mode::COMMIT;
        return;
    }

    index.dangerously_push_entry(Default::default(), oid, gix::index::entry::Flags::empty(), gix::index::entry::Mode::COMMIT, path);
}

fn submodule_url_from_modules(repo: &gix::Repository, submodule: &gix::Submodule<'_>) -> Result<gix::Url, git2::Error> {
    let workdir = repo.workdir().ok_or_else(|| git2::Error::from_str("Repository has no working directory"))?;
    let modules = load_config_file(workdir.join(".gitmodules"), gix::config::Source::Local)?;
    let submodule_name = submodule.name().to_str().map_err(|_| git2::Error::from_str("Submodule name is not valid UTF-8"))?;
    let key = format!("submodule.{submodule_name}.url");
    let url = modules.string(key).ok_or_else(|| git2::Error::from_str("Submodule URL is missing"))?;
    gix::Url::from_bytes(url.as_ref()).map_err(to_git2_error)
}

fn update_repo_config(repo: &gix::Repository, update: impl FnOnce(&mut gix::config::File<'static>) -> Result<(), git2::Error>) -> Result<(), git2::Error> {
    edit_repo_config(repo, |config| {
        update(config)?;
        Ok(true)
    })
}

fn sync_superproject_submodule_url(repo: &gix::Repository, submodule: &gix::Submodule<'_>, url: &gix::bstr::BStr) -> Result<(), git2::Error> {
    update_repo_config(repo, |config| {
        let section = "submodule";
        let subsection = Some(submodule.name());
        config.set_raw_value_by(section, subsection, "url", url).map(drop).map_err(to_git2_error)
    })
}

fn sync_checked_out_submodule_url(submodule: &gix::Repository, url: &gix::Url) -> Result<(), git2::Error> {
    let remote = submodule.find_fetch_remote(None).map_err(to_git2_error)?;
    update_repo_config(submodule, |config| {
        let remote = remote.with_url(url.clone()).map_err(to_git2_error)?;
        remote.save_to(config).map_err(to_git2_error)
    })
}

fn open_or_init_submodule_repo(submodule: &gix::Submodule<'_>) -> Result<(gix::Repository, bool), git2::Error> {
    if let Some(repo) = submodule.open().map_err(to_git2_error)? {
        return Ok((repo, false));
    }

    let workdir = submodule.work_dir().map_err(to_git2_error)?;
    let create_opts = gix::create::Options { destination_must_be_empty: true, ..Default::default() };
    let repo = gix::ThreadSafeRepository::init_opts(workdir, gix::create::Kind::WithWorktree, create_opts, gix::open::Options::default_for_level(gix::sec::Trust::Full))
        .map_err(to_git2_error)?
        .to_thread_local();
    Ok((repo, true))
}

fn configure_submodule_remote<'repo>(repo: &'repo gix::Repository, url: gix::Url) -> Result<gix::Remote<'repo>, git2::Error> {
    let mut remote = repo.remote_at_without_url_rewrite(url).map_err(to_git2_error)?;
    remote = remote.with_fetch_tags(gix::remote::fetch::Tags::All);
    remote = remote.with_refspecs(Some("+refs/heads/*:refs/remotes/origin/*"), gix::remote::Direction::Fetch).map_err(to_git2_error)?;

    update_repo_config(repo, |config| remote.save_as_to("origin", config).map_err(to_git2_error))?;
    Ok(remote)
}

fn checkout_submodule_commit(repo: &mut gix::Repository, target_oid: gix::ObjectId, newly_initialized: bool) -> Result<(), git2::Error> {
    let tree_id = {
        let commit = repo.find_commit(target_oid).map_err(to_git2_error)?;
        commit.tree().map_err(to_git2_error)?.id
    };
    checkout_tree(repo, tree_id, newly_initialized)?;
    set_head_to_object(repo, target_oid, "submodule update")
}

pub fn sync_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let gix_repo = open_repo(repo)?;
    let Some(submodule) = find_submodule(&gix_repo, name)? else {
        return Err(git2::Error::from_str("Submodule not found"));
    };
    let url = submodule_url_from_modules(&gix_repo, &submodule)?;
    let url_bstring = url.to_bstring();
    sync_superproject_submodule_url(&gix_repo, &submodule, url_bstring.as_ref())?;
    if let Some(submodule_repo) = submodule.open().map_err(to_git2_error)? {
        sync_checked_out_submodule_url(&submodule_repo, &url)?;
    }
    Ok(())
}

pub fn stage_submodule_head(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let gix_repo = open_repo(repo)?;
    let Some(submodule) = find_submodule(&gix_repo, name)? else {
        return Err(git2::Error::from_str("Submodule not found"));
    };
    let path = submodule_path(&submodule)?;
    let sub_repo = submodule.open().map_err(to_git2_error)?.ok_or_else(|| git2::Error::from_str("Submodule is not initialized"))?;
    let head_oid = sub_repo.head_id().map_err(to_git2_error)?.detach();
    let mut index = gix_repo.index_or_load_from_head_or_empty().map_err(to_git2_error)?.into_owned();

    stage_commit_pointer(&mut index, path.as_ref(), head_oid);
    write_index(&mut index)
}

pub fn unstage_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let gix_repo = open_repo(repo)?;
    let Some(submodule) = find_submodule(&gix_repo, name)? else {
        return Err(git2::Error::from_str("Submodule not found"));
    };
    let path = submodule_path(&submodule)?;
    let mut index = gix_repo.index_or_load_from_head_or_empty().map_err(to_git2_error)?.into_owned();

    let restore_oid = gix_repo
        .head_commit()
        .ok()
        .and_then(|head| head.tree().ok())
        .and_then(|mut tree| tree.peel_to_entry_by_path(gix::path::from_bstr(path.as_bstr())).ok().flatten())
        .and_then(|entry| entry.mode().is_commit().then_some(entry.object_id()));

    if let Some(oid) = restore_oid {
        stage_commit_pointer(&mut index, path.as_ref(), oid);
    } else {
        index.remove_entries(|_, existing_path, entry| existing_path == path.as_bstr() && entry.stage() == gix::index::entry::Stage::Unconflicted);
    }

    write_index(&mut index)
}

pub fn update_submodule(repo_path: &str, name: &str, auth_session: AuthSession) -> thread::JoinHandle<NetworkResult> {
    let repo_path = repo_path.to_string();
    let name = name.to_string();

    thread::spawn(move || {
        let attempt = AuthAttempt::new(auth_session, network::UPDATE_SUBMODULE());
        let result = (|| -> Result<(), git2::Error> {
            let repo = Repository::open(&repo_path)?;
            let gix_repo = open_repo(&repo)?;
            let Some(submodule) = find_submodule(&gix_repo, &name)? else {
                return Err(git2::Error::from_str("Submodule not found"));
            };
            let target_oid = submodule.index_id().map_err(to_git2_error)?.ok_or_else(|| git2::Error::from_str("Submodule is not initialized"))?;
            let url = submodule_url_from_modules(&gix_repo, &submodule)?;

            {
                let (repo, newly_initialized) = open_or_init_submodule_repo(&submodule)?;
                let mut repo = repo;
                repo.committer_or_set_generic_fallback().map_err(to_git2_error)?;

                {
                    let remote = configure_submodule_remote(&repo, url)?;
                    let mut connection = remote.connect(gix::remote::Direction::Fetch).map_err(to_git2_error)?;
                    let auth = attempt.clone();
                    connection.set_credentials(move |action| auth.gix_credentials(action));

                    let mut progress = gix::progress::Discard;
                    let pending_pack = connection.prepare_fetch(&mut progress, Default::default()).map_err(to_git2_error)?;
                    let should_interrupt = AtomicBool::new(false);
                    pending_pack.receive(&mut progress, &should_interrupt).map_err(to_git2_error)?;
                }

                checkout_submodule_commit(&mut repo, target_oid, newly_initialized)?;
            }
            Ok(())
        })();

        network_result(network::UPDATE_SUBMODULE(), &attempt, result)
    })
}

#[cfg(test)]
#[path = "../../tests/git/actions/submodules.rs"]
mod tests;
