use crate::{
    core::oids::Oids,
    helpers::{palette::Theme, symbols::SymbolTheme},
};
use chrono::{Datelike, NaiveDate};
use chrono::{TimeZone, Utc};
use git2::{Oid, Repository};
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
        if let Some(days_ago) = commit_days_ago(self.today, seconds) {
            self.counts[days_ago] += 1;
        }
    }

    pub fn build(&self) -> [[usize; WEEKS]; DAYS] {
        build_heatmap_from_counts_for_day(self.counts, self.today)
    }
}

pub fn commits_per_day(repo: &Repository, oids: impl IntoIterator<Item = Oid>) -> [usize; TOTAL_DAYS] {
    // Use UTC dates so commits near midnight are bucketed consistently.
    let today: NaiveDate = Utc::now().date_naive();
    commits_per_day_for_day(repo, oids, today)
}

fn commits_per_day_for_day(repo: &Repository, oids: impl IntoIterator<Item = Oid>, today: NaiveDate) -> [usize; TOTAL_DAYS] {
    let mut counts = [0usize; TOTAL_DAYS];

    for oid in oids {
        let Some(commit_date) = commit_date(repo, oid) else {
            continue;
        };

        match bucket_date(today, commit_date) {
            DateBucket::Count(days_ago) => counts[days_ago] += 1,
            DateBucket::Future => continue,
            DateBucket::BeforeWindow => continue,
        }
    }

    counts
}

pub fn empty_heatmap() -> [[usize; WEEKS]; DAYS] {
    [[0usize; WEEKS]; DAYS]
}

pub fn build_heatmap(repo: &Repository, oids: impl IntoIterator<Item = Oid>) -> [[usize; WEEKS]; DAYS] {
    let today = Utc::now().date_naive();
    build_heatmap_from_counts_for_day(commits_per_day_for_day(repo, oids, today), today)
}

pub fn build_heatmap_from_sorted_aliases(repo: &Repository, oids: &Oids) -> [[usize; WEEKS]; DAYS] {
    let today = Utc::now().date_naive();
    build_heatmap_from_counts_for_day(commits_per_day_for_day(repo, oids.get_sorted_aliases().iter().map(|alias| oids.get_oid_by_alias(*alias)), today), today)
}

fn build_heatmap_from_counts_for_day(counts: [usize; TOTAL_DAYS], today: NaiveDate) -> [[usize; WEEKS]; DAYS] {
    let weekday_today = today.weekday().num_days_from_monday() as usize;
    let mut grid = [[0usize; WEEKS]; DAYS];

    for (cell, count) in heatmap_cells(weekday_today, counts) {
        grid[cell.day][cell.week] = count;
    }

    grid
}

fn commit_date(repo: &Repository, oid: Oid) -> Option<NaiveDate> {
    let commit = repo.find_commit(oid).ok()?;
    Utc.timestamp_opt(commit.time().seconds(), 0).single().map(|date| date.date_naive())
}

fn bucket_date(today: NaiveDate, commit_date: NaiveDate) -> DateBucket {
    let days_ago = today.signed_duration_since(commit_date).num_days();

    if days_ago < 0 {
        DateBucket::Future
    } else if days_ago >= TOTAL_DAYS as i64 {
        DateBucket::BeforeWindow
    } else {
        DateBucket::Count(days_ago as usize)
    }
}

fn commit_days_ago(today: NaiveDate, seconds: i64) -> Option<usize> {
    let commit_date = Utc.timestamp_opt(seconds, 0).single()?.date_naive();
    let days_ago = today.signed_duration_since(commit_date).num_days();

    (0..TOTAL_DAYS as i64).contains(&days_ago).then_some(days_ago as usize)
}

fn heatmap_cells(count_weekday: usize, counts: [usize; TOTAL_DAYS]) -> impl Iterator<Item = (HeatmapCell, usize)> {
    counts.into_iter().enumerate().filter_map(move |(days_ago, count)| heatmap_cell(count_weekday, days_ago).map(|cell| (cell, count)))
}

fn heatmap_cell(weekday_today: usize, days_ago: usize) -> Option<HeatmapCell> {
    let offset = 6 - weekday_today;
    let logical = days_ago + offset;
    let week = logical / 7;
    (week < WEEKS).then(|| HeatmapCell { day: weekday_for_age(weekday_today, days_ago), week: WEEKS - 1 - week })
}

fn weekday_for_age(weekday_today: usize, days_ago: usize) -> usize {
    (weekday_today + DAYS - (days_ago % DAYS)) % DAYS
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct HeatmapCell {
    day: usize,
    week: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DateBucket {
    Count(usize),
    Future,
    BeforeWindow,
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
