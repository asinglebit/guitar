use crate::helpers::palette::Theme;
use chrono::{Duration, NaiveDate};
use im::HashMap;
use ratatui::{style::Style, text::Span};

pub const WEEKS: usize = 53;
pub const DAYS: usize = 7;

pub fn build_heatmap(
    counts: &HashMap<NaiveDate, usize>,
    today: NaiveDate,
) -> [[usize; WEEKS]; DAYS] {
    let mut grid = [[0usize; WEEKS]; DAYS];

    let start = today - Duration::days((WEEKS * 7) as i64);

    for week in 0..WEEKS {
        for (day, row) in grid.iter_mut().enumerate() {
            let date = start + Duration::days((week * 7 + day) as i64);
            row[week] = *counts.get(&date).unwrap_or(&0);
        }
    }

    grid
}

pub fn heat_cell(count: usize, theme: &Theme) -> Span<'_> {
    let (ch, color) = match count {
        0 => ("⋅ ", Some(theme.COLOR_GREY_800)),
        1 => ("⠁ ", Some(theme.COLOR_GRASS)),
        2 => ("⠃ ", Some(theme.COLOR_GRASS)),
        3 => ("⠇ ", Some(theme.COLOR_GRASS)),
        4 => ("⠏ ", Some(theme.COLOR_GRASS)),
        5 => ("⠟ ", Some(theme.COLOR_GRASS)),
        6 => ("⠿ ", Some(theme.COLOR_GRASS)),
        7 => ("⡿ ", Some(theme.COLOR_GRASS)),
        _ => ("⣿ ", Some(theme.COLOR_GRASS)),
    };

    let mut style = Style::default();
    if let Some(c) = color {
        style = style.fg(c);
    }

    Span::styled(ch, style)
}
