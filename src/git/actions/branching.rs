use git2::{Branch, BranchType, Error, Oid, Repository};

pub fn create_branch(repo: &Repository, branch_name: &str, target_oid: Oid) -> Result<(), Error> {
    // Branch creation is intentionally non-checkout; the graph stays on the current HEAD.
    let target_commit = repo.find_commit(target_oid)?;

    repo.branch(branch_name, &target_commit, false)?;

    Ok(())
}

pub fn delete_branch(repo: &Repository, branch: &str) -> Result<(), git2::Error> {
    let mut local = repo.find_branch(branch, BranchType::Local)?;
    local.delete()?;
    Ok(())
}

pub fn rename_branch(repo: &Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err(Error::from_str("branch name cannot be empty"));
    }
    if old_name == new_name {
        return Err(Error::from_str("new branch name must differ from current branch name"));
    }
    if !Branch::name_is_valid(new_name)? {
        return Err(Error::from_str("branch name is invalid"));
    }
    if repo.find_branch(new_name, BranchType::Local).is_ok() {
        return Err(Error::from_str("branch name already exists"));
    }

    let mut branch = repo.find_branch(old_name, BranchType::Local)?;
    branch.rename(new_name, false)?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/git/actions/branching.rs"]
mod tests;
