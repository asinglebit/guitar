use super::*;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_dir(name: &str) -> PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-watcher-{name}-{id}"));
    fs::create_dir_all(path.join(".git/refs/heads")).unwrap();
    fs::create_dir_all(path.join(".git/objects/ab")).unwrap();
    fs::create_dir_all(path.join("src")).unwrap();
    path
}

#[test]
fn excludes_git_object_storage_but_keeps_refs() {
    let root = temp_dir("excludes");
    assert!(is_excluded(&root.join(".git/objects/ab/cdef"), &root));
    assert!(is_excluded(&root.join(".git/logs/HEAD"), &root));
    assert!(!is_excluded(&root.join(".git/refs/heads/main"), &root));
    assert!(!is_excluded(&root.join(".git/packed-refs"), &root));
    assert!(!is_excluded(&root.join(".git/HEAD"), &root));
    assert!(!is_excluded(&root.join(".git"), &root));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn excludes_build_directories_but_keeps_sources() {
    let root = temp_dir("build-dirs");
    assert!(is_excluded(&root.join("target/debug/guitar"), &root));
    assert!(is_excluded(&root.join("node_modules/left-pad/index.js"), &root));
    assert!(!is_excluded(&root.join("src/main.rs"), &root));
    assert!(!is_excluded(&root, &root));
    // Paths outside the repository are never treated as excluded.
    assert!(!is_excluded(Path::new("/somewhere/else"), &root));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn spawns_for_a_real_directory_and_reports_its_path() {
    let root = temp_dir("spawn");
    let path = root.to_str().unwrap().to_string();
    let watcher = spawn_repo_watcher(&path).expect("watcher should spawn for a real directory");
    assert_eq!(watcher.path, path);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn returns_none_for_a_missing_directory() {
    assert!(spawn_repo_watcher("/guitar/definitely/not/a/real/path").is_none());
}
