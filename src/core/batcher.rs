use crate::git::gix::{commit_graph_if_available, enable_history_object_cache};
use git2::{BranchType, Oid, Repository};
use gix::traverse::commit::ParentIds;
use im::HashSet;
use std::cell::RefCell;
use std::collections::HashSet as StdHashSet;
use std::path::PathBuf;
use std::rc::Rc;

type CommitWalk = gix::traverse::commit::Simple<gix::OdbHandle, fn(&gix::oid) -> bool>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkCommit {
    pub oid: Oid,
    pub parent_ids: ParentIds,
    pub commit_time: Option<i64>,
}

// Own a lazy gitoxide commit cursor so history pages don't precompute the entire graph.
pub struct Batcher {
    repo_path: PathBuf,
    walk: Option<CommitWalk>,
}

impl Batcher {
    // Build the initial commit cursor from all visible local and remote branch tips.
    pub fn new(repo: Rc<RefCell<Repository>>, repo_path: impl Into<PathBuf>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<Self, git2::Error> {
        let repo_path = repo_path.into();
        let walk = Self::build(&repo.borrow(), &repo_path, hidden_branch_names, extra_roots)?;
        Ok(Self { repo_path, walk: Some(walk) })
    }

    // Recreate the cursor after branch filters, fetches, or repository state changes.
    pub fn reset(&mut self, repo: Rc<RefCell<Repository>>, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<(), git2::Error> {
        self.walk = Some(Self::build(&repo.borrow(), &self.repo_path, hidden_branch_names, extra_roots)?);
        Ok(())
    }

    // Pull the next page, dropping commits gitoxide cannot resolve.
    pub fn next(&mut self, count: usize) -> Vec<WalkCommit> {
        let mut page = Vec::with_capacity(count);
        self.next_into(count, &mut page);
        page
    }

    // Pull the next page into an existing output buffer to avoid a temporary page allocation.
    pub fn next_into(&mut self, count: usize, out: &mut Vec<WalkCommit>) -> usize {
        let before = out.len();
        let Some(walk) = self.walk.as_mut() else {
            return 0;
        };

        while out.len() - before < count {
            let Some(result) = walk.next() else {
                self.walk = None;
                break;
            };

            let Ok(info) = result else { continue };
            out.push(WalkCommit { oid: Oid::from_bytes(info.id.as_slice()).unwrap(), parent_ids: info.parent_ids, commit_time: info.commit_time.map(Into::into) });
        }
        out.len() - before
    }

    pub fn remaining(&self) -> usize {
        0
    }

    fn build(repo: &Repository, repo_path: &PathBuf, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<CommitWalk, git2::Error> {
        let mut gix_repo = gix::open(repo_path).map_err(|error| git2::Error::from_str(&error.to_string()))?;
        enable_history_object_cache(&mut gix_repo);
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

        let commit_graph = commit_graph_if_available(&gix_repo);
        let walk = gix::traverse::commit::Simple::new(tips, gix_repo.objects.clone())
            .sorting(gix::traverse::commit::simple::Sorting::ByCommitTime(gix::traverse::commit::simple::CommitTimeOrder::NewestFirst))
            .map(|walk| walk.commit_graph(commit_graph))
            .map_err(|error| git2::Error::from_str(&error.to_string()))?;
        Ok(walk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{BranchType, Commit, Signature};
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
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&Commit<'_>> = parent.iter().collect();
        commit_with_parents(repo, file, message, &parents)
    }

    fn commit_with_parents(repo: &Repository, file: &str, message: &str, parents: &[&Commit<'_>]) -> Oid {
        let workdir = repo.workdir().unwrap().to_path_buf();
        fs::write(workdir.join(file), message).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, parents).unwrap()
    }

    fn branch_tip(repo: &Repository, name: &str) -> Oid {
        repo.find_branch(name, BranchType::Local).unwrap().get().target().unwrap()
    }

    fn head_refname(repo: &Repository) -> String {
        repo.head().unwrap().name().unwrap().to_string()
    }

    #[test]
    fn next_into_appends_pages_without_replacing_existing_output() {
        let (path, repo) = temp_repo("next-into");
        let first = commit(&repo.borrow(), "first.txt", "first");
        let second = commit(&repo.borrow(), "second.txt", "second");
        let third = commit(&repo.borrow(), "third.txt", "third");
        let sentinel = WalkCommit { oid: Oid::zero(), parent_ids: ParentIds::new(), commit_time: None };
        let mut batcher = Batcher::new(repo.clone(), path.clone(), &HashSet::new(), &[third]).unwrap();
        let mut out = vec![sentinel.clone()];

        assert_eq!(batcher.next_into(2, &mut out), 2);
        assert_eq!(batcher.next_into(2, &mut out), 1);
        assert_eq!(batcher.next_into(2, &mut out), 0);

        let oids = out.iter().map(|commit| commit.oid).collect::<Vec<_>>();
        assert_eq!(oids, vec![sentinel.oid, third, second, first]);
        assert_eq!(out[1].parent_ids.iter().map(|id| Oid::from_bytes(id.as_slice()).unwrap()).collect::<Vec<_>>(), vec![second]);
        assert_eq!(out[2].parent_ids.iter().map(|id| Oid::from_bytes(id.as_slice()).unwrap()).collect::<Vec<_>>(), vec![first]);
        assert!(out[3].parent_ids.is_empty());
    }

    #[test]
    fn duplicate_branch_tips_are_returned_once() {
        let (path, repo) = temp_repo("duplicate-tips");
        let first = commit(&repo.borrow(), "first.txt", "first");
        let second = commit(&repo.borrow(), "second.txt", "second");
        repo.borrow().branch("duplicate", &repo.borrow().find_commit(second).unwrap(), false).unwrap();
        let mut batcher = Batcher::new(repo.clone(), path.clone(), &HashSet::new(), &[second]).unwrap();

        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![second, first]);
    }

    #[test]
    fn hidden_branch_tip_is_not_used_as_a_walk_root() {
        let (path, repo) = temp_repo("hidden-tip");
        let main = commit(&repo.borrow(), "main.txt", "main");
        let main_ref = head_refname(&repo.borrow());
        repo.borrow().branch("hidden", &repo.borrow().find_commit(main).unwrap(), false).unwrap();
        repo.borrow().set_head("refs/heads/hidden").unwrap();
        repo.borrow().checkout_head(None).unwrap();
        let hidden = commit(&repo.borrow(), "hidden.txt", "hidden");
        repo.borrow().set_head(&main_ref).unwrap();
        repo.borrow().checkout_head(None).unwrap();

        let mut hidden_names = HashSet::new();
        hidden_names.insert("hidden".to_string());
        let mut batcher = Batcher::new(repo.clone(), path.clone(), &hidden_names, &[]).unwrap();

        assert_eq!(branch_tip(&repo.borrow(), "hidden"), hidden);
        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![main]);
    }

    #[test]
    fn merge_commit_preserves_first_two_parent_ids() {
        let (path, repo) = temp_repo("merge-parents");
        let first = commit(&repo.borrow(), "first.txt", "first");
        let main_ref = head_refname(&repo.borrow());
        repo.borrow().branch("side", &repo.borrow().find_commit(first).unwrap(), false).unwrap();
        let main = commit(&repo.borrow(), "main.txt", "main");
        repo.borrow().set_head("refs/heads/side").unwrap();
        repo.borrow().checkout_head(None).unwrap();
        let side = commit(&repo.borrow(), "side.txt", "side");
        repo.borrow().set_head(&main_ref).unwrap();
        repo.borrow().checkout_head(None).unwrap();
        let merge = {
            let repo_ref = repo.borrow();
            let main_commit = repo_ref.find_commit(main).unwrap();
            let side_commit = repo_ref.find_commit(side).unwrap();
            commit_with_parents(&repo_ref, "merge.txt", "merge", &[&main_commit, &side_commit])
        };
        let mut batcher = Batcher::new(repo.clone(), path.clone(), &HashSet::new(), &[merge]).unwrap();

        let page = batcher.next(10);
        let merge_commit = page.iter().find(|commit| commit.oid == merge).expect("merge commit is returned");

        assert_eq!(merge_commit.parent_ids.iter().map(|id| Oid::from_bytes(id.as_slice()).unwrap()).collect::<Vec<_>>(), vec![main, side]);
    }

    #[test]
    fn exhausted_batcher_stays_empty_until_reset() {
        let (path, repo) = temp_repo("exhausted");
        let first = commit(&repo.borrow(), "first.txt", "first");
        let second = commit(&repo.borrow(), "second.txt", "second");
        let mut batcher = Batcher::new(repo.clone(), path.clone(), &HashSet::new(), &[second]).unwrap();

        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![second, first]);
        assert!(batcher.next(10).is_empty());
        assert!(batcher.next(10).is_empty());

        let third = commit(&repo.borrow(), "third.txt", "third");
        batcher.reset(repo, &HashSet::new(), &[third]).unwrap();

        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![third, second, first]);
    }
}
