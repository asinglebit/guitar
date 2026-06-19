use crate::helpers::{palette::Theme, symbols::SymbolTheme};
use chrono::{Datelike, NaiveDate};
use chrono::{TimeZone, Utc};
use git2::{Oid, Repository};
use im::HashMap;
use ratatui::{style::Style, text::Span};

pub const WEEKS: usize = 53;
pub const DAYS: usize = 7;

pub fn commits_per_day(repo: &git2::Repository, oids: &Vec<Oid>) -> HashMap<usize, usize> {
    // Use UTC dates so commits near midnight are bucketed consistently.
    let today: NaiveDate = Utc::now().date_naive();
    let mut counts: HashMap<usize, usize> = HashMap::new();

    for oid in oids {
        let commit = match repo.find_commit(*oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Git commit times are stored as epoch seconds.
        let commit_date = Utc.timestamp_opt(commit.time().seconds(), 0).single().unwrap().date_naive();

        let days_ago = today.signed_duration_since(commit_date).num_days();

        // Ignore commits outside the rendered 53-week grid.
        if !(0..53 * 7).contains(&days_ago) {
            continue;
        }

        *counts.entry(days_ago as usize).or_insert(0) += 1;
    }

    counts
}

pub fn empty_heatmap() -> [[usize; WEEKS]; DAYS] {
    [[0usize; WEEKS]; DAYS]
}

pub fn build_heatmap(repo: &Repository, oids: &Vec<Oid>) -> [[usize; WEEKS]; DAYS] {
    // Rows are weekdays starting Monday, columns run oldest to newest.
    let mut grid = [[0usize; WEEKS]; DAYS];

    // Counts are keyed by days ago, with 0 representing today.
    let counts = commits_per_day(repo, oids);

    let today = Utc::now().date_naive();

    // Chrono uses 0 for Monday and 6 for Sunday.
    let weekday_today = today.weekday().num_days_from_monday() as usize;

    let total_days = WEEKS * DAYS;

    // Align the newest column so today lands on its weekday row.
    let offset = 6 - weekday_today;

    for days_ago in 0..total_days {
        // Shift relative age into the displayed grid coordinate system.
        let logical = days_ago + offset;

        let week = logical / 7;

        if week >= WEEKS {
            continue;
        }

        // Reverse week order because the screen reads oldest to newest.
        let week_idx = WEEKS - 1 - week;

        // Convert age back into a Monday-based weekday row.
        let day_idx = (weekday_today + 7 - (days_ago % 7)) % 7;

        let count = *counts.get(&days_ago).unwrap_or(&0);

        grid[day_idx][week_idx] = count;
    }

    grid
}

pub fn heat_cell(count: usize, theme: &Theme, symbols: &SymbolTheme) -> Span<'static> {
    let (character, color) = match count {
        0 => (symbols.heatmap.cell(count), Some(theme.COLOR_TEXT)),
        _ => (symbols.heatmap.cell(count), Some(theme.COLOR_GRASS)),
    };
    let style = if let Some(c) = color { Style::default().fg(c) } else { Style::default() };
    Span::styled(format!("{:>2}", character), style)
}
