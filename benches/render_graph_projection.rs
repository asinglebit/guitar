mod fixtures;

use divan::{
    Bencher, black_box,
    counter::{BytesCount, ItemsCount},
};
use fixtures::graph_fixture;
use git2::Oid;
use guitar::{
    core::{
        chunk::NONE,
        graph_service::{GraphHistory, GraphRow},
        renderers::render_graph_projection as render_graph_projection_lines,
    },
    helpers::{palette::Theme, symbols::SymbolTheme},
};
use ratatui::style::Color;

fn main() {
    divan::main();
}

fn render_case(theme: &Theme, symbols: &SymbolTheme, rows: &[GraphRow], history: &GraphHistory, head_alias: u32, start: usize, end: usize, render_uncommitted_row: bool) -> (usize, usize) {
    let lines = black_box(render_graph_projection_lines(theme, symbols, rows, history, head_alias, start, end, render_uncommitted_row));
    let bytes = lines.iter().map(|line| line.width()).sum();
    (lines.len(), bytes)
}

fn bench_render(bencher: Bencher, theme: &Theme, symbols: &SymbolTheme, rows: &[GraphRow], history: &GraphHistory, head_alias: u32, start: usize, end: usize, render_uncommitted_row: bool) {
    let rendered = render_case(theme, symbols, rows, history, head_alias, start, end, render_uncommitted_row);

    bencher
        .counter(ItemsCount::new(end.saturating_sub(start)))
        .counter(BytesCount::new(rendered.1))
        .bench(|| black_box(render_case(theme, symbols, rows, history, head_alias, start, end, render_uncommitted_row)));
}

fn accent_theme(mut theme: Theme) -> Theme {
    theme.COLOR_TEXT = Color::Rgb(220, 220, 220);
    theme.COLOR_BORDER = Color::Rgb(84, 84, 84);
    theme.COLOR_HIGHLIGHTED = Color::Rgb(255, 200, 120);
    theme
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_small(bencher: Bencher) {
    let fixture = graph_fixture(6);
    bench_render(bencher, &fixture.theme, &fixture.symbols, &fixture.rows, &fixture.history, fixture.head_alias, 0, fixture.rows.len(), false);
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_dense(bencher: Bencher) {
    let fixture = graph_fixture(24);
    bench_render(bencher, &fixture.theme, &fixture.symbols, &fixture.rows, &fixture.history, fixture.head_alias, 0, fixture.rows.len(), false);
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_uncommitted_row(bencher: Bencher) {
    let fixture = graph_fixture(12);
    let rows = vec![GraphRow {
        index: 0,
        alias: NONE,
        oid: Oid::zero(),
        summary: "uncommitted".to_string(),
        committer_date: String::new(),
        committer_name: String::new(),
        has_any_branch: false,
        branches: Vec::new(),
        tags: Vec::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: Vec::new(),
        reflog: None,
    }];
    bench_render(bencher, &fixture.theme, &fixture.symbols, &rows, &fixture.history, fixture.head_alias, 0, rows.len(), true);
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_ascii_theme(bencher: Bencher) {
    let fixture = graph_fixture(18);
    let theme = accent_theme(Theme::classic());
    let symbols = SymbolTheme::ascii();
    bench_render(bencher, &theme, &symbols, &fixture.rows, &fixture.history, fixture.head_alias, 4, fixture.rows.len(), false);
}
