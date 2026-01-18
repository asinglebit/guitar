use crate::core::oids::Oids;
use git2::{BranchType, Oid, Repository, Revwalk};
use im::HashSet;
use std::cell::RefCell;
use std::{rc::Rc, sync::Mutex};

// Encapsulate a revwalk over the git repository, allowing incremental fetching of commits
pub struct Batcher {
    revwalk: Mutex<Revwalk<'static>>,
}

impl Batcher {
    // Creates a new Batcher by building a revwalk from the repo
    pub fn new(repo: Rc<RefCell<Repository>>, visible_branch_names: &HashSet<String>) -> Result<Self, git2::Error> {
        let revwalk = Self::build(&repo.borrow(), visible_branch_names)?;
        Ok(Self { revwalk: Mutex::new(revwalk) })
    }

    // Reset the revwalk
    pub fn reset(&self, repo: Rc<RefCell<Repository>>, visible_branch_names: &HashSet<String>) -> Result<(), git2::Error> {
        let revwalk = Self::build(&repo.borrow(), visible_branch_names)?;
        let mut guard = self.revwalk.lock().unwrap();
        *guard = revwalk;
        Ok(())
    }

    // Get up to "count" commits from the global revwalk
    pub fn next(&self, count: usize) -> Vec<Oid> {
        let mut revwalk = self.revwalk.lock().unwrap();
        revwalk.by_ref().take(count).filter_map(Result::ok).collect()
    }

    fn build(repo: &Repository, visible_branch_names: &HashSet<String>) -> Result<Revwalk<'static>, git2::Error> {
        let repo_ref: &'static Repository = unsafe { std::mem::transmute::<&Repository, &'static Repository>(repo) };

        let mut revwalk = repo_ref.revwalk()?;

        for branch_type in [BranchType::Local, BranchType::Remote] {
            for branch_result in repo.branches(Some(branch_type))? {
                let (branch, _) = branch_result?;

                let Some(oid) = branch.get().target() else { continue };

                let name = branch.name()?.unwrap_or("").to_string();

                // No filter → show everything
                if visible_branch_names.is_empty() || visible_branch_names.contains(&name) {
                    revwalk.push(oid)?;
                }
            }
        }

        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        Ok(revwalk)
    }
}
