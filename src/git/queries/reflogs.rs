use crate::git::gix::gix_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadReflogEntry {
    pub selector: String,
    pub old_oid: gix::ObjectId,
    pub new_oid: gix::ObjectId,
    pub message: String,
    pub time: gix::date::Time,
}

pub fn get_head_reflog_entries(repo: &gix::Repository) -> Result<Vec<HeadReflogEntry>, git2::Error> {
    let head = repo.head().map_err(gix_error)?;
    let mut log_iter = head.log_iter();
    let Some(reflog) = log_iter.rev().map_err(gix_error)? else {
        return Err(git2::Error::from_str("HEAD reflog not found"));
    };

    let mut entries = Vec::new();

    for (idx, entry) in reflog.enumerate() {
        let entry = entry.map_err(gix_error)?;
        if entry.new_oid.is_null() || repo.find_commit(entry.new_oid).is_err() {
            continue;
        }

        let message = if entry.message.is_empty() { "reflog".to_string() } else { String::from_utf8_lossy(entry.message.as_ref()).to_string() };
        entries.push(HeadReflogEntry { selector: format!("HEAD@{{{idx}}}"), old_oid: entry.previous_oid, new_oid: entry.new_oid, message, time: entry.signature.time });
    }

    Ok(entries)
}

#[cfg(test)]
#[path = "../../tests/git/queries/reflogs.rs"]
mod tests;
