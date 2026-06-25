use super::*;
use crate::{
    app::{app::Focus, state::layout::Layout},
    core::{
        branches::Branches,
        oids::git2_to_gix_oid,
        submodules::SubmoduleStackEntry,
        worktrees::{WorktreeEntry, WorktreeKind},
    },
    helpers::symbols::submodule::DEFAULT as SYM_SUBMODULE,
};
use git2::{Oid, Repository, Signature};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_repo(name: &str) -> (PathBuf, Repository) {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let path = std::env::temp_dir().join(format!("guitar-statusbar-{name}-{id}"));
    fs::create_dir_all(&path).unwrap();
    let repo = Repository::init(&path).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
    }
    fs::write(path.join("file.txt"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
    drop(tree);
    (path, repo)
}

fn rendered_symbols(terminal: &Terminal<TestBackend>) -> String {
    terminal.backend().buffer().content().iter().map(|cell| cell.symbol()).collect::<String>()
}

fn current_worktree(path: PathBuf, branch: Option<&str>, head: Oid) -> WorktreeEntry {
    WorktreeEntry {
        name: path.file_name().and_then(|name| name.to_str()).unwrap_or("repo").to_string(),
        path,
        branch: branch.map(str::to_string),
        head: Some(git2_to_gix_oid(head)),
        alias: None,
        kind: WorktreeKind::Main,
        is_current: true,
        is_valid: true,
        is_prunable: false,
        locked_reason: None,
        is_dirty: false,
    }
}

#[test]
fn statusbar_renders_submodule_stack_before_branch() {
    let (path, repo) = temp_repo("submodule-stack");
    let head = repo.head().unwrap().target().unwrap();
    let mut app = App {
        layout: Layout { statusbar_left: Rect::new(0, 0, 180, 1), statusbar_right: Rect::new(180, 0, 20, 1), ..Default::default() },
        submodule_stack: vec![
            SubmoduleStackEntry::new(path.clone(), PathBuf::from("deps/child"), "deps/child".into()),
            SubmoduleStackEntry::new(path.join("deps/child"), PathBuf::from("vendor/grandchild"), "vendor/grandchild".into()),
        ],
        worktrees: crate::core::worktrees::Worktrees::from_entries(vec![current_worktree(path.clone(), Some("master"), head)]),
        ..Default::default()
    };
    let backend = TestBackend::new(200, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_statusbar(frame)).unwrap();

    let rendered = rendered_symbols(&terminal);
    let breadcrumb = format!("{SYM_SUBMODULE} {}", path.file_name().unwrap().to_string_lossy());
    assert!(rendered.contains(&breadcrumb));
    assert!(rendered.contains("deps/child"));
    assert!(rendered.contains("vendor/grandchild"));
    assert!(rendered.find(&breadcrumb).unwrap() < rendered.find('●').unwrap());
}

#[test]
fn statusbar_branch_count_uses_cached_branch_rows_without_scanning_refs() {
    let (path, repo) = temp_repo("cached-branches");
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("repo-only-a", &head, false).unwrap();
    repo.branch("repo-only-b", &head, false).unwrap();

    let mut branches = Branches { sorted: vec![(1, "main".to_string()), (2, "hidden".to_string())], ..Default::default() };
    branches.hidden_branch_names.insert("hidden".to_string());

    let mut app = App {
        focus: Focus::Branches,
        layout: Layout { statusbar_left: Rect::new(0, 0, 40, 1), statusbar_right: Rect::new(40, 0, 20, 1), ..Default::default() },
        branches,
        worktrees: crate::core::worktrees::Worktrees::from_entries(vec![current_worktree(path, Some("main"), head.id())]),
        ..Default::default()
    };
    let backend = TestBackend::new(60, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_statusbar(frame)).unwrap();

    let rendered = rendered_symbols(&terminal);
    assert!(rendered.contains("1/1"), "{rendered}");
    assert!(!rendered.contains("1/3"), "{rendered}");
}
