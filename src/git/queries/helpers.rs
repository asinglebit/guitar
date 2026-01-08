use crate::helpers::text::{decode, sanitize};
use chrono::TimeZone;
use chrono::{NaiveDate, Utc};
use git2::{Diff, DiffFormat::Patch, ObjectType, Repository};
use im::HashMap;
use std::collections::HashSet;
// Structure representing all uncommitted changes in the repository
#[derive(Debug, Default, Clone)]
pub struct UncommittedChanges {
    pub unstaged: FileChanges, // Changes in the working directory not yet staged
    pub staged: FileChanges,   // Changes that have been staged
    pub modified_count: usize, // Number of modified files (deduplicated)
    pub added_count: usize,    // Number of added files (deduplicated)
    pub deleted_count: usize,  // Number of deleted files (deduplicated)
    pub is_clean: bool,        // True if there are no changes
    pub is_staged: bool,       // True if there are staged changes
    pub is_unstaged: bool,     // True if there are unstaged changes
}

// Structure representing a set of file changes (added, modified, deleted)
#[derive(Debug, Default, Clone)]
pub struct FileChanges {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
}

// Represents a single file change (filename + status)
#[derive(Debug)]
pub struct FileChange {
    pub filename: String,
    pub status: FileStatus,
}

// Enumeration describing the type of file change
#[derive(Debug, Clone, Copy)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Other,
}

// Represents a line change within a diff hunk
#[derive(Debug)]
pub struct LineChange {
    pub origin: char,    // '+', '-', or ' ' indicating added, removed, or context
    pub content: String, // Line text content
}

// Represents a diff hunk (header and its line changes)
#[derive(Debug)]
pub struct Hunk {
    pub header: String,         // e.g. @@ -X,Y +X,Y @@
    pub lines: Vec<LineChange>, // All line changes in this hunk
}

// Deduplicate and count unique filenames from two lists
pub fn deduplicate(a: &[String], b: &[String]) -> usize {
    a.iter().chain(b).collect::<HashSet<_>>().len()
}

// Recursively traverse a Git tree and collect all file entries
pub fn walk_tree(repo: &Repository, tree: &git2::Tree, base: &str, changes: &mut Vec<FileChange>) {
    for entry in tree.iter() {
        if let Some(name) = entry.name() {
            let path = if base.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", base, name)
            };

            match entry.kind() {
                Some(ObjectType::Blob) => {
                    changes.push(FileChange {
                        filename: path,
                        status: FileStatus::Added,
                    });
                }
                Some(ObjectType::Tree) => {
                    if let Ok(subtree) = entry.to_object(repo).and_then(|o| o.peel_to_tree()) {
                        walk_tree(repo, &subtree, &path, changes);
                    }
                }
                _ => {}
            }
        }
    }
}

// Convert a Git diff into structured hunks and line changes
pub fn diff_to_hunks(diff: Diff) -> Result<Vec<Hunk>, git2::Error> {
    let mut hunks = Vec::new();

    // Print diff in patch format and collect hunks
    diff.print(Patch, |_, hunk_opt, line| {
        // Start a new hunk if encountered
        if let Some(hunk) = hunk_opt {
            hunks.push(Hunk {
                header: sanitize(decode(hunk.header())).to_string(),
                lines: Vec::new(),
            });
        }

        // Add line to the most recent hunk
        if let Some(last) = hunks.last_mut() {
            last.lines.push(LineChange {
                origin: line.origin(),
                content: sanitize(decode(line.content())).to_string(),
            });
        }

        true
    })?;

    Ok(hunks)
}

pub fn commits_per_day(repo: &Repository) -> HashMap<NaiveDate, usize> {
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();

    let mut map = HashMap::new();

    for oid in revwalk.flatten() {
        let commit = repo.find_commit(oid).unwrap();
        let time = commit.time();
        let secs = time.seconds();

        let date = Utc.timestamp_opt(secs, 0).single().unwrap().date_naive();

        *map.entry(date).or_insert(0) += 1;
    }

    map
}
