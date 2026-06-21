use git2::{BranchType, Oid, Repository, Revwalk};
use im::HashSet;
use std::cell::RefCell;
use std::collections::HashSet as StdHashSet;
use std::rc::Rc;

// Own the revwalk cursor so commit history can be loaded in pages.
pub struct Batcher {
    revwalk: Revwalk<'static>,
}

impl Batcher {
    // Build the initial revwalk from all visible local and remote branch tips.
    pub fn new(repo: Rc<RefCell<Repository>>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<Self, git2::Error> {
        let revwalk = Self::build(&repo.borrow(), hidden_branch_names, extra_roots)?;
        Ok(Self { revwalk })
    }

    // Recreate the cursor after branch filters, fetches, or repository state changes.
    pub fn reset(&mut self, repo: Rc<RefCell<Repository>>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<(), git2::Error> {
        self.revwalk = Self::build(&repo.borrow(), hidden_branch_names, extra_roots)?;
        Ok(())
    }

    // Pull the next page, dropping commits libgit2 cannot resolve.
    pub fn next(&mut self, count: usize) -> Vec<Oid> {
        self.revwalk.by_ref().take(count).filter_map(Result::ok).collect()
    }

    // Pull the next page into an existing output buffer to avoid a temporary page allocation.
    pub fn next_into(&mut self, count: usize, out: &mut Vec<Oid>) -> usize {
        let before = out.len();
        out.extend(self.revwalk.by_ref().take(count).filter_map(Result::ok));
        out.len() - before
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

        for oid in extra_roots {
            if pushed.insert(*oid) {
                revwalk.push(*oid)?;
            }
        }

        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        Ok(revwalk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_repo(name: &str) -> (PathBuf, Rc<RefCell<Repository>>) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-batcher-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (path, Rc::new(RefCell::new(repo)))
    }

    fn commit(repo: &Repository, file: &str, message: &str) -> Oid {
        let workdir = repo.workdir().unwrap().to_path_buf();
        fs::write(workdir.join(file), message).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).unwrap()
    }

    #[test]
    fn next_into_appends_pages_without_replacing_existing_output() {
        let (_path, repo) = temp_repo("next-into");
        let first = commit(&repo.borrow(), "first.txt", "first");
        let second = commit(&repo.borrow(), "second.txt", "second");
        let third = commit(&repo.borrow(), "third.txt", "third");
        let sentinel = Oid::zero();
        let mut batcher = Batcher::new(repo.clone(), &HashSet::new(), &[third]).unwrap();
        let mut out = vec![sentinel];

        assert_eq!(batcher.next_into(2, &mut out), 2);
        assert_eq!(batcher.next_into(2, &mut out), 1);
        assert_eq!(batcher.next_into(2, &mut out), 0);

        assert_eq!(out, vec![sentinel, third, second, first]);
    }
}
