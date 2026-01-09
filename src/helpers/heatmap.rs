use crate::helpers::palette::Theme;
use chrono::{TimeZone, Utc};
use git2::{Oid, Repository};
use im::HashMap;
use ratatui::{style::Style, text::Span};

pub const WEEKS: usize = 53;
pub const DAYS: usize = 7;

pub fn commits_per_day(repo: &Repository, oids: &Vec<Oid>) -> HashMap<usize, usize> {
    let today = Utc::now().date_naive();
    let mut counts: HashMap<usize, usize> = HashMap::new();

    for oid in oids {
        let commit = match repo.find_commit(*oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Convert commit time to DATE (UTC)
        let commit_date = Utc.timestamp_opt(commit.time().seconds(), 0).single().unwrap().date_naive();

        let days_ago = (today - commit_date).num_days();

        // Ignore anything outside the 53×7 window
        if !(0..371).contains(&days_ago) {
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
    let mut grid = [[0usize; WEEKS]; DAYS];
    let counts = commits_per_day(repo, oids);
    for week in 0..WEEKS {
        for (day, week_row) in grid.iter_mut().enumerate() {
            let day_index = week * DAYS + day;
            let days_ago = (WEEKS * DAYS) - 1 - day_index;
            week_row[week] = *counts.get(&days_ago).unwrap_or(&0);
        }
    }
    grid
}

pub fn heat_cell(count: usize, theme: &Theme) -> Span<'_> {
    let (character, color) = match count {
        0 => ("⠁ ", Some(theme.COLOR_GREY_800)),
        1 => ("⠁ ", Some(theme.COLOR_GRASS)),
        2 => ("⠃ ", Some(theme.COLOR_GRASS)),
        3 => ("⠇ ", Some(theme.COLOR_GRASS)),
        4 => ("⠏ ", Some(theme.COLOR_GRASS)),
        5 => ("⠟ ", Some(theme.COLOR_GRASS)),
        6 => ("⠿ ", Some(theme.COLOR_GRASS)),
        7 => ("⡿ ", Some(theme.COLOR_GRASS)),
        _ => ("⣿ ", Some(theme.COLOR_GRASS)),
    };
    let style = if let Some(c) = color { Style::default().fg(c) } else { Style::default() };
    Span::styled(character, style)
}
