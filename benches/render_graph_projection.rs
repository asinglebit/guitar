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
        renderers::{GraphProjectionRender, MessageProjectionRender, render_graph_projection as render_graph_projection_lines, render_message_projection},
    },
    git::queries::helpers::UncommittedChanges,
    helpers::{palette::Theme, symbols::SymbolTheme},
};
use ratatui::style::Color;

fn main() {
    divan::main();
}

struct GraphRenderCase<'a> {
    theme: &'a Theme,
    symbols: &'a SymbolTheme,
    rows: &'a [GraphRow],
    history: &'a GraphHistory,
    head_alias: u32,
    start: usize,
    end: usize,
    render_uncommitted_row: bool,
}

impl GraphRenderCase<'_> {
    fn render(&self) -> (usize, usize) {
        let lines = black_box(render_graph_projection_lines(GraphProjectionRender {
            theme: self.theme,
            symbols: self.symbols,
            rows: self.rows,
            history: self.history,
            head_alias: self.head_alias,
            start: self.start,
            end: self.end,
            render_uncommitted_row: self.render_uncommitted_row,
        }));
        let bytes = lines.iter().map(|line| line.width()).sum();
        (lines.len(), bytes)
    }
}

fn bench_render(bencher: Bencher, case: GraphRenderCase<'_>) {
    let rendered = case.render();

    bencher.counter(ItemsCount::new(case.end.saturating_sub(case.start))).counter(BytesCount::new(rendered.1)).bench(|| black_box(case.render()));
}

fn message_case(theme: &Theme, symbols: &SymbolTheme, rows: &[GraphRow], selected: usize, show_refs: bool) -> (usize, usize) {
    let uncommitted = UncommittedChanges::default();
    let lines = black_box(render_message_projection(MessageProjectionRender {
        theme,
        symbols,
        rows,
        show_reflog_labels: true,
        show_ref_labels: show_refs,
        selected,
        uncommitted: &uncommitted,
        render_uncommitted_row: true,
    }));
    let bytes = lines.iter().map(|line| line.width()).sum();
    (lines.len(), bytes)
}

fn bench_message(bencher: Bencher, theme: &Theme, symbols: &SymbolTheme, rows: &[GraphRow], selected: usize, show_refs: bool) {
    let rendered = message_case(theme, symbols, rows, selected, show_refs);

    bencher.counter(ItemsCount::new(rows.len())).counter(BytesCount::new(rendered.1)).bench(|| black_box(message_case(theme, symbols, rows, selected, show_refs)));
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
    bench_render(
        bencher,
        GraphRenderCase {
            theme: &fixture.theme,
            symbols: &fixture.symbols,
            rows: &fixture.rows,
            history: &fixture.history,
            head_alias: fixture.head_alias,
            start: 0,
            end: fixture.rows.len(),
            render_uncommitted_row: false,
        },
    );
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_dense(bencher: Bencher) {
    let fixture = graph_fixture(24);
    bench_render(
        bencher,
        GraphRenderCase {
            theme: &fixture.theme,
            symbols: &fixture.symbols,
            rows: &fixture.rows,
            history: &fixture.history,
            head_alias: fixture.head_alias,
            start: 0,
            end: fixture.rows.len(),
            render_uncommitted_row: false,
        },
    );
}

#[divan::bench(sample_count = 30, sample_size = 20)]
fn render_graph_projection_large(bencher: Bencher) {
    let fixture = graph_fixture(256);
    let case = GraphRenderCase {
        theme: &fixture.theme,
        symbols: &fixture.symbols,
        rows: &fixture.rows,
        history: &fixture.history,
        head_alias: fixture.head_alias,
        start: 0,
        end: fixture.rows.len(),
        render_uncommitted_row: false,
    };
    let rendered = case.render();

    bencher.counter(ItemsCount::new(fixture.rows.len())).counter(BytesCount::new(rendered.1)).bench(|| black_box(case.render()));
}

#[divan::bench(sample_count = 30, sample_size = 20)]
fn render_message_projection_large_with_refs(bencher: Bencher) {
    let fixture = graph_fixture(256);
    bench_message(bencher, &fixture.theme, &fixture.symbols, &fixture.rows, 0, true);
}

#[divan::bench(sample_count = 30, sample_size = 20)]
fn render_message_projection_large_without_refs(bencher: Bencher) {
    let fixture = graph_fixture(256);
    bench_message(bencher, &fixture.theme, &fixture.symbols, &fixture.rows, 0, false);
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_uncommitted_row(bencher: Bencher) {
    let fixture = graph_fixture(12);
    let rows = vec![GraphRow {
        index: 0,
        alias: NONE,
        oid: Oid::zero(),
        short_oid: String::new(),
        summary: "uncommitted".to_string(),
        committer_date: String::new(),
        committer_name: String::new(),
        has_any_branch: false,
        branches: Vec::new(),
        tags: Vec::new(),
        is_stash: false,
        stash_lane: None,
        worktrees: Vec::new(),
        has_current_worktree: false,
        reflog: None,
    }];
    bench_render(
        bencher,
        GraphRenderCase {
            theme: &fixture.theme,
            symbols: &fixture.symbols,
            rows: &rows,
            history: &fixture.history,
            head_alias: fixture.head_alias,
            start: 0,
            end: rows.len(),
            render_uncommitted_row: true,
        },
    );
}

#[divan::bench(sample_count = 80, sample_size = 100)]
fn render_graph_projection_ascii_theme(bencher: Bencher) {
    let fixture = graph_fixture(18);
    let theme = accent_theme(Theme::classic());
    let symbols = SymbolTheme::ascii();
    bench_render(
        bencher,
        GraphRenderCase {
            theme: &theme,
            symbols: &symbols,
            rows: &fixture.rows,
            history: &fixture.history,
            head_alias: fixture.head_alias,
            start: 4,
            end: fixture.rows.len(),
            render_uncommitted_row: false,
        },
    );
}
