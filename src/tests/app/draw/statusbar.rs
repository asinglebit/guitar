use super::*;
use crate::{app::state::layout::Layout, core::submodules::SubmoduleStackEntry, helpers::layout::LayoutConfig, helpers::symbols::submodule::DEFAULT as SYM_SUBMODULE};
use git2::{Repository, Signature};
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

#[test]
fn statusbar_renders_submodule_stack_before_branch() {
    let (path, repo) = temp_repo("submodule-stack");
    let mut app = App {
        layout: Layout { statusbar_left: Rect::new(0, 0, 180, 1), statusbar_right: Rect::new(180, 0, 20, 1), ..Default::default() },
        submodule_stack: vec![
            SubmoduleStackEntry::new(path.clone(), PathBuf::from("deps/child"), "deps/child".into()),
            SubmoduleStackEntry::new(path.join("deps/child"), PathBuf::from("vendor/grandchild"), "vendor/grandchild".into()),
        ],
        ..Default::default()
    };
    let backend = TestBackend::new(200, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| app.draw_statusbar(frame, &repo)).unwrap();

    let rendered = rendered_symbols(&terminal);
    let breadcrumb = format!("{SYM_SUBMODULE} {}", path.file_name().unwrap().to_string_lossy());
    assert!(rendered.contains(&breadcrumb));
    assert!(rendered.contains("deps/child"));
    assert!(rendered.contains("vendor/grandchild"));
    assert!(rendered.find(&breadcrumb).unwrap() < rendered.find('●').unwrap());
}

fn right_bar_symbols(terminal: &Terminal<TestBackend>) -> String {
    // The right status bar starts at column 180 in these tests; the left bar has its own circles.
    rendered_symbols(terminal).chars().skip(180).collect()
}

fn statusbar_app() -> App {
    // App::default() loads the developer's saved layout.json, so pin the config the indicators read.
    App { layout: Layout { statusbar_left: Rect::new(0, 0, 180, 1), statusbar_right: Rect::new(180, 0, 20, 1), ..Default::default() }, layout_config: LayoutConfig::default(), ..Default::default() }
}

#[test]
fn statusbar_shows_a_watcher_circle_only_while_the_watcher_is_on() {
    let (path, repo) = temp_repo("file-watcher");
    let mut app = statusbar_app();
    let mut terminal = Terminal::new(TestBackend::new(200, 1)).unwrap();

    terminal.draw(|frame| app.draw_statusbar(frame, &repo)).unwrap();
    assert_eq!(right_bar_symbols(&terminal).matches('●').count(), 0);

    app.layout_config.is_file_watcher = true;
    terminal.draw(|frame| app.draw_statusbar(frame, &repo)).unwrap();
    assert_eq!(right_bar_symbols(&terminal).matches('●').count(), 1);

    fs::remove_dir_all(&path).ok();
}

#[test]
fn statusbar_shows_zen_and_watcher_circles_side_by_side() {
    let (path, repo) = temp_repo("zen-and-watcher");
    let mut app = statusbar_app();
    app.layout_config.is_zen = true;
    app.layout_config.is_file_watcher = true;
    let mut terminal = Terminal::new(TestBackend::new(200, 1)).unwrap();

    terminal.draw(|frame| app.draw_statusbar(frame, &repo)).unwrap();

    assert_eq!(right_bar_symbols(&terminal).matches('●').count(), 2);
    fs::remove_dir_all(&path).ok();
}

#[test]
fn statusbar_shows_every_indicator_together_in_order() {
    let (path, repo) = temp_repo("all-indicators");
    let mut app = statusbar_app();
    app.mode = InputMode::Action;
    app.layout_config.is_zen = true;
    app.layout_config.is_file_watcher = true;
    let mut terminal = Terminal::new(TestBackend::new(200, 1)).unwrap();

    terminal.draw(|frame| app.draw_statusbar(frame, &repo)).unwrap();

    // Action, zen, file watcher.
    assert_eq!(right_bar_symbols(&terminal).matches('●').count(), 3);
    fs::remove_dir_all(&path).ok();
}
