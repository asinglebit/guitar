use crate::core::oids::{gix_time_to_git2_time, gix_to_git2_oid};
use git2::{Oid, Repository, Time};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadReflogEntry {
    pub selector: String,
    pub old_oid: Oid,
    pub new_oid: Oid,
    pub message: String,
    pub time: Time,
}

pub fn get_head_reflog_entries(repo: &Repository) -> Result<Vec<HeadReflogEntry>, git2::Error> {
    let gix_repo = open_gix_repo(repo)?;
    get_head_reflog_entries_gix(&gix_repo)
}

fn open_gix_repo(repo: &Repository) -> Result<gix::Repository, git2::Error> {
    let path = repo.workdir().unwrap_or(repo.path());
    gix::open(path).map_err(|error| git2::Error::from_str(&error.to_string()))
}

fn get_head_reflog_entries_gix(repo: &gix::Repository) -> Result<Vec<HeadReflogEntry>, git2::Error> {
    let head = repo.head().map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let mut log_iter = head.log_iter();
    let Some(reflog) = log_iter.rev().map_err(|error| git2::Error::from_str(&error.to_string()))? else {
        return Err(git2::Error::from_str("HEAD reflog not found"));
    };

    let mut entries = Vec::new();

    for (idx, entry) in reflog.enumerate() {
        let entry = entry.map_err(|error| git2::Error::from_str(&error.to_string()))?;
        let new_oid = gix_to_git2_oid(entry.new_oid);
        if new_oid.is_zero() || repo.find_commit(entry.new_oid).is_err() {
            continue;
        }

        let message = if entry.message.is_empty() { "reflog".to_string() } else { String::from_utf8_lossy(entry.message.as_ref()).to_string() };
        let time = gix_time_to_git2_time(entry.signature.time);

        entries.push(HeadReflogEntry { selector: format!("HEAD@{{{idx}}}"), old_oid: gix_to_git2_oid(entry.previous_oid), new_oid, message, time });
    }

    Ok(entries)
}

#[cfg(test)]
fn get_head_reflog_entries_git2(repo: &Repository) -> Result<Vec<HeadReflogEntry>, git2::Error> {
    let reflog = repo.reflog("HEAD")?;
    let mut entries = Vec::new();

    for (idx, entry) in reflog.iter().enumerate() {
        let new_oid = entry.id_new();
        if new_oid.is_zero() || repo.find_commit(new_oid).is_err() {
            continue;
        }

        let message = entry.message().map(str::to_string).or_else(|| entry.message_bytes().map(|bytes| String::from_utf8_lossy(bytes).to_string())).unwrap_or_else(|| "reflog".to_string());

        entries.push(HeadReflogEntry { selector: format!("HEAD@{{{idx}}}"), old_oid: entry.id_old(), new_oid, message, time: entry.committer().when() });
    }

    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/reflogs.rs"]
mod tests;
