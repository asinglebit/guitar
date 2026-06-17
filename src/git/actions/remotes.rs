use crate::git::queries::remotes::{GUITAR_DEFAULT_REMOTE_CONFIG, PUSH_DEFAULT_CONFIG};
use git2::{Error, Remote, Repository};

fn validate_remote_name(name: &str) -> Result<&str, Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::from_str("remote name cannot be empty"));
    }
    if !Remote::is_valid_name(name) {
        return Err(Error::from_str("remote name is invalid"));
    }
    Ok(name)
}

fn validate_remote_url(url: &str) -> Result<&str, Error> {
    let url = url.trim();
    if url.is_empty() {
        return Err(Error::from_str("remote URL cannot be empty"));
    }
    Ok(url)
}

pub fn add_remote(repo: &Repository, name: &str, url: &str) -> Result<(), Error> {
    let name = validate_remote_name(name)?;
    let url = validate_remote_url(url)?;
    repo.remote(name, url)?;
    Ok(())
}

pub fn rename_remote(repo: &Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    let old_name = validate_remote_name(old_name)?;
    let new_name = validate_remote_name(new_name)?;
    if old_name == new_name {
        return Err(Error::from_str("new remote name must differ from current remote name"));
    }
    repo.remote_rename(old_name, new_name)?;
    rename_default_remote_config(repo, old_name, new_name)?;
    Ok(())
}

pub fn delete_remote(repo: &Repository, name: &str) -> Result<(), Error> {
    let name = validate_remote_name(name)?;
    repo.remote_delete(name)?;
    clear_default_remote_config(repo, name)?;
    Ok(())
}

pub fn set_remote_url(repo: &Repository, name: &str, url: &str) -> Result<(), Error> {
    let name = validate_remote_name(name)?;
    let url = validate_remote_url(url)?;
    repo.find_remote(name)?;
    repo.remote_set_url(name, url)?;
    Ok(())
}

pub fn set_remote_push_url(repo: &Repository, name: &str, push_url: Option<&str>) -> Result<(), Error> {
    let name = validate_remote_name(name)?;
    let push_url = push_url.map(str::trim).filter(|value| !value.is_empty());
    repo.find_remote(name)?;
    repo.remote_set_pushurl(name, push_url)?;
    Ok(())
}

pub fn set_default_remote(repo: &Repository, name: &str) -> Result<(), Error> {
    let name = validate_remote_name(name)?;
    repo.find_remote(name)?;
    let mut config = repo.config()?;
    config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, name)?;
    config.set_str(PUSH_DEFAULT_CONFIG, name)?;
    Ok(())
}

fn rename_default_remote_config(repo: &Repository, old_name: &str, new_name: &str) -> Result<(), Error> {
    let mut config = repo.config()?;
    let guitar_default_matches = config_value_matches(&config, GUITAR_DEFAULT_REMOTE_CONFIG, old_name);
    let push_default_matches = config_value_matches(&config, PUSH_DEFAULT_CONFIG, old_name);

    if guitar_default_matches {
        config.set_str(GUITAR_DEFAULT_REMOTE_CONFIG, new_name)?;
        config.set_str(PUSH_DEFAULT_CONFIG, new_name)?;
    } else if push_default_matches {
        config.set_str(PUSH_DEFAULT_CONFIG, new_name)?;
    }

    Ok(())
}

fn clear_default_remote_config(repo: &Repository, name: &str) -> Result<(), Error> {
    let mut config = repo.config()?;
    if config_value_matches(&config, GUITAR_DEFAULT_REMOTE_CONFIG, name) {
        let _ = config.remove(GUITAR_DEFAULT_REMOTE_CONFIG);
    }
    if config_value_matches(&config, PUSH_DEFAULT_CONFIG, name) {
        let _ = config.remove(PUSH_DEFAULT_CONFIG);
    }
    Ok(())
}

fn config_value_matches(config: &git2::Config, key: &str, expected: &str) -> bool {
    config.get_string(key).map(|value| value.trim() == expected).unwrap_or(false)
}

#[cfg(test)]
#[path = "../../tests/git/actions/remotes.rs"]
mod tests;
