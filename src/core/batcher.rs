use git2::{BranchType, Oid, Repository};
use im::HashSet;
use std::cell::RefCell;
use std::collections::{HashSet as StdHashSet, VecDeque};
use std::path::PathBuf;
use std::{rc::Rc, sync::Mutex};

use gix::traverse::commit::topo::{Builder as TopoBuilder, Sorting as TopoSorting};

// Own the history queue so commit pages can be loaded without libgit2 revwalk state.
pub struct Batcher {
    repo_path: PathBuf,
    commits: Mutex<VecDeque<Oid>>,
}

impl Batcher {
    // Build the initial history queue from all visible local and remote branch tips.
    pub fn new(repo: Rc<RefCell<Repository>>, repo_path: impl Into<PathBuf>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<Self, git2::Error> {
        let repo_path = repo_path.into();
        let commits = Self::build(&repo.borrow(), &repo_path, hidden_branch_names, extra_roots)?;
        Ok(Self { repo_path, commits: Mutex::new(commits) })
    }

    // Recreate the cursor after branch filters, fetches, or repository state changes.
    pub fn reset(&self, repo: Rc<RefCell<Repository>>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<(), git2::Error> {
        let commits = Self::build(&repo.borrow(), &self.repo_path, hidden_branch_names, extra_roots)?;
        let mut guard = self.commits.lock().unwrap();
        *guard = commits;
        Ok(())
    }

    // Pull the next page, dropping commits gitoxide cannot resolve.
    pub fn next(&self, count: usize) -> Vec<Oid> {
        let mut commits = self.commits.lock().unwrap();
        let mut page = Vec::with_capacity(count);
        for _ in 0..count {
            let Some(oid) = commits.pop_front() else { break };
            page.push(oid);
        }
        page
    }

    // Pull the next page into an existing output buffer to avoid a temporary page allocation.
    pub fn next_into(&self, count: usize, out: &mut Vec<Oid>) -> usize {
        let before = out.len();
        out.extend(self.next(count));
        out.len() - before
    }

    fn build(repo: &Repository, repo_path: &PathBuf, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<VecDeque<Oid>, git2::Error> {
        let gix_repo = gix::open(repo_path.clone()).map_err(|error| git2::Error::from_str(&error.to_string()))?;
        let mut pushed = StdHashSet::new();
        let mut tips = Vec::new();

        for branch_type in [BranchType::Local, BranchType::Remote] {
            for branch_result in repo.branches(Some(branch_type))? {
                let (branch, _) = branch_result?;

                let Some(oid) = branch.get().target() else { continue };

                let name = branch.name()?.unwrap_or("").to_string();

                // Hidden branch names are a deny-list; new branches are visible by default.
                if !hidden_branch_names.contains(&name) {
                    pushed.insert(oid);
                    tips.push(gix::ObjectId::from_bytes_or_panic(oid.as_bytes()));
                }
            }
        }

        for oid in extra_roots {
            if pushed.insert(*oid) {
                tips.push(gix::ObjectId::from_bytes_or_panic(oid.as_bytes()));
            }
        }

        if tips.is_empty() {
            return Ok(VecDeque::new());
        }

        let topo = TopoBuilder::new(&gix_repo.objects).with_tips(tips).sorting(TopoSorting::TopoOrder).build().map_err(|error| git2::Error::from_str(&error.to_string()))?;
        topo.map(|result| result.map_err(|error| git2::Error::from_str(&error.to_string())).map(|info| Oid::from_bytes(info.id.as_slice()).unwrap())).collect()
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
        let (path, repo) = temp_repo("next-into");
        let first = commit(&repo.borrow(), "first.txt", "first");
        let second = commit(&repo.borrow(), "second.txt", "second");
        let third = commit(&repo.borrow(), "third.txt", "third");
        let sentinel = Oid::zero();
        let mut batcher = Batcher::new(repo.clone(), path.clone(), &HashSet::new(), &[third]).unwrap();
        let mut out = vec![sentinel];

        assert_eq!(batcher.next_into(2, &mut out), 2);
        assert_eq!(batcher.next_into(2, &mut out), 1);
        assert_eq!(batcher.next_into(2, &mut out), 0);

        assert_eq!(out, vec![sentinel, third, second, first]);
    }
}
