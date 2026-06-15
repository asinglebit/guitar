use crate::{app::app::PaneWindowCache, core::graph_service::GraphPaneRow, helpers::palette::Theme};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::ListItem,
};

pub(super) fn aligned_pane_rows(window: &PaneWindowCache, target_start: usize, target_end: usize) -> Option<Vec<Option<&GraphPaneRow>>> {
    if window.start >= target_end || target_start >= window.end {
        return None;
    }

    Some((target_start..target_end).map(|index| index.checked_sub(window.start).and_then(|offset| window.rows.get(offset))).collect())
}

pub(super) fn blank_lines(len: usize) -> Vec<Line<'static>> {
    vec![Line::default(); len]
}

pub(super) fn preloaded_pane_window(start: usize, end: usize, total_lines: usize, visible_height: usize) -> (usize, usize) {
    if total_lines == 0 || visible_height == 0 {
        return (start, end);
    }

    (start.saturating_sub(visible_height), end.saturating_add(visible_height).min(total_lines))
}

pub(super) fn zebra_list_items<'a>(lines: &[Line<'a>], visible_height: usize, global_start: usize, selected: usize, is_focused: bool, selection_enabled: bool, theme: &Theme) -> Vec<ListItem<'a>> {
    (0..visible_height)
        .map(|idx| {
            let line = lines.get(idx).cloned().unwrap_or_default();
            let global_idx = global_start + idx;
            let is_selected = selection_enabled && is_focused && global_idx == selected;

            let mut item = if is_selected {
                let spans: Vec<Span> = line.iter().map(|span| Span::styled(span.content.clone(), span.style)).collect();
                ListItem::new(Line::from(spans)).style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_800)))
            } else {
                ListItem::new(line)
            };

            if !is_selected && global_idx.is_multiple_of(2) {
                item = item.style(Style::default().bg(theme.background_or_default(theme.COLOR_GREY_900)));
            }

            item
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/app/draw/pane_window.rs"]
mod tests;
