use crate::core::oids::Oids;
use git2::{BranchType, Oid, Repository, Revwalk};
use std::cell::RefCell;
use std::{collections::HashMap, rc::Rc, sync::Mutex};

// Encapsulate a revwalk over the git repository, allowing incremental fetching of commits
pub struct Batcher {
    revwalk: Mutex<Revwalk<'static>>,
}

impl Batcher {
    // Creates a new Batcher by building a revwalk from the repo
    pub fn new(
        repo: Rc<RefCell<Repository>>,
        visible: HashMap<u32, Vec<String>>,
        oids: &mut Oids,
    ) -> Result<Self, git2::Error> {
        let revwalk = Self::build(&repo.borrow(), visible, oids)?;
        Ok(Self {
            revwalk: Mutex::new(revwalk),
        })
    }

    // Reset the revwalk
    pub fn reset(
        &self,
        repo: Rc<RefCell<Repository>>,
        visible: HashMap<u32, Vec<String>>,
        oids: &mut Oids,
    ) -> Result<(), git2::Error> {
        let revwalk = Self::build(&repo.borrow(), visible, oids)?;
        let mut guard = self.revwalk.lock().unwrap();
        *guard = revwalk;
        Ok(())
    }

    // Get up to "count" commits from the global revwalk
    pub fn next(&self, count: usize) -> Vec<Oid> {
        let mut revwalk = self.revwalk.lock().unwrap();
        revwalk
            .by_ref()
            .take(count)
            .filter_map(Result::ok)
            .collect()
    }

    // Internal helper to build a revwalk for all branch tips
    fn build(
        repo: &Repository,
        visible: HashMap<u32, Vec<String>>,
        oids: &mut Oids,
    ) -> Result<Revwalk<'static>, git2::Error> {
        // Safe: we keep repo alive in Rc, so transmute to 'static is safe
        let repo_ref: &'static Repository =
            unsafe { std::mem::transmute::<&Repository, &'static Repository>(repo) };
        let mut revwalk = repo_ref.revwalk()?;

        // TODO: Steal faster implementation from get_tip_oids function!
        // Push all branches_local and branches_remote branch tips
        for branch_type in [BranchType::Local, BranchType::Remote] {
            for branch_result in repo.branches(Some(branch_type))? {
                let (branch, _) = branch_result?;
                if let Some(oid) = branch.get().target() {
                    // Get the oidi
                    let alias = oids.get_alias_by_oid(oid);

                    if visible.is_empty() || visible.contains_key(&alias) {
                        revwalk.push(oid)?;
                    }
                }
            }
        }

        // Topological and chronological sorting
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        Ok(revwalk)
    }
}
