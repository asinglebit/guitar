use crate::helpers::palette::Theme;
use chrono::{Datelike, NaiveDate};
use chrono::{TimeZone, Utc};
use git2::{Oid, Repository};
use im::HashMap;
use ratatui::{style::Style, text::Span};

pub const WEEKS: usize = 53;
pub const DAYS: usize = 7;

pub fn commits_per_day(repo: &git2::Repository, oids: &Vec<Oid>) -> HashMap<usize, usize> {
    let today: NaiveDate = Utc::now().date_naive(); // only date
    let mut counts: HashMap<usize, usize> = HashMap::new();

    for oid in oids {
        let commit = match repo.find_commit(*oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Convert commit time to NaiveDate
        let commit_date = Utc.timestamp_opt(commit.time().seconds(), 0).single().unwrap().date_naive();

        // Compute integer days ago
        let days_ago = today.signed_duration_since(commit_date).num_days();

        // Only count commits within 53 weeks (371 days)
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
    // One row for each day of the week, starting on Monday
    let mut grid = [[0usize; WEEKS]; DAYS];

    // Map: days_ago -> commit count
    let counts = commits_per_day(repo, oids);

    // Today in UTC, aligned to calendar days
    let today = Utc::now().date_naive();

    // 0 = Monday, 6 = Sunday
    let weekday_today = today.weekday().num_days_from_monday() as usize;

    // Total renderable cells
    let total_days = WEEKS * DAYS;

    // Offset so the last column start on the current day of the week
    let offset = 6 - weekday_today;

    for days_ago in 0..total_days {
        // Shift the logical position for presentation
        let logical = days_ago + offset;

        // Which week column this day belongs to
        let week = logical / 7;

        // Ignore anything beyond the oldest week
        if week >= WEEKS {
            continue;
        }

        // Get column index, from oldest to current
        let week_idx = WEEKS - 1 - week;

        // Get row index from Monday to Sunday
        let day_idx = (weekday_today + 6 - (logical % 7)) % 7;

        // Number of commits for this day
        let count = *counts.get(&days_ago).unwrap_or(&0);

        // Fill the heatmap cell
        grid[day_idx][week_idx] = count;
    }

    grid
}

pub fn heat_cell(count: usize, theme: &Theme) -> Span<'_> {
    let (character, color) = match count {
        0 => ("⠁", Some(theme.COLOR_TEXT)),
        1 => ("⠁", Some(theme.COLOR_GRASS)),
        2 => ("⠃", Some(theme.COLOR_GRASS)),
        3 => ("⠇", Some(theme.COLOR_GRASS)),
        4 => ("⠏", Some(theme.COLOR_GRASS)),
        5 => ("⠟", Some(theme.COLOR_GRASS)),
        6 => ("⠿", Some(theme.COLOR_GRASS)),
        7 => ("⡿", Some(theme.COLOR_GRASS)),
        _ => ("⣿", Some(theme.COLOR_GRASS)),
    };
    let style = if let Some(c) = color { Style::default().fg(c) } else { Style::default() };
    Span::styled(format!("{:>2}", character), style)
}
