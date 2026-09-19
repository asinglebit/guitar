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

#[test]
fn reports_a_write_in_the_working_tree() {
    let root = temp_dir("events-worktree");
    let watcher = spawn_repo_watcher(root.to_str().unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));

    fs::write(root.join("src/file.txt"), "hello").unwrap();

    let got = watcher.rx.recv_timeout(std::time::Duration::from_secs(5));
    fs::remove_dir_all(&root).ok();
    assert!(got.is_ok(), "a working tree write should reach the channel");
}

#[test]
fn reports_a_ref_update_inside_git() {
    let root = temp_dir("events-refs");
    let watcher = spawn_repo_watcher(root.to_str().unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));

    fs::write(root.join(".git/refs/heads/main"), "0000000000000000000000000000000000000000\n").unwrap();

    let got = watcher.rx.recv_timeout(std::time::Duration::from_secs(5));
    fs::remove_dir_all(&root).ok();
    assert!(got.is_ok(), "a ref update should reach the channel");
}

#[test]
fn ignores_writes_into_git_object_storage() {
    let root = temp_dir("events-objects");
    let watcher = spawn_repo_watcher(root.to_str().unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));

    fs::write(root.join(".git/objects/ab/cdef"), "object").unwrap();

    let got = watcher.rx.recv_timeout(std::time::Duration::from_millis(800));
    fs::remove_dir_all(&root).ok();
    assert!(got.is_err(), "object writes must not trigger a reload");
}

#[test]
fn reads_are_not_treated_as_changes() {
    use notify::event::{AccessKind, DataChange, MetadataKind, ModifyKind, RenameMode};

    // The regression this guards: the app reads the repository on every reload, and counting those
    // Access events as changes kept the debounce window permanently open, so the owed reload never
    // ran and only live-read UI such as the status bar appeared to update.
    assert!(!is_change(&EventKind::Access(AccessKind::Open(notify::event::AccessMode::Any))));
    assert!(!is_change(&EventKind::Access(AccessKind::Read)));
    assert!(!is_change(&EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime))));
    assert!(!is_change(&EventKind::Other));

    assert!(is_change(&EventKind::Create(notify::event::CreateKind::File)));
    assert!(is_change(&EventKind::Remove(notify::event::RemoveKind::File)));
    assert!(is_change(&EventKind::Modify(ModifyKind::Data(DataChange::Content))));
    assert!(is_change(&EventKind::Modify(ModifyKind::Name(RenameMode::Both))));
    assert!(is_change(&EventKind::Any));
}

#[test]
fn reading_files_produces_no_channel_traffic() {
    let root = temp_dir("events-reads");
    fs::write(root.join("src/file.txt"), "hello").unwrap();
    let watcher = spawn_repo_watcher(root.to_str().unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    while watcher.rx.try_recv().is_ok() {}

    // Exactly what the graph walker does on every reload.
    for _ in 0..20 {
        let _ = fs::read_to_string(root.join("src/file.txt")).unwrap();
        let _ = fs::read_dir(root.join(".git/refs/heads")).unwrap().count();
    }

    let got = watcher.rx.recv_timeout(std::time::Duration::from_millis(800));
    fs::remove_dir_all(&root).ok();
    assert!(got.is_err(), "reads must not queue a reload, or the debounce never settles");
}
