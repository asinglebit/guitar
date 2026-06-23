use crate::{
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    git::repository::open,
    helpers::localisation::network,
};
use git2::Repository;
use gix::bstr::ByteSlice;
use gix::sec::trust::DefaultForLevel;
use std::{fs::OpenOptions, io::Write, sync::atomic::AtomicBool, thread};

fn open_gix_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    gix::open(path).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn find_submodule<'repo>(repo: &'repo gix::Repository, name: &str) -> Result<Option<gix::Submodule<'repo>>, git2::Error> {
    let Some(mut submodules) = repo.submodules().map_err(|error| git2::Error::from_str(&error.to_string()))? else {
        return Ok(None);
    };
    let wanted = name.as_bytes().as_bstr();
    Ok(submodules.find(|submodule| submodule.name() == wanted || submodule.path().is_ok_and(|path| path.as_ref() == wanted)))
}

fn submodule_path(submodule: &gix::Submodule<'_>) -> Result<gix::bstr::BString, git2::Error> {
    submodule.path().map(|path| path.into_owned()).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn write_index(index: &mut gix::index::File) -> Result<(), git2::Error> {
    index.sort_entries();
    index.write(gix::index::write::Options::default()).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn stage_commit_pointer(index: &mut gix::index::File, path: &gix::bstr::BStr, oid: gix::ObjectId) {
    if let Some(entry) = index.entry_mut_by_path_and_stage(path, gix::index::entry::Stage::Unconflicted) {
        entry.id = oid;
        entry.mode = gix::index::entry::Mode::COMMIT;
        return;
    }

    index.dangerously_push_entry(Default::default(), oid, gix::index::entry::Flags::empty(), gix::index::entry::Mode::COMMIT, path);
}

fn load_local_config(path: std::path::PathBuf) -> Result<gix::config::File<'static>, git2::Error> {
    gix::config::File::from_path_no_includes(path, gix::config::Source::Local).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn write_local_config(config: &gix::config::File<'static>) -> Result<(), git2::Error> {
    let path = config.meta().path.as_deref().ok_or_else(|| git2::Error::from_str("Configuration path is missing"))?;
    let mut file = OpenOptions::new().create(false).write(true).truncate(true).open(path).map_err(|error| git2::Error::from_str(&error.to_string()))?;

    file.write_all(config.detect_newline_style()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    config.write_to_filter(&mut file, |section| section.meta().source == gix::config::Source::Local).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn submodule_url_from_modules(repo: &gix::Repository, submodule: &gix::Submodule<'_>) -> Result<gix::Url, git2::Error> {
    let workdir = repo.workdir().ok_or_else(|| git2::Error::from_str("Repository has no working directory"))?;
    let modules = load_local_config(workdir.join(".gitmodules"))?;
    let submodule_name = submodule.name().to_str().map_err(|_| git2::Error::from_str("Submodule name is not valid UTF-8"))?;
    let key = format!("submodule.{submodule_name}.url");
    let url = modules.string(key).ok_or_else(|| git2::Error::from_str("Submodule URL is missing"))?;
    gix::Url::from_bytes(url.as_ref()).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn sync_superproject_submodule_url(repo: &gix::Repository, submodule: &gix::Submodule<'_>, url: &gix::bstr::BStr) -> Result<(), git2::Error> {
    let config_path = repo.config_snapshot().plumbing().meta().path.clone().ok_or_else(|| git2::Error::from_str("Repository configuration path is missing"))?;
    let mut config = load_local_config(config_path)?;
    config.set_raw_value_by("submodule", Some(submodule.name()), "url", url).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    write_local_config(&config)
}

fn sync_checked_out_submodule_url(submodule: &gix::Repository, url: &gix::Url) -> Result<(), git2::Error> {
    let remote = submodule.find_fetch_remote(None).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let config_path = submodule.config_snapshot().plumbing().meta().path.clone().ok_or_else(|| git2::Error::from_str("Submodule configuration path is missing"))?;
    let mut config = load_local_config(config_path)?;
    remote.with_url(url.clone()).map_err(|error| git2::Error::from_str(&error.to_string()))?.save_to(&mut config).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    write_local_config(&config)
}

fn open_or_init_submodule_repo(submodule: &gix::Submodule<'_>) -> Result<(gix::Repository, bool), git2::Error> {
    if let Some(repo) = submodule.open().map_err(|error| git2::Error::from_str(&error.to_string()))? {
        return Ok((repo, false));
    }

    let workdir = submodule.work_dir().map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let create_opts = gix::create::Options { destination_must_be_empty: true, ..Default::default() };
    let repo = gix::ThreadSafeRepository::init_opts(workdir, gix::create::Kind::WithWorktree, create_opts, gix::open::Options::default_for_level(gix::sec::Trust::Full))
        .map_err(|error| git2::Error::from_str(&error.to_string()))?
        .to_thread_local();
    Ok((repo, true))
}

fn configure_submodule_remote<'repo>(repo: &'repo gix::Repository, url: gix::Url) -> Result<gix::Remote<'repo>, git2::Error> {
    let mut remote = repo.remote_at_without_url_rewrite(url).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    remote = remote.with_fetch_tags(gix::remote::fetch::Tags::All);
    remote = remote.with_refspecs(Some("+refs/heads/*:refs/remotes/origin/*"), gix::remote::Direction::Fetch).map_err(|error| git2::Error::from_str(&error.to_string()))?;

    let config_path = repo.config_snapshot().plumbing().meta().path.clone().ok_or_else(|| git2::Error::from_str("Repository configuration path is missing"))?;
    let mut config = load_local_config(config_path)?;
    remote.save_as_to("origin", &mut config).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    write_local_config(&config)?;
    Ok(remote)
}

fn checkout_submodule_commit(repo: &mut gix::Repository, target_oid: gix::ObjectId, newly_initialized: bool) -> Result<(), git2::Error> {
    let tree_id = {
        let commit = repo.find_commit(target_oid).map_err(|error| git2::Error::from_str(&error.to_string()))?;
        commit.tree().map_err(|error| git2::Error::from_str(&error.to_string()))?.id
    };
    let mut index = repo.index_from_tree(&tree_id).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let mut options = repo.checkout_options(gix_worktree::stack::state::attributes::Source::IdMapping).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    options.destination_is_initially_empty = newly_initialized;

    let workdir = repo.workdir().ok_or_else(|| git2::Error::from_str("Submodule has no working directory"))?;
    let should_interrupt = AtomicBool::new(false);
    let files = gix::progress::Discard;
    let bytes = gix::progress::Discard;
    gix::worktree::state::checkout(&mut index, workdir, repo.objects.clone().into_arc().map_err(|error| git2::Error::from_str(&error.to_string()))?, &files, &bytes, &should_interrupt, options)
        .map_err(|error| git2::Error::from_str(&error.to_string()))?;
    index.write(Default::default()).map_err(|error| git2::Error::from_str(&error.to_string()))?;

    let head: gix::refs::FullName = "HEAD".try_into().expect("valid");
    repo.edit_reference(gix::refs::transaction::RefEdit {
        change: gix::refs::transaction::Change::Update { log: Default::default(), expected: gix::refs::transaction::PreviousValue::Any, new: gix::refs::Target::Object(target_oid) },
        name: head,
        deref: false,
    })
    .map_err(|error| git2::Error::from_str(&error.to_string()))?;
    Ok(())
}

pub fn sync_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let gix_repo = open_gix_repo(repo)?;
    let Some(submodule) = find_submodule(&gix_repo, name)? else {
        return Err(git2::Error::from_str("Submodule not found"));
    };
    let url = submodule_url_from_modules(&gix_repo, &submodule)?;
    let url_bstring = url.to_bstring();
    sync_superproject_submodule_url(&gix_repo, &submodule, url_bstring.as_ref())?;
    if let Some(submodule_repo) = submodule.open().map_err(|error| git2::Error::from_str(&error.to_string()))? {
        sync_checked_out_submodule_url(&submodule_repo, &url)?;
    }
    Ok(())
}

pub fn stage_submodule_head(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let gix_repo = open_gix_repo(repo)?;
    let Some(submodule) = find_submodule(&gix_repo, name)? else {
        return Err(git2::Error::from_str("Submodule not found"));
    };
    let path = submodule_path(&submodule)?;
    let sub_repo = submodule.open().map_err(|error| git2::Error::from_str(&error.to_string()))?.ok_or_else(|| git2::Error::from_str("Submodule is not initialized"))?;
    let head_oid = sub_repo.head_id().map_err(|error| git2::Error::from_str(&error.to_string()))?.detach();
    let mut index = gix_repo.index_or_load_from_head_or_empty().map_err(|error| git2::Error::from_str(&error.to_string()))?.into_owned();

    stage_commit_pointer(&mut index, path.as_ref(), head_oid);
    write_index(&mut index)
}

pub fn unstage_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let gix_repo = open_gix_repo(repo)?;
    let Some(submodule) = find_submodule(&gix_repo, name)? else {
        return Err(git2::Error::from_str("Submodule not found"));
    };
    let path = submodule_path(&submodule)?;
    let mut index = gix_repo.index_or_load_from_head_or_empty().map_err(|error| git2::Error::from_str(&error.to_string()))?.into_owned();

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
            let repo = open(&repo_path)?;
            let gix_repo = open_gix_repo(&repo)?;
            let Some(submodule) = find_submodule(&gix_repo, &name)? else {
                return Err(git2::Error::from_str("Submodule not found"));
            };
            let target_oid = submodule.index_id().map_err(|error| git2::Error::from_str(&error.to_string()))?.ok_or_else(|| git2::Error::from_str("Submodule is not initialized"))?;
            let url = submodule_url_from_modules(&gix_repo, &submodule)?;

            {
                let (repo, newly_initialized) = open_or_init_submodule_repo(&submodule)?;
                let mut repo = repo;
                repo.committer_or_set_generic_fallback().map_err(|error| git2::Error::from_str(&error.to_string()))?;

                {
                    let remote = configure_submodule_remote(&repo, url)?;
                    let mut connection = remote.connect(gix::remote::Direction::Fetch).map_err(|error| git2::Error::from_str(&error.to_string()))?;
                    let auth = attempt.clone();
                    connection.set_credentials(move |action| auth.gix_credentials(action));

                    let mut progress = gix::progress::Discard;
                    let pending_pack = connection.prepare_fetch(&mut progress, Default::default()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
                    let should_interrupt = AtomicBool::new(false);
                    pending_pack.receive(&mut progress, &should_interrupt).map_err(|error| git2::Error::from_str(&error.to_string()))?;
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
