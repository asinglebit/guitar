use crate::{
    git::auth::{AuthAttempt, AuthSession, NetworkResult, network_result},
    git::repository::open,
    helpers::localisation::network,
};
use git2::{FetchOptions, RemoteCallbacks, Repository, SubmoduleUpdateOptions};
use gix::bstr::ByteSlice;
use std::thread;

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

pub fn sync_submodule(repo: &Repository, name: &str) -> Result<(), git2::Error> {
    let mut submodule = repo.find_submodule(name)?;
    submodule.sync()
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
            let config = repo.config()?;
            let mut submodule = repo.find_submodule(&name)?;

            let mut callbacks = RemoteCallbacks::new();
            let auth = attempt.clone();
            callbacks.credentials(move |url, username_from_url, allowed| auth.credentials(&config, url, username_from_url, allowed));

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            let mut options = SubmoduleUpdateOptions::new();
            options.fetch(fetch_options);

            submodule.update(true, Some(&mut options))
        })();

        network_result(network::UPDATE_SUBMODULE(), &attempt, result)
    })
}

#[cfg(test)]
#[path = "../../tests/git/actions/submodules.rs"]
mod tests;
