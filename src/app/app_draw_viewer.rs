use crate::{
    app::{
        app::{App, Focus, ViewerLayoutSignature, Viewport},
        app_default::{SplitViewerRow, ViewerMode},
    },
    git::queries::{
        diffs::{get_conflict_file, get_file_at_oid, get_file_at_workdir, get_file_diff_at_oid, get_file_diff_at_workdir},
        helpers::{ConflictFile, Hunk},
    },
    helpers::text::wrap_words,
};
use git2::Oid;
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

#[derive(Clone)]
struct SplitCell {
    number: usize,
    origin: char,
    text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConflictSection {
    Normal,
    Ancestor,
    Ours,
    Theirs,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConflictMarker {
    Start,
    Ancestor,
    Separator,
    End,
}

fn conflict_marker(line: &str) -> Option<ConflictMarker> {
    if line.starts_with("<<<<<<<") {
        Some(ConflictMarker::Start)
    } else if line.starts_with("|||||||") {
        Some(ConflictMarker::Ancestor)
    } else if line.starts_with("=======") {
        Some(ConflictMarker::Separator)
    } else if line.starts_with(">>>>>>>") {
        Some(ConflictMarker::End)
    } else {
        None
    }
}

impl App {
    pub fn current_viewer_layout_signature(&self) -> ViewerLayoutSignature {
        ViewerLayoutSignature { graph_width: self.layout.graph.width, split_left_width: self.layout.viewer_split_left.width, split_right_width: self.layout.viewer_split_right.width }
    }

    pub fn viewer_row_count(&self) -> usize {
        match self.viewer_mode {
            ViewerMode::Full => self.viewer_lines.len(),
            ViewerMode::Hunks => self.viewer_hunks.len(),
            ViewerMode::Split => self.viewer_split_rows.len(),
        }
    }

    pub fn split_unified_index(&self, split_idx: usize) -> usize {
        self.viewer_split_rows.get(split_idx).and_then(|row| row.unified_indices.first().copied()).unwrap_or(0)
    }

    pub fn closest_split_row_for_unified(&self, unified_idx: usize) -> usize {
        self.viewer_split_rows.iter().enumerate().min_by_key(|(_, row)| row.unified_indices.iter().map(|idx| idx.abs_diff(unified_idx)).min().unwrap_or(usize::MAX)).map(|(idx, _)| idx).unwrap_or(0)
    }

    pub fn draw_viewer(&mut self, frame: &mut Frame) {
        if self.viewer_mode == ViewerMode::Split {
            self.draw_split_viewer(frame);
            return;
        }

        // Viewer content gets horizontal padding so diff prefixes do not touch borders.
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        // Retained for quick tuning of wrap width while working on the viewer layout.
        // let available_width = self.layout.graph.width as usize - 1;
        // let max_text_width = available_width.saturating_sub(2);

        // Hunk mode presents only changed-line anchors while reusing the full viewer rows.
        let active_lines: Vec<&ListItem> = match self.viewer_mode {
            ViewerMode::Full => self.viewer_lines.iter().collect(),
            ViewerMode::Hunks => self.viewer_hunks.iter().filter_map(|&i| self.viewer_lines.get(i)).collect(),
            ViewerMode::Split => Vec::new(),
        };

        let total_lines = active_lines.len();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        if total_lines == 0 {
            self.viewer_selected = 0;
        } else if self.viewer_selected >= total_lines {
            self.viewer_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.viewer_selected, &self.viewer_scroll, total_lines, visible_height);

        let start = self.viewer_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Clone visible rows so selection styling does not mutate the cached viewer data.
        let list_items: Vec<ListItem> = active_lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let absolute_idx = start + i;
                let mut item = (*line).clone();
                if absolute_idx == self.viewer_selected && self.focus == Focus::Viewport {
                    item = item.style(Style::default().bg(self.theme.COLOR_GREY_800));
                }
                item
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames the viewer as a standalone surface.
            let list = List::new(list_items)
                .block(Block::default().padding(padding).borders(Borders::ALL).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded));

            frame.render_widget(list, self.layout.graph);

            let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.viewer_scroll.get());
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("╮"))
                .end_symbol(Some("╯"))
                .track_symbol(Some("│"))
                .thumb_symbol("▌")
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);

            return;
        }

        // Normal mode shares the graph viewport borders with neighboring panes.
        let list = List::new(list_items).block(
            Block::default().padding(padding).borders(Borders::RIGHT | Borders::LEFT).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded),
        );

        frame.render_widget(list, self.layout.graph);

        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.viewer_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(if self.layout_config.is_inspector || self.layout_config.is_status { Some("─") } else { Some("╮") })
            .end_symbol(if self.layout_config.is_inspector || self.layout_config.is_status { Some("─") } else { Some("╯") })
            .track_symbol(Some("│"))
            .thumb_symbol("▌")
            .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
    }

    fn draw_split_viewer(&mut self, frame: &mut Frame) {
        if self.layout.viewer_split_left.width == 0 || self.layout.viewer_split_right.width == 0 {
            return;
        }

        let total_lines = self.viewer_split_rows.len();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        if total_lines == 0 {
            self.viewer_selected = 0;
        } else if self.viewer_selected >= total_lines {
            self.viewer_selected = total_lines.saturating_sub(1);
        }

        self.trap_selection(self.viewer_selected, &self.viewer_scroll, total_lines, visible_height);

        let start = self.viewer_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        let mut left_items = Vec::new();
        let mut right_items = Vec::new();

        for (i, row) in self.viewer_split_rows[start..end].iter().enumerate() {
            let absolute_idx = start + i;
            let mut left = row.left.clone();
            let mut right = row.right.clone();
            if absolute_idx == self.viewer_selected && self.focus == Focus::Viewport {
                let selected = Style::default().bg(self.theme.COLOR_GREY_800);
                left = left.style(selected);
                right = right.style(selected);
            }
            left_items.push(left);
            right_items.push(right);
        }

        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };
        let left_borders = if self.layout_config.is_zen { Borders::LEFT | Borders::TOP | Borders::BOTTOM } else { Borders::LEFT };
        let right_borders = if self.layout_config.is_zen { Borders::RIGHT | Borders::TOP | Borders::BOTTOM } else { Borders::RIGHT };

        let left_list = List::new(left_items)
            .block(Block::default().padding(padding).borders(left_borders).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded));
        let right_list = List::new(right_items)
            .block(Block::default().padding(padding).borders(right_borders).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded));

        frame.render_widget(left_list, self.layout.viewer_split_left);
        frame.render_widget(right_list, self.layout.viewer_split_right);
        self.draw_split_divider(frame);

        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.viewer_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(if !self.layout_config.is_zen && (self.layout_config.is_inspector || self.layout_config.is_status) { Some("─") } else { Some("╮") })
            .end_symbol(if !self.layout_config.is_zen && (self.layout_config.is_inspector || self.layout_config.is_status) { Some("─") } else { Some("╯") })
            .track_symbol(Some("│"))
            .thumb_symbol("▌")
            .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
    }

    fn draw_split_divider(&self, frame: &mut Frame) {
        let area = self.layout.divider_viewer_split;
        if area.width == 0 || area.height == 0 {
            return;
        }

        let lines: Vec<Line> = (0..area.height)
            .map(|y| {
                let symbol = if self.layout_config.is_zen && y == 0 {
                    "┬"
                } else if self.layout_config.is_zen && y + 1 == area.height {
                    "┴"
                } else {
                    "│"
                };
                Line::from(Span::styled(symbol, Style::default().fg(self.theme.COLOR_BORDER)))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), area);
    }

    // Resolve the selected status row into a repository-relative path.
    pub fn get_selected_file_name(&self) -> Option<String> {
        match self.focus {
            Focus::StatusTop => {
                if self.graph_selected != 0 && !self.current_diff.is_empty() {
                    // Commit status rows come from current_diff.
                    Some(self.current_diff.get(self.status_top_selected)?.filename.to_string())
                } else if self.graph_selected == 0 && self.uncommitted.is_staged {
                    // Staged uncommitted rows are grouped conflicts, modified, added, then deleted.
                    let conflict_len = self.uncommitted.conflicts.len();
                    let modified_len = self.uncommitted.staged.modified.len();
                    let added_len = self.uncommitted.staged.added.len();
                    let index = self.status_top_selected;

                    if index < conflict_len {
                        self.uncommitted.conflicts.get(index).cloned()
                    } else if index < conflict_len + modified_len {
                        self.uncommitted.staged.modified.get(index - conflict_len).cloned()
                    } else if index < conflict_len + modified_len + added_len {
                        self.uncommitted.staged.added.get(index - conflict_len - modified_len).cloned()
                    } else {
                        self.uncommitted.staged.deleted.get(index - conflict_len - modified_len - added_len).cloned()
                    }
                } else {
                    None
                }
            },
            Focus::StatusBottom => {
                if self.graph_selected == 0 && self.uncommitted.is_unstaged {
                    // Unstaged rows use the same grouping as staged rows.
                    let conflict_len = self.uncommitted.conflicts.len();
                    let modified_len = self.uncommitted.unstaged.modified.len();
                    let added_len = self.uncommitted.unstaged.added.len();
                    let index = self.status_bottom_selected;

                    if index < conflict_len {
                        self.uncommitted.conflicts.get(index).cloned()
                    } else if index < conflict_len + modified_len {
                        self.uncommitted.unstaged.modified.get(index - conflict_len).cloned()
                    } else if index < conflict_len + modified_len + added_len {
                        self.uncommitted.unstaged.added.get(index - conflict_len - modified_len).cloned()
                    } else {
                        self.uncommitted.unstaged.deleted.get(index - conflict_len - modified_len - added_len).cloned()
                    }
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    pub fn open_viewer(&mut self, repo: &git2::Repository) {
        if let Some(file_name) = self.get_selected_file_name() {
            self.file_name = Some(file_name);
            let oid = if self.graph_selected != 0 { self.oids.get_oid_by_idx(self.graph_selected) } else { &Oid::zero() };
            self.update_viewer(*oid, repo);
            self.viewport = Viewport::Viewer;
        }
    }

    pub fn refresh_viewer_for_layout_change(&mut self) {
        if self.viewport != Viewport::Viewer || self.file_name.is_none() {
            return;
        }
        let Some(repo) = self.repo.clone() else {
            return;
        };

        let old_mode = self.viewer_mode;
        let old_unified_idx = match old_mode {
            ViewerMode::Full => self.viewer_selected,
            ViewerMode::Hunks => self.viewer_hunks.get(self.viewer_selected).copied().unwrap_or(0),
            ViewerMode::Split => self.split_unified_index(self.viewer_selected),
        };
        let oid = if self.graph_selected != 0 { *self.oids.get_oid_by_idx(self.graph_selected) } else { Oid::zero() };

        self.update_viewer(oid, &repo);
        self.viewer_mode = old_mode;
        self.viewer_selected = match old_mode {
            ViewerMode::Full => old_unified_idx.min(self.viewer_lines.len().saturating_sub(1)),
            ViewerMode::Hunks => self.viewer_hunks.iter().enumerate().min_by_key(|(_, h)| h.abs_diff(old_unified_idx)).map(|(idx, _)| idx).unwrap_or(0),
            ViewerMode::Split => self.closest_split_row_for_unified(old_unified_idx),
        };
        self.viewer_scroll.set(self.viewer_selected);
    }

    pub fn mark_viewer_layout_dirty(&mut self) {
        if self.viewport == Viewport::Viewer {
            self.is_viewer_layout_dirty = true;
        }
    }

    pub fn update_viewer(&mut self, oid: Oid, repo: &git2::Repository) {
        // The selected filename is owned by App so viewer reloads can reuse it.
        let filename = self.file_name.clone().unwrap();

        if oid == Oid::zero()
            && self.uncommitted.conflicts.iter().any(|path| path == &filename)
            && let Ok(Some(conflict)) = get_conflict_file(repo, &filename)
        {
            self.update_conflict_viewer(conflict);
            return;
        }

        // Oid::zero represents the uncommitted pseudo-row and reads from the working tree.
        let (original_lines, hunks) = if oid == Oid::zero() {
            (get_file_at_workdir(repo, &filename), get_file_diff_at_workdir(repo, &filename).unwrap_or_default())
        } else {
            (get_file_at_oid(repo, oid, &filename), get_file_diff_at_oid(repo, oid, &filename).unwrap_or_default())
        };

        self.viewer_lines.clear();
        self.viewer_split_rows.clear();
        self.viewer_edges.clear();
        self.viewer_hunks.clear();
        let mut current_line: usize = 0;
        let mut current_line_old: usize = 0;

        // Origin changes mark useful navigation edges inside a diff.
        let mut last_origin: Option<char> = None;

        for hunk in hunks.iter() {
            let header = &hunk.header;
            let old_start_idx: usize = header.old_start.saturating_sub(1) as usize;

            // Fill unchanged file content before the next hunk starts.
            while current_line < old_start_idx && current_line < original_lines.len() {
                let wrapped = wrap_words(original_lines[current_line].clone(), (self.layout.graph.width as usize).saturating_sub(8));
                for (idx, line) in wrapped.into_iter().enumerate() {
                    self.viewer_lines.push(ListItem::new(
                        Line::from(vec![
                            Span::styled((if idx == 0 { format!("{:3}  ", current_line + 1) } else { "     ".to_string() }).to_string(), Style::default().fg(self.theme.COLOR_BORDER)),
                            Span::styled(line.to_string(), Style::default().fg(self.theme.COLOR_GREY_500)),
                        ])
                        .style(Style::default()),
                    ));
                }
                current_line += 1;
                current_line_old += 1;
            }

            // Process patch lines after dropping hunk header marker lines.
            for line in hunk.lines.iter().filter(|l| l.origin != 'H') {
                let text = line.content.trim_end_matches('\n');

                // Store edge positions where additions, removals, and context switch.
                if let Some(prev) = last_origin
                    && prev != line.origin
                {
                    self.viewer_edges.push(self.viewer_lines.len().saturating_sub(1));
                }
                last_origin = Some(line.origin);

                // Line origin controls prefix, color, and which side's counter advances.
                let (style, prefix, side, fg, count) = match line.origin {
                    '-' => (Style::default().bg(self.theme.COLOR_DARK_RED).fg(self.theme.COLOR_RED), "- ".to_string(), self.theme.COLOR_RED, self.theme.COLOR_RED, current_line_old + 1),
                    '+' => (Style::default().bg(self.theme.COLOR_LIGHT_GREEN_900).fg(self.theme.COLOR_GREEN), "+ ".to_string(), self.theme.COLOR_GREEN, self.theme.COLOR_GREEN, current_line + 1),
                    ' ' => (Style::default(), "".to_string(), self.theme.COLOR_BORDER, self.theme.COLOR_GREY_500, current_line + 1),
                    _ => (Style::default(), "".to_string(), self.theme.COLOR_BORDER, self.theme.COLOR_GREY_500, 0),
                };

                let wrapped = wrap_words(format!("{}{}", prefix, text), (self.layout.graph.width as usize).saturating_sub(9));
                for (idx, line_wrapped) in wrapped.into_iter().enumerate() {
                    // Hunk mode indexes only changed rows.
                    if line.origin != ' ' {
                        self.viewer_hunks.push(self.viewer_lines.len());
                    }

                    self.viewer_lines.push(
                        ListItem::new(Line::from(vec![
                            Span::styled((if idx == 0 { format!("{:3}  ", count) } else { "     ".to_string() }).to_string(), Style::default().fg(side)),
                            Span::styled(line_wrapped.to_string(), Style::default().fg(fg)),
                        ]))
                        .style(style),
                    );
                }

                // Deleted lines advance only the old side, added lines only the new side.
                match line.origin {
                    '-' => {
                        current_line_old += 1;
                    },
                    '+' => {
                        current_line += 1;
                    },
                    ' ' => {
                        current_line += 1;
                        current_line_old += 1;
                    },
                    _ => {},
                }
            }
        }

        // Append unchanged file content after the final hunk.
        while current_line < original_lines.len() {
            let wrapped = wrap_words(original_lines[current_line].clone(), (self.layout.graph.width as usize).saturating_sub(8));
            for (idx, line) in wrapped.into_iter().enumerate() {
                self.viewer_lines.push(
                    ListItem::new(Line::from(vec![
                        Span::styled((if idx == 0 { format!("{:3}  ", current_line + 1) } else { "     ".to_string() }).to_string(), Style::default().fg(self.theme.COLOR_BORDER)),
                        Span::styled(line.to_string(), Style::default().fg(self.theme.COLOR_GREY_500)),
                    ]))
                    .style(Style::default()),
                );
            }
            current_line += 1;
        }

        self.build_split_viewer_rows(&original_lines, &hunks);
        self.viewer_selected = self.viewer_edges.first().copied().unwrap_or(0);
    }

    fn update_conflict_viewer(&mut self, conflict: ConflictFile) {
        self.viewer_lines.clear();
        self.viewer_split_rows.clear();
        self.viewer_edges.clear();
        self.viewer_hunks.clear();

        let mut section = ConflictSection::Normal;
        for (idx, line) in conflict.workdir.iter().enumerate() {
            let marker = conflict_marker(line);
            let origin = match marker {
                Some(ConflictMarker::Start) => {
                    section = ConflictSection::Ours;
                    '!'
                },
                Some(ConflictMarker::Ancestor) => {
                    section = ConflictSection::Ancestor;
                    '!'
                },
                Some(ConflictMarker::Separator) => {
                    section = ConflictSection::Theirs;
                    '!'
                },
                Some(ConflictMarker::End) => {
                    section = ConflictSection::Normal;
                    '!'
                },
                None => match section {
                    ConflictSection::Ours => '+',
                    ConflictSection::Theirs => '-',
                    ConflictSection::Ancestor | ConflictSection::Normal => ' ',
                },
            };

            if origin != ' ' {
                self.viewer_hunks.push(self.viewer_lines.len());
            }
            if marker.is_some() {
                self.viewer_edges.push(self.viewer_lines.len());
            }

            self.push_conflict_unified_line(idx + 1, origin, line);
        }

        self.build_conflict_split_rows(&conflict);
        self.viewer_selected = self.viewer_edges.first().copied().unwrap_or(0);
    }

    fn push_conflict_unified_line(&mut self, number: usize, origin: char, text: &str) {
        let (style, prefix, number_fg, text_fg) = match origin {
            '!' => (Style::default().fg(self.theme.COLOR_ORANGE), "! ", self.theme.COLOR_ORANGE, self.theme.COLOR_ORANGE),
            '-' => (Style::default().bg(self.theme.COLOR_DARK_RED).fg(self.theme.COLOR_RED), "- ", self.theme.COLOR_RED, self.theme.COLOR_RED),
            '+' => (Style::default().bg(self.theme.COLOR_LIGHT_GREEN_900).fg(self.theme.COLOR_GREEN), "+ ", self.theme.COLOR_GREEN, self.theme.COLOR_GREEN),
            _ => (Style::default(), "", self.theme.COLOR_BORDER, self.theme.COLOR_GREY_500),
        };

        let wrapped = wrap_words(format!("{}{}", prefix, text), (self.layout.graph.width as usize).saturating_sub(9));
        for (idx, line_wrapped) in wrapped.into_iter().enumerate() {
            self.viewer_lines.push(
                ListItem::new(Line::from(vec![
                    Span::styled(if idx == 0 { format!("{:3}  ", number) } else { "     ".to_string() }, Style::default().fg(number_fg)),
                    Span::styled(line_wrapped, Style::default().fg(text_fg)),
                ]))
                .style(style),
            );
        }
    }

    fn build_conflict_split_rows(&mut self, conflict: &ConflictFile) {
        let (left_width, right_width) = self.split_pane_text_widths();
        let mut section = ConflictSection::Normal;
        let mut ours_line = 1;
        let mut theirs_line = 1;
        let mut rendered_marker = false;

        for (idx, line) in conflict.workdir.iter().enumerate() {
            let source_idx = idx.min(self.viewer_lines.len().saturating_sub(1));
            match conflict_marker(line) {
                Some(ConflictMarker::Start) => {
                    rendered_marker = true;
                    section = ConflictSection::Ours;
                    let marker = SplitCell { number: idx + 1, origin: '!', text: line.clone() };
                    self.push_split_pair(Some(marker.clone()), Some(marker), left_width, right_width, vec![source_idx]);
                },
                Some(ConflictMarker::Ancestor) => {
                    rendered_marker = true;
                    section = ConflictSection::Ancestor;
                    let marker = SplitCell { number: idx + 1, origin: '!', text: line.clone() };
                    self.push_split_pair(Some(marker.clone()), Some(marker), left_width, right_width, vec![source_idx]);
                },
                Some(ConflictMarker::Separator) => {
                    rendered_marker = true;
                    section = ConflictSection::Theirs;
                    let marker = SplitCell { number: idx + 1, origin: '!', text: line.clone() };
                    self.push_split_pair(Some(marker.clone()), Some(marker), left_width, right_width, vec![source_idx]);
                },
                Some(ConflictMarker::End) => {
                    rendered_marker = true;
                    section = ConflictSection::Normal;
                    let marker = SplitCell { number: idx + 1, origin: '!', text: line.clone() };
                    self.push_split_pair(Some(marker.clone()), Some(marker), left_width, right_width, vec![source_idx]);
                },
                None => match section {
                    ConflictSection::Normal => {
                        let left = SplitCell { number: idx + 1, origin: ' ', text: line.clone() };
                        let right = SplitCell { number: idx + 1, origin: ' ', text: line.clone() };
                        self.push_split_pair(Some(left), Some(right), left_width, right_width, vec![source_idx]);
                    },
                    ConflictSection::Ancestor => {
                        let ancestor = SplitCell { number: idx + 1, origin: '!', text: line.clone() };
                        self.push_split_pair(Some(ancestor.clone()), Some(ancestor), left_width, right_width, vec![source_idx]);
                    },
                    ConflictSection::Ours => {
                        let left = SplitCell { number: ours_line, origin: '-', text: line.clone() };
                        self.push_split_pair(Some(left), None, left_width, right_width, vec![source_idx]);
                        ours_line += 1;
                    },
                    ConflictSection::Theirs => {
                        let right = SplitCell { number: theirs_line, origin: '+', text: line.clone() };
                        self.push_split_pair(None, Some(right), left_width, right_width, vec![source_idx]);
                        theirs_line += 1;
                    },
                },
            }
        }

        if rendered_marker || conflict.ours.is_empty() || conflict.theirs.is_empty() {
            return;
        }

        let rows = conflict.ours.len().max(conflict.theirs.len()).max(1);
        for row in 0..rows {
            let source_idx = row.min(self.viewer_lines.len().saturating_sub(1));
            let left = conflict.ours.get(row).map(|text| SplitCell { number: row + 1, origin: '-', text: text.clone() });
            let right = conflict.theirs.get(row).map(|text| SplitCell { number: row + 1, origin: '+', text: text.clone() });
            self.push_split_pair(left, right, left_width, right_width, vec![source_idx]);
        }
    }

    fn build_split_viewer_rows(&mut self, original_lines: &[String], hunks: &[Hunk]) {
        self.viewer_split_rows.clear();

        let (left_width, right_width) = self.split_pane_text_widths();
        let mut current_line: usize = 0;
        let mut current_line_old: usize = 0;
        let mut unified_idx: usize = 0;

        for hunk in hunks {
            let new_start_idx = hunk.header.new_start.saturating_sub(1) as usize;

            while current_line < new_start_idx && current_line < original_lines.len() {
                let text = original_lines[current_line].clone();
                let source_idx = unified_idx;
                unified_idx += self.unified_unchanged_wrap_count(&text);
                let old = SplitCell { number: current_line_old + 1, origin: ' ', text: text.clone() };
                let new = SplitCell { number: current_line + 1, origin: ' ', text };
                self.push_split_pair(Some(old), Some(new), left_width, right_width, vec![source_idx]);
                current_line += 1;
                current_line_old += 1;
            }

            let lines: Vec<_> = hunk.lines.iter().filter(|line| line.origin != 'H').collect();
            let mut idx = 0;

            while idx < lines.len() {
                match lines[idx].origin {
                    '-' => {
                        let old_start = current_line_old;
                        let mut removed = Vec::new();
                        while idx < lines.len() && lines[idx].origin == '-' {
                            let text = lines[idx].content.trim_end_matches('\n').to_string();
                            let source_idx = unified_idx;
                            unified_idx += self.unified_changed_wrap_count('-', &text);
                            removed.push((text, source_idx));
                            idx += 1;
                        }

                        let new_start = current_line;
                        let mut added = Vec::new();
                        while idx < lines.len() && lines[idx].origin == '+' {
                            let text = lines[idx].content.trim_end_matches('\n').to_string();
                            let source_idx = unified_idx;
                            unified_idx += self.unified_changed_wrap_count('+', &text);
                            added.push((text, source_idx));
                            idx += 1;
                        }

                        let rows = removed.len().max(added.len());
                        for row in 0..rows {
                            let mut source_indices = Vec::new();
                            if let Some((_, source_idx)) = removed.get(row) {
                                source_indices.push(*source_idx);
                            }
                            if let Some((_, source_idx)) = added.get(row) {
                                source_indices.push(*source_idx);
                            }
                            if source_indices.is_empty() {
                                source_indices.push(unified_idx);
                            }
                            let old = removed.get(row).map(|(text, _)| SplitCell { number: old_start + row + 1, origin: '-', text: text.clone() });
                            let new = added.get(row).map(|(text, _)| SplitCell { number: new_start + row + 1, origin: '+', text: text.clone() });
                            self.push_split_pair(old, new, left_width, right_width, source_indices);
                        }

                        current_line_old += removed.len();
                        current_line += added.len();
                    },
                    '+' => {
                        let new_start = current_line;
                        let mut added = Vec::new();
                        while idx < lines.len() && lines[idx].origin == '+' {
                            let text = lines[idx].content.trim_end_matches('\n').to_string();
                            let source_idx = unified_idx;
                            unified_idx += self.unified_changed_wrap_count('+', &text);
                            added.push((text, source_idx));
                            idx += 1;
                        }

                        for (row, (text, source_idx)) in added.iter().enumerate() {
                            let new = SplitCell { number: new_start + row + 1, origin: '+', text: text.clone() };
                            self.push_split_pair(None, Some(new), left_width, right_width, vec![*source_idx]);
                        }

                        current_line += added.len();
                    },
                    ' ' => {
                        let text = lines[idx].content.trim_end_matches('\n').to_string();
                        let source_idx = unified_idx;
                        unified_idx += self.unified_changed_wrap_count(' ', &text);
                        let old = SplitCell { number: current_line_old + 1, origin: ' ', text: text.clone() };
                        let new = SplitCell { number: current_line + 1, origin: ' ', text };
                        self.push_split_pair(Some(old), Some(new), left_width, right_width, vec![source_idx]);
                        current_line += 1;
                        current_line_old += 1;
                        idx += 1;
                    },
                    _ => {
                        idx += 1;
                    },
                }
            }
        }

        while current_line < original_lines.len() {
            let text = original_lines[current_line].clone();
            let source_idx = unified_idx;
            unified_idx += self.unified_unchanged_wrap_count(&text);
            let old = SplitCell { number: current_line_old + 1, origin: ' ', text: text.clone() };
            let new = SplitCell { number: current_line + 1, origin: ' ', text };
            self.push_split_pair(Some(old), Some(new), left_width, right_width, vec![source_idx]);
            current_line += 1;
            current_line_old += 1;
        }
    }

    fn split_pane_text_widths(&self) -> (usize, usize) {
        let left_width = self.layout.viewer_split_left.width as usize;
        let right_width = self.layout.viewer_split_right.width as usize;
        if left_width > 0 && right_width > 0 {
            return (left_width.saturating_sub(8).max(1), right_width.saturating_sub(8).max(1));
        }

        let total_width = (self.layout.graph.width as usize).saturating_sub(1);
        if total_width <= 1 {
            return (1, 1);
        }

        let left_weight = self.layout_config.weight_viewer_split_left.max(1) as usize;
        let right_weight = self.layout_config.weight_viewer_split_right.max(1) as usize;
        let total_weight = left_weight + right_weight;
        let left_pane_width = ((total_width * left_weight) / total_weight).max(1).min(total_width.saturating_sub(1));
        let right_pane_width = total_width.saturating_sub(left_pane_width).max(1);

        (left_pane_width.saturating_sub(8).max(1), right_pane_width.saturating_sub(8).max(1))
    }

    fn unified_unchanged_wrap_count(&self, text: &str) -> usize {
        wrap_words(text.to_string(), (self.layout.graph.width as usize).saturating_sub(8)).len().max(1)
    }

    fn unified_changed_wrap_count(&self, origin: char, text: &str) -> usize {
        wrap_words(format!("{}{}", Self::split_prefix(origin), text), (self.layout.graph.width as usize).saturating_sub(9)).len().max(1)
    }

    fn push_split_pair(&mut self, left: Option<SplitCell>, right: Option<SplitCell>, left_width: usize, right_width: usize, unified_indices: Vec<usize>) {
        let left_wrapped = left.as_ref().map(|cell| wrap_words(format!("{}{}", Self::split_prefix(cell.origin), cell.text), left_width)).unwrap_or_else(|| vec![String::new()]);
        let right_wrapped = right.as_ref().map(|cell| wrap_words(format!("{}{}", Self::split_prefix(cell.origin), cell.text), right_width)).unwrap_or_else(|| vec![String::new()]);

        let rows = left_wrapped.len().max(right_wrapped.len()).max(1);
        for idx in 0..rows {
            let left_cell = if idx < left_wrapped.len() { left.as_ref() } else { None };
            let right_cell = if idx < right_wrapped.len() { right.as_ref() } else { None };
            let left_text = left_wrapped.get(idx).map(String::as_str).unwrap_or("");
            let right_text = right_wrapped.get(idx).map(String::as_str).unwrap_or("");

            self.viewer_split_rows.push(SplitViewerRow {
                left: self.split_list_item(left_cell, left_text, idx == 0),
                right: self.split_list_item(right_cell, right_text, idx == 0),
                unified_indices: unified_indices.clone(),
            });
        }
    }

    fn split_prefix(origin: char) -> &'static str {
        match origin {
            '-' => "- ",
            '+' => "+ ",
            '!' => "! ",
            _ => "",
        }
    }

    fn split_list_item(&self, cell: Option<&SplitCell>, text: &str, show_number: bool) -> ListItem<'static> {
        let origin = cell.map(|cell| cell.origin).unwrap_or(' ');
        let item_style = match origin {
            '-' => Style::default().bg(self.theme.COLOR_DARK_RED).fg(self.theme.COLOR_RED),
            '+' => Style::default().bg(self.theme.COLOR_LIGHT_GREEN_900).fg(self.theme.COLOR_GREEN),
            '!' => Style::default().fg(self.theme.COLOR_ORANGE),
            _ => Style::default(),
        };
        let number_fg = match origin {
            '-' => self.theme.COLOR_RED,
            '+' => self.theme.COLOR_GREEN,
            '!' => self.theme.COLOR_ORANGE,
            _ => self.theme.COLOR_BORDER,
        };
        let text_fg = match origin {
            '-' => self.theme.COLOR_RED,
            '+' => self.theme.COLOR_GREEN,
            '!' => self.theme.COLOR_ORANGE,
            _ => self.theme.COLOR_GREY_500,
        };
        let number = if show_number { cell.map(|cell| format!("{:3}  ", cell.number)).unwrap_or_else(|| "     ".to_string()) } else { "     ".to_string() };

        ListItem::new(Line::from(vec![Span::styled(number, Style::default().fg(number_fg)), Span::styled(text.to_string(), Style::default().fg(text_fg))])).style(item_style)
    }
}
