use crate::helpers::text::{decode, sanitize};
use git2::{Diff, DiffFormat::Patch, ObjectType, Repository};
use std::collections::HashSet;

// Snapshot of uncommitted state split the same way the status panes are drawn.
#[derive(Debug, Default, Clone)]
pub struct UncommittedChanges {
    pub unstaged: FileChanges,  // Working tree changes not yet staged.
    pub staged: FileChanges,    // Index changes ready to commit.
    pub conflicts: Vec<String>, // Paths currently carrying index conflicts.
    pub modified_count: usize,  // Unique modified paths across staged and unstaged.
    pub added_count: usize,     // Unique added paths across staged and unstaged.
    pub deleted_count: usize,   // Unique deleted paths across staged and unstaged.
    pub conflict_count: usize,  // Unique conflicted paths.
    pub is_clean: bool,         // True when both staged and unstaged lists are empty.
    pub is_staged: bool,        // True when the index has at least one change.
    pub is_unstaged: bool,      // True when the workdir has at least one change.
    pub has_conflicts: bool,    // True when the index has unresolved conflicts.
}

// File buckets are kept separate so status actions can address a stable row group.
#[derive(Debug, Default, Clone)]
pub struct FileChanges {
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
}

// One file row in a commit diff.
#[derive(Debug)]
pub struct FileChange {
    pub filename: String,
    pub status: FileStatus,
}

// Change kinds normalized from libgit2 deltas for display.
#[derive(Debug, Clone, Copy)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Other,
}

// One rendered patch line with its libgit2 origin marker.
#[derive(Debug)]
pub struct LineChange {
    pub origin: char,    // Added, removed, or context marker.
    pub content: String, // Sanitized line text.
}

// A patch hunk plus its parsed header.
#[derive(Debug)]
pub struct Hunk {
    pub header: HunkHeader,
    pub lines: Vec<LineChange>, // All patch lines belonging to this hunk.
}

// Parsed form of a unified diff hunk header.
#[derive(Debug)]
pub struct HunkHeader {
    pub old_start: u32,     // First old-side line number.
    pub old_lines: u32,     // Number of old-side lines.
    pub new_start: u32,     // First new-side line number.
    pub new_lines: u32,     // Number of new-side lines.
    pub raw_header: String, // Full header, including optional function context.
}

// Count unique filenames across staged and unstaged buckets.
pub fn deduplicate(a: &[String], b: &[String]) -> usize {
    a.iter().chain(b).collect::<HashSet<_>>().len()
}

// Recursively flatten a tree into added file rows, used for root commits and tree deltas.
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

// Convert libgit2 patch callbacks into the viewer's hunk model.
pub fn diff_to_hunks(diff: Diff) -> Result<Vec<Hunk>, git2::Error> {
    let mut hunks = Vec::new();

    // Patch format gives both hunk headers and individual lines in one pass.
    diff.print(Patch, |_, hunk_opt, line| {
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

        // Lines arrive after their hunk header, so append to the latest hunk.
        if let Some(last) = hunks.last_mut() {
            last.lines.push(LineChange { origin: line.origin(), content: sanitize(decode(line.content())).to_string() });
        }

        true
    })?;

    Ok(hunks)
}
