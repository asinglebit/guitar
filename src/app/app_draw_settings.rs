use crate::helpers::heatmap::heat_cell;
use crate::helpers::keymap::{InputMode, action_keymap_visible_entries};
use crate::helpers::palette::*;
use crate::helpers::symbols::WEEKDAY_LABELS;
use crate::helpers::version::VERSION;
use crate::{
    app::app::{App, Direction, Focus},
    core::renderers::render_keybindings,
    git::queries::commits::get_git_user_info,
    helpers::text::fill_width,
};
use ratatui::widgets::Borders;
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_settings(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        // Settings owns the center viewport and uses centered rows throughout.
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        let available_width = self.layout.graph.width.saturating_sub(1) as usize;

        // Credentials are read live so the settings view reflects git config changes.
        let (name, email) = get_git_user_info(repo).unwrap();

        // settings_selections maps interactive rows back to their line indices.
        let mut lines: Vec<Line> = Vec::new();
        self.settings_selections = Vec::new();
        lines.push(Line::default());

        // Each heat cell renders as two terminal columns.
        let cell_width = 2;

        // Outer margins are reserved so the centered content breathes in narrow terminals.
        let border_width = 8;

        let weekday_label_width = 2;
        let usable_width = available_width.saturating_sub(border_width).saturating_sub(weekday_label_width);

        // Only the most recent weeks that fit are rendered.
        let max_weeks_fit = (usable_width / cell_width).max(1);
        let total_weeks = self.heatmap[0].len();
        let visible_weeks = max_weeks_fit.min(total_weeks);

        let week_start = (total_weeks.saturating_sub(visible_weeks).saturating_add(2)).min(total_weeks);

        // All settings rows align to the heatmap body width.
        let heatmap_width = visible_weeks * cell_width;

        lines.push(Line::default());
        lines.push(
            Line::from(Span::styled(
                fill_width(" version:", format!("{} ", VERSION).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );
        self.settings_selections.push(lines.len().saturating_sub(1));

        // Heatmap rows use weekday labels followed by the cropped commit grid.
        lines.push(Line::default());
        for (day_idx, &label) in WEEKDAY_LABELS.iter().enumerate() {
            let mut spans = Vec::new();
            spans.push(Span::styled(format!(" {}  ", label), Style::default().fg(self.theme.COLOR_TEXT)));
            spans.extend(self.heatmap[day_idx][week_start..].iter().map(|&count| heat_cell(count, &self.theme)));
            lines.push(Line::from(spans).centered());
        }

        // Config paths are informational, but still selectable for consistent navigation.
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(fill_width(" paths:", "", heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
        lines.push(Line::default());
        let mut pathbuf = dirs::config_dir().unwrap();
        pathbuf.push("guitar");
        let path = pathbuf.as_path().to_str().unwrap();
        lines.push(
            Line::from(Span::styled(
                fill_width(" keymap:", format!(" {}/keymap.json ", path).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );
        self.settings_selections.push(lines.len().saturating_sub(1));
        lines.push(Line::from(Span::styled(fill_width(" layout:", format!(" {}/layout.json ", path).as_str(), heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))).centered());
        self.settings_selections.push(lines.len().saturating_sub(1));
        lines.push(
            Line::from(Span::styled(
                fill_width(" recent repositories:", format!(" {}/recent.json ", path).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );
        self.settings_selections.push(lines.len().saturating_sub(1));

        // Credential rows are selectable because they are important setup information.
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(fill_width(" credentials:", "", heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
        lines.push(Line::default());
        lines.push(
            Line::from(Span::styled(
                fill_width(" name:", format!("{} ", name.unwrap()).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );

        // Selectable rows are recorded immediately after being pushed.
        self.settings_selections.push(lines.len().saturating_sub(1));
        lines.push(Line::from(Span::styled(fill_width(" email:", format!("{} ", email.unwrap()).as_str(), heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))).centered());

        self.settings_selections.push(lines.len().saturating_sub(1));
        lines.push(
            Line::from(Span::styled(
                fill_width(" authorization:", "external ssh agent ", heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );

        self.settings_selections.push(lines.len().saturating_sub(1));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(fill_width(" themes:", "", heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))).centered());
        lines.push(Line::default());
        lines.push(
            Line::from(Span::styled(
                fill_width(" classic", format!("({}) ", if self.theme.name == ThemeNames::Classic { "*" } else { " " }).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );

        self.settings_selections.push(lines.len() - 1);
        lines.push(
            Line::from(Span::styled(
                fill_width(" ansi", format!("({}) ", if self.theme.name == ThemeNames::Ansi { "*" } else { " " }).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT),
            ))
            .centered(),
        );

        self.settings_selections.push(lines.len().saturating_sub(1));
        lines.push(
            Line::from(Span::styled(
                fill_width(" monochrome", format!("({}) ", if self.theme.name == ThemeNames::Monochrome { "*" } else { " " }).as_str(), heatmap_width),
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ))
            .centered(),
        );

        self.settings_selections.push(lines.len().saturating_sub(1));

        // Keymap sections are generated from the active keymap data, not duplicated text.
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(fill_width(" shortcuts / normal mode:", "", heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))).centered());
        lines.push(Line::default());
        if let Some(mode_keymap) = self.keymaps.get(&InputMode::Normal) {
            render_keybindings(&self.theme, mode_keymap, heatmap_width).iter().enumerate().for_each(|(idx, kb_line)| {
                let spans: Vec<Span> = kb_line
                    .clone()
                    .spans
                    .iter()
                    .map(|span| {
                        let mut style = span.style;
                        if idx % 2 == 0 {
                            style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
                        }
                        Span::styled(span.content.clone(), style)
                    })
                    .collect();
                lines.push(Line::from(spans).centered());

                self.settings_selections.push(lines.len().saturating_sub(1));
            });
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(fill_width(" shortcuts / action mode:", "", heatmap_width), Style::default().fg(self.theme.COLOR_TEXT))).centered());
        lines.push(Line::default());
        if let Some(action_keymap) = self.keymaps.get(&InputMode::Action) {
            // Action mode hides inherited duplicates, but shows keys whose command changes.
            let unique_action = action_keymap_visible_entries(self.keymaps.get(&InputMode::Normal), action_keymap);

            render_keybindings(&self.theme, &unique_action, heatmap_width).iter().enumerate().for_each(|(idx, kb_line)| {
                let spans: Vec<Span> = kb_line
                    .clone()
                    .spans
                    .iter()
                    .map(|span| {
                        let mut style = span.style;
                        if idx % 2 == 0 {
                            style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
                        }
                        Span::styled(span.content.clone(), style)
                    })
                    .collect();

                lines.push(Line::from(spans).centered());
                self.settings_selections.push(lines.len().saturating_sub(1));
            });
        }

        // Settings uses sticky scroll so the selected row stays near the bottom while moving down.
        let total_lines = lines.len();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        // Navigation lands on selectable rows even when the rendered list contains headings.
        if !self.settings_selections.contains(&self.settings_selected) {
            let mut nearest = None;

            if self.last_input_direction == Some(Direction::Down) {
                nearest = self.settings_selections.iter().copied().find(|&i| i > self.settings_selected);
            }

            if nearest.is_none() && self.last_input_direction == Some(Direction::Up) {
                nearest = self.settings_selections.iter().rev().copied().find(|&i| i < self.settings_selected);
            }

            // Without direction, choose the closest selectable row by distance.
            if nearest.is_none() {
                nearest = self.settings_selections.iter().min_by_key(|&&i| i.abs_diff(self.settings_selected)).copied();
            }

            if let Some(target) = nearest {
                self.settings_selected = target;
            }
        }

        let start = (self.settings_selected + 1).saturating_sub(visible_height);
        let end = (start + visible_height).min(total_lines);

        // Ensure blank lines still occupy space after conversion to ListItem.
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let absolute_idx = start + i;
                let mut item = line.clone();

                if item.spans.is_empty() {
                    item.spans.push(Span::raw(" "));
                }

                // Highlight only while settings has viewport focus.
                if absolute_idx == self.settings_selected && self.focus == Focus::Viewport {
                    let spans: Vec<Span> = item
                        .spans
                        .iter()
                        .map(|span| {
                            let mut style = span.style;
                            style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_800));
                            Span::styled(span.content.clone(), style)
                        })
                        .collect();
                    item = Line::from(spans).centered();
                }

                ListItem::from(item)
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames settings as a standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded).padding(padding));

            frame.render_widget(list, self.layout.graph);

            let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(start);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol("▌")
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.app, &mut scrollbar_state);

            return;
        }

        // Normal mode settings reuses the graph viewport without graph borders.
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.graph);

        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(start);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("╮"))
            .end_symbol(Some("╯"))
            .track_symbol(Some("│"))
            .thumb_symbol("▌")
            .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.app, &mut scrollbar_state);
    }
}
