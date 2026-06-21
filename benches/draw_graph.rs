mod fixtures;

use divan::{Bencher, black_box};
use fixtures::{commit_file, graph_fixture, temp_repo};
use guitar::{
    App,
    app::{
        app::{Focus, GraphWindowCache, Viewport},
        state::layout::Layout,
    },
};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

fn main() {
    divan::main();
}

fn app_with_graph_window(cycles: usize, visible_height: u16) -> (fixtures::TempFixture, git2::Repository, App) {
    let (temp, repo) = temp_repo("draw-graph-bench");
    commit_file(&repo, "file.txt", "content\n", "initial");

    let fixture = graph_fixture(cycles);
    let mut app = App {
        viewport: Viewport::Graph,
        focus: Focus::Viewport,
        theme: fixture.theme,
        symbols: fixture.symbols,
        layout: Layout { graph: Rect::new(0, 0, 140, visible_height), graph_scrollbar: Rect::new(139, 0, 1, visible_height), ..Default::default() },
        ..Default::default()
    };
    app.layout_config.is_shas = true;
    app.layout_config.is_graph_dates = true;
    app.layout_config.is_graph_committers = true;
    app.layout_config.is_graph_refs = true;
    app.layout_config.is_zen = false;
    app.graph.total = fixture.rows.len();
    app.graph.graph_window = Some(GraphWindowCache { version: 1, start: 0, end: fixture.rows.len(), head_alias: fixture.head_alias, rows: fixture.rows, history: fixture.history });
    (temp, repo, app)
}

fn draw_cached_graph(app: &mut App, repo: &git2::Repository, terminal: &mut Terminal<TestBackend>) -> usize {
    terminal.draw(|frame| app.draw_graph(frame, repo)).unwrap();
    black_box(terminal.backend().buffer().content().len())
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn draw_graph_cached_visible_page(bencher: Bencher) {
    let (_temp, repo, mut app) = app_with_graph_window(64, 48);
    let backend = TestBackend::new(140, 48);
    let mut terminal = Terminal::new(backend).unwrap();

    bencher.counter(divan::counter::ItemsCount::new(48usize)).bench_local(|| black_box(draw_cached_graph(&mut app, &repo, &mut terminal)));
}

#[divan::bench(sample_count = 50, sample_size = 50)]
fn draw_graph_cached_scrolled_prefetch_window(bencher: Bencher) {
    let (_temp, repo, mut app) = app_with_graph_window(96, 48);
    app.graph_selected = 80;
    app.graph_scroll.set(80);
    if let Some(window) = app.graph.graph_window.as_mut() {
        window.start = 32;
        window.end = window.start + window.rows.len();
        for row in &mut window.rows {
            row.index += 32;
        }
        app.graph.total = window.end;
    }
    let backend = TestBackend::new(140, 48);
    let mut terminal = Terminal::new(backend).unwrap();

    bencher.counter(divan::counter::ItemsCount::new(48usize)).bench_local(|| black_box(draw_cached_graph(&mut app, &repo, &mut terminal)));
}
