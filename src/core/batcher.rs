use git2::{BranchType, ObjectType, Oid, Repository, Revwalk};
use im::HashSet;
use std::cell::RefCell;
use std::collections::HashSet as StdHashSet;
use std::{rc::Rc, sync::Mutex};

// Own the revwalk cursor so commit history can be loaded in pages.
pub struct Batcher {
    revwalk: Mutex<Revwalk<'static>>,
}

impl Batcher {
    // Build the initial revwalk from visible branch tips, tags, and a detached HEAD.
    pub fn new(repo: Rc<RefCell<Repository>>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<Self, git2::Error> {
        let revwalk = Self::build(&repo.borrow(), hidden_branch_names, extra_roots)?;
        Ok(Self { revwalk: Mutex::new(revwalk) })
    }

    // Recreate the cursor after branch filters, fetches, or repository state changes.
    pub fn reset(&self, repo: Rc<RefCell<Repository>>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<(), git2::Error> {
        let revwalk = Self::build(&repo.borrow(), hidden_branch_names, extra_roots)?;
        let mut guard = self.revwalk.lock().unwrap();
        *guard = revwalk;
        Ok(())
    }

    // Pull the next page, dropping commits libgit2 cannot resolve.
    pub fn next(&self, count: usize) -> Vec<Oid> {
        let mut revwalk = self.revwalk.lock().unwrap();
        revwalk.by_ref().take(count).filter_map(Result::ok).collect()
    }

    fn build(repo: &Repository, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<Revwalk<'static>, git2::Error> {
        // The repository outlives the revwalk in App state; this keeps libgit2's lifetime usable here.
        let repo_ref: &'static Repository = unsafe { std::mem::transmute::<&Repository, &'static Repository>(repo) };

        let mut revwalk = repo_ref.revwalk()?;
        let mut pushed = StdHashSet::new();

        for branch_type in [BranchType::Local, BranchType::Remote] {
            for branch_result in repo.branches(Some(branch_type))? {
                let (branch, _) = branch_result?;

                let Some(oid) = branch.get().target() else { continue };

                let name = branch.name()?.unwrap_or("").to_string();

                // Hidden branch names are a deny-list; new branches are visible by default.
                if !hidden_branch_names.contains(&name) {
                    revwalk.push(oid)?;
                    pushed.insert(oid);
                }
            }
        }

        // A detached HEAD may not be reachable from any branch, so walk from it directly.
        if repo.head_detached().unwrap_or(false)
            && let Some(oid) = repo.head().ok().and_then(|head| head.target())
            && pushed.insert(oid)
        {
            revwalk.push(oid)?;
        }

        // Tags keep history visible in clones that only have tags, such as detached release checkouts.
        for reference in repo.references_glob("refs/tags/*")?.flatten() {
            let Ok(commit) = reference.peel(ObjectType::Commit) else { continue };
            if pushed.insert(commit.id()) {
                revwalk.push(commit.id())?;
            }
        }

        for oid in extra_roots {
            if pushed.insert(*oid) {
                revwalk.push(*oid)?;
            }
        }

        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        Ok(revwalk)
    }
}
