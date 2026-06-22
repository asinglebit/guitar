use crate::{git::gix::commit_graph_if_available, helpers::branch_visibility::branch_name_from_ref};
use git2::Oid;
use gix::traverse::commit::ParentIds;
use im::HashSet;
use std::collections::HashSet as StdHashSet;

type CommitWalk = gix::traverse::commit::Simple<gix::OdbHandle, fn(&gix::oid) -> bool>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkCommit {
    pub oid: Oid,
    pub parent_ids: ParentIds,
    pub commit_time: Option<i64>,
}

// Own a lazy gitoxide commit cursor so history pages don't precompute the entire graph.
pub struct Batcher {
    walk: Option<CommitWalk>,
}

impl Batcher {
    // Build the initial commit cursor from all visible local and remote branch tips.
    pub fn new(repo: &gix::Repository, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<Self, git2::Error> {
        let walk = Self::build(repo, hidden_branch_names, extra_roots)?;
        Ok(Self { walk: Some(walk) })
    }

    // Recreate the cursor after branch filters, fetches, or repository state changes.
    pub fn reset(&mut self, repo: &gix::Repository, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<(), git2::Error> {
        self.walk = Some(Self::build(repo, hidden_branch_names, extra_roots)?);
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

    fn build(repo: &gix::Repository, hidden_branch_names: &HashSet<String>, extra_roots: &[Oid]) -> Result<CommitWalk, git2::Error> {
        let mut pushed: StdHashSet<gix::ObjectId> = StdHashSet::new();
        let mut tips: Vec<gix::ObjectId> = Vec::new();

        let references = repo.references().map_err(|error| git2::Error::from_str(&error.to_string()))?;
        for references in [references.local_branches(), references.remote_branches()] {
            let references = references.map_err(|error| git2::Error::from_str(&error.to_string()))?.peeled().map_err(|error| git2::Error::from_str(&error.to_string()))?;

            for reference in references {
                let Ok(reference) = reference else { continue };
                let Some(name) = branch_name_from_ref(reference.name().as_bstr()) else { continue };
                let Some(oid) = reference.try_id().map(|id| id.detach()) else { continue };

                // Hidden branch names are a deny-list; new branches are visible by default.
                if !hidden_branch_names.contains(name) && pushed.insert(oid) {
                    tips.push(oid);
                }
            }
        }

        for oid in extra_roots {
            let oid = gix::ObjectId::from_bytes_or_panic(oid.as_bytes());
            if pushed.insert(oid) {
                tips.push(oid);
            }
        }

        let commit_graph = commit_graph_if_available(repo);
        let walk = gix::traverse::commit::Simple::new(tips, repo.objects.clone())
            .sorting(gix::traverse::commit::simple::Sorting::ByCommitTime(gix::traverse::commit::simple::CommitTimeOrder::NewestFirst))
            .map(|walk| walk.commit_graph(commit_graph))
            .map_err(|error| git2::Error::from_str(&error.to_string()))?;
        Ok(walk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{BranchType, Commit, Repository, Signature};
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_repo(name: &str) -> (PathBuf, Repository) {
        let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("guitar-batcher-{name}-{id}"));
        fs::create_dir_all(&path).unwrap();
        let repo = Repository::init(&path).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        (path, repo)
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
        let first = commit(&repo, "first.txt", "first");
        let second = commit(&repo, "second.txt", "second");
        let third = commit(&repo, "third.txt", "third");
        let sentinel = WalkCommit { oid: Oid::zero(), parent_ids: ParentIds::new(), commit_time: None };
        let gix_repo = gix::open(&path).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &HashSet::new(), &[third]).unwrap();
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
        let first = commit(&repo, "first.txt", "first");
        let second = commit(&repo, "second.txt", "second");
        repo.branch("duplicate", &repo.find_commit(second).unwrap(), false).unwrap();
        let gix_repo = gix::open(&path).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &HashSet::new(), &[second]).unwrap();

        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![second, first]);
    }

    #[test]
    fn hidden_branch_tip_is_not_used_as_a_walk_root() {
        let (path, repo) = temp_repo("hidden-tip");
        let main = commit(&repo, "main.txt", "main");
        let main_ref = head_refname(&repo);
        repo.branch("hidden", &repo.find_commit(main).unwrap(), false).unwrap();
        repo.set_head("refs/heads/hidden").unwrap();
        repo.checkout_head(None).unwrap();
        let hidden = commit(&repo, "hidden.txt", "hidden");
        repo.set_head(&main_ref).unwrap();
        repo.checkout_head(None).unwrap();

        let mut hidden_names = HashSet::new();
        hidden_names.insert("hidden".to_string());
        let gix_repo = gix::open(&path).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &hidden_names, &[]).unwrap();

        assert_eq!(branch_tip(&repo, "hidden"), hidden);
        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![main]);
    }

    #[test]
    fn merge_commit_preserves_first_two_parent_ids() {
        let (path, repo) = temp_repo("merge-parents");
        let first = commit(&repo, "first.txt", "first");
        let main_ref = head_refname(&repo);
        repo.branch("side", &repo.find_commit(first).unwrap(), false).unwrap();
        let main = commit(&repo, "main.txt", "main");
        repo.set_head("refs/heads/side").unwrap();
        repo.checkout_head(None).unwrap();
        let side = commit(&repo, "side.txt", "side");
        repo.set_head(&main_ref).unwrap();
        repo.checkout_head(None).unwrap();
        let merge = {
            let main_commit = repo.find_commit(main).unwrap();
            let side_commit = repo.find_commit(side).unwrap();
            commit_with_parents(&repo, "merge.txt", "merge", &[&main_commit, &side_commit])
        };
        let gix_repo = gix::open(&path).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &HashSet::new(), &[merge]).unwrap();

        let page = batcher.next(10);
        let merge_commit = page.iter().find(|commit| commit.oid == merge).expect("merge commit is returned");

        assert_eq!(merge_commit.parent_ids.iter().map(|id| Oid::from_bytes(id.as_slice()).unwrap()).collect::<Vec<_>>(), vec![main, side]);
    }

    #[test]
    fn exhausted_batcher_stays_empty_until_reset() {
        let (path, repo) = temp_repo("exhausted");
        let first = commit(&repo, "first.txt", "first");
        let second = commit(&repo, "second.txt", "second");
        let gix_repo = gix::open(&path).unwrap();
        let mut batcher = Batcher::new(&gix_repo, &HashSet::new(), &[second]).unwrap();

        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![second, first]);
        assert!(batcher.next(10).is_empty());
        assert!(batcher.next(10).is_empty());

        let third = commit(&repo, "third.txt", "third");
        let gix_repo = gix::open(&path).unwrap();
        batcher.reset(&gix_repo, &HashSet::new(), &[third]).unwrap();

        assert_eq!(batcher.next(10).iter().map(|commit| commit.oid).collect::<Vec<_>>(), vec![third, second, first]);
    }
}
