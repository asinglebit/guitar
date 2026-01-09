use crate::helpers::text::{decode, sanitize};
use git2::{Diff, DiffFormat::Patch, ObjectType, Repository};
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
    pub header: HunkHeader,
    pub lines: Vec<LineChange>, // All line changes in this hunk
}

// Represents a diff hunk header, whose raw byte format is:
// @@ -k,l +n,m @@ optional context string
#[derive(Debug)]
pub struct HunkHeader {
    pub old_start: u32, // -k
    pub old_lines: u32, // l
    pub new_start: u32, // +n
    pub new_lines: u32, // m
    // We may eventually want to use the optional context part of the header,
    // in which case we'd need to parse it out from raw_header.
    pub raw_header: String, // The entire header as a string
}

// Deduplicate and count unique filenames from two lists
pub fn deduplicate(a: &[String], b: &[String]) -> usize {
    a.iter().chain(b).collect::<HashSet<_>>().len()
}

// Recursively traverse a Git tree and collect all file entries
pub fn walk_tree(repo: &Repository, tree: &git2::Tree, base: &str, changes: &mut Vec<FileChange>) {
    for entry in tree.iter() {
        if let Some(name) = entry.name() {
            let path = if base.is_empty() { name.to_string() } else { format!("{}/{}", base, name) };

            match entry.kind() {
                Some(ObjectType::Blob) => {
                    changes.push(FileChange { filename: path, status: FileStatus::Added });
                },
                Some(ObjectType::Tree) => {
                    if let Ok(subtree) = entry.to_object(repo).and_then(|o| o.peel_to_tree()) {
                        walk_tree(repo, &subtree, &path, changes);
                    }
                },
                _ => {},
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
                header: HunkHeader {
                    old_start: hunk.old_start(),
                    old_lines: hunk.old_lines(),
                    new_start: hunk.new_start(),
                    new_lines: hunk.new_lines(),
                    raw_header: sanitize(decode(hunk.header())).to_string(),
                },
                lines: Vec::new(),
            });
        }

        // Add line to the most recent hunk
        if let Some(last) = hunks.last_mut() {
            last.lines.push(LineChange { origin: line.origin(), content: sanitize(decode(line.content())).to_string() });
        }

        true
    })?;

    Ok(hunks)
}
