use crate::{
    core::oids::{Oids, git2_to_gix_oid},
    helpers::{palette::Theme, symbols::SymbolTheme},
};
use chrono::{Datelike, NaiveDate};
use chrono::{TimeZone, Utc};
use git2::Oid;
use gix::prelude::FindExt;
use ratatui::{style::Style, text::Span};

pub const WEEKS: usize = 53;
pub const DAYS: usize = 7;
const TOTAL_DAYS: usize = WEEKS * DAYS;

pub fn commits_per_day(repo: &gix::Repository, oids: &[Oid]) -> [usize; TOTAL_DAYS] {
    commits_per_day_in_order(repo, oids.iter().copied())
}

fn commits_per_day_in_order(repo: &gix::Repository, oids: impl IntoIterator<Item = Oid>) -> [usize; TOTAL_DAYS] {
    // Use UTC dates so commits near midnight are bucketed consistently.
    let today: NaiveDate = Utc::now().date_naive();
    let mut counts = [0usize; TOTAL_DAYS];
    let mut object_buf = Vec::new();

    for oid in oids {
        object_buf.clear();
        let commit = match repo.objects.find_commit(git2_to_gix_oid(oid).as_ref(), &mut object_buf) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Git commit times are stored as epoch seconds.
        let Some(commit_date) = commit.time().ok().and_then(|time| Utc.timestamp_opt(time.seconds, 0).single()).map(|date| date.date_naive()) else {
            continue;
        };

        let days_ago = today.signed_duration_since(commit_date).num_days();

        if days_ago < 0 {
            continue;
        }

        // Input is newest-first, so the first older commit means the rendered
        // heatmap year is complete.
        if days_ago >= TOTAL_DAYS as i64 {
            break;
        }

        counts[days_ago as usize] += 1;
    }

    counts
}

pub fn empty_heatmap() -> [[usize; WEEKS]; DAYS] {
    [[0usize; WEEKS]; DAYS]
}

pub fn build_heatmap(repo: &gix::Repository, oids: &[Oid]) -> [[usize; WEEKS]; DAYS] {
    build_heatmap_from_counts(commits_per_day(repo, oids))
}

pub fn build_heatmap_from_sorted_aliases(repo: &gix::Repository, oids: &Oids) -> [[usize; WEEKS]; DAYS] {
    build_heatmap_from_counts(commits_per_day_in_order(repo, oids.get_sorted_aliases().iter().map(|alias| *oids.get_oid_by_alias(*alias))))
}

fn build_heatmap_from_counts(counts: [usize; TOTAL_DAYS]) -> [[usize; WEEKS]; DAYS] {
    // Rows are weekdays starting Monday, columns run oldest to newest.
    let mut grid = [[0usize; WEEKS]; DAYS];

    let today = Utc::now().date_naive();

    // Chrono uses 0 for Monday and 6 for Sunday.
    let weekday_today = today.weekday().num_days_from_monday() as usize;

    // Align the newest column so today lands on its weekday row.
    let offset = 6 - weekday_today;

    for (days_ago, count) in counts.iter().enumerate() {
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

        grid[day_idx][week_idx] = *count;
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

#[cfg(test)]
#[path = "../tests/helpers/heatmap.rs"]
mod tests;
