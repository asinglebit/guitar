use crate::{
    core::oids::Oids,
    helpers::{palette::Theme, symbols::SymbolTheme},
};
use chrono::{Datelike, NaiveDate};
use chrono::{TimeZone, Utc};
use gix::prelude::FindExt;
use ratatui::{style::Style, text::Span};

pub const WEEKS: usize = 53;
pub const DAYS: usize = 7;
const TOTAL_DAYS: usize = WEEKS * DAYS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeatmapCounts {
    today: NaiveDate,
    counts: [usize; TOTAL_DAYS],
}

impl Default for HeatmapCounts {
    fn default() -> Self {
        Self { today: Utc::now().date_naive(), counts: [0usize; TOTAL_DAYS] }
    }
}

impl HeatmapCounts {
    pub fn add_commit_seconds(&mut self, seconds: i64) {
        let Some(commit_date) = Utc.timestamp_opt(seconds, 0).single().map(|date| date.date_naive()) else {
            return;
        };

        let days_ago = self.today.signed_duration_since(commit_date).num_days();
        if !(0..TOTAL_DAYS as i64).contains(&days_ago) {
            return;
        }

        self.counts[days_ago as usize] += 1;
    }

    pub fn build(&self) -> [[usize; WEEKS]; DAYS] {
        build_heatmap_from_counts_for_day(self.counts, self.today)
    }
}

pub fn commits_per_day(repo: &gix::Repository, oids: impl IntoIterator<Item = gix::ObjectId>) -> [usize; TOTAL_DAYS] {
    // Use UTC dates so commits near midnight are bucketed consistently.
    let today: NaiveDate = Utc::now().date_naive();
    commits_per_day_for_day(repo, oids, today)
}

fn commits_per_day_for_day(repo: &gix::Repository, oids: impl IntoIterator<Item = gix::ObjectId>, today: NaiveDate) -> [usize; TOTAL_DAYS] {
    let mut counts = [0usize; TOTAL_DAYS];
    let mut object_buf = Vec::new();

    for oid in oids {
        object_buf.clear();
        let commit = match repo.objects.find_commit(oid.as_ref(), &mut object_buf) {
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

        if days_ago >= TOTAL_DAYS as i64 {
            continue;
        }

        counts[days_ago as usize] += 1;
    }

    counts
}

pub fn empty_heatmap() -> [[usize; WEEKS]; DAYS] {
    [[0usize; WEEKS]; DAYS]
}

pub fn build_heatmap(repo: &gix::Repository, oids: impl IntoIterator<Item = gix::ObjectId>) -> [[usize; WEEKS]; DAYS] {
    let today = Utc::now().date_naive();
    build_heatmap_from_counts_for_day(commits_per_day_for_day(repo, oids, today), today)
}

pub fn build_heatmap_from_sorted_aliases(repo: &gix::Repository, oids: &Oids) -> [[usize; WEEKS]; DAYS] {
    let today = Utc::now().date_naive();
    build_heatmap_from_counts_for_day(commits_per_day_for_day(repo, oids.get_sorted_aliases().iter().map(|alias| *oids.get_oid_by_alias(*alias)), today), today)
}

fn build_heatmap_from_counts_for_day(counts: [usize; TOTAL_DAYS], today: NaiveDate) -> [[usize; WEEKS]; DAYS] {
    // Rows are weekdays starting Monday, columns run oldest to newest.
    let mut grid = [[0usize; WEEKS]; DAYS];

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
    let style = color.map_or_else(Style::default, |c| Style::default().fg(c));
    Span::styled(format!("{:>2}", character), style)
}

#[cfg(test)]
#[path = "../tests/helpers/heatmap.rs"]
mod tests;
