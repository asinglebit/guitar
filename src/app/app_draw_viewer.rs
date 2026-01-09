use crate::{
    app::app::{App, Focus, Viewport},
    git::queries::diffs::{get_file_at_oid, get_file_at_workdir, get_file_diff_at_oid, get_file_diff_at_workdir},
    helpers::text::wrap_words,
};
use git2::Oid;
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

impl App {
    pub fn draw_viewer(&mut self, frame: &mut Frame) {
        // Padding
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        // Calculate maximum available width for text
        // let available_width = self.layout.graph.width as usize - 1;
        // let max_text_width = available_width.saturating_sub(2);

        // Get vertical dimensions
        let total_lines = self.viewer_lines.len();
        let visible_height = self.layout.graph.height as usize;

        // Clamp selection
        if total_lines == 0 {
            self.viewer_selected = 0;
        } else if self.viewer_selected >= total_lines {
            self.viewer_selected = total_lines - 1;
        }

        // Trap selection
        self.trap_selection(self.viewer_selected, &self.viewer_scroll, total_lines, visible_height);

        // Calculate scroll
        let start = self.viewer_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Setup list items
        let list_items: Vec<ListItem> = self.viewer_lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let absolute_idx = start + i;
                let mut item = line.clone();
                if absolute_idx == self.viewer_selected && self.focus == Focus::Viewport {
                    item = item.style(Style::default().bg(self.theme.COLOR_GREY_800));
                }
                item
            })
            .collect();

        // Setup the list
        let list = List::new(list_items).block(
            Block::default().padding(padding).borders(Borders::RIGHT | Borders::LEFT).border_style(Style::default().fg(self.theme.COLOR_BORDER)).border_type(ratatui::widgets::BorderType::Rounded),
        );

        // Render the list
        frame.render_widget(list, self.layout.graph);

        // Setup the scrollbar
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height)).position(self.viewer_scroll.get());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(if self.layout_config.is_inspector || self.layout_config.is_status { Some("─") } else { Some("╮") })
            .end_symbol(if self.layout_config.is_inspector || self.layout_config.is_status { Some("─") } else { Some("╯") })
            .track_symbol(Some("│"))
            .thumb_symbol("▌")
            .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        // Render the scrollbar
        frame.render_stateful_widget(scrollbar, self.layout.graph_scrollbar, &mut scrollbar_state);
    }

    pub fn open_viewer(&mut self) {
        match self.focus {
            Focus::StatusTop => {
                // If a commit is selected in the top graph view
                if self.graph_selected != 0 && !self.current_diff.is_empty() {
                    // Set the file_name to the currently selected file in the diff
                    self.file_name = Some(self.current_diff.get(self.status_top_selected).unwrap().filename.to_string());

                    // Update the viewer to show the file at the selected commit OID
                    let oid = self.oids.get_oid_by_idx(self.graph_selected);
                    self.update_viewer(*oid);
                    self.viewport = Viewport::Viewer;
                } else if self.graph_selected == 0 && self.uncommitted.is_staged {
                    // If HEAD is selected and staged uncommitted changes exist
                    let modified_len = self.uncommitted.staged.modified.len();
                    let added_len = self.uncommitted.staged.added.len();
                    let index = self.status_top_selected;

                    // Select the file name from staged changes depending on index
                    self.file_name = if index < modified_len {
                        self.uncommitted.staged.modified.get(index).cloned()
                    } else if index < modified_len + added_len {
                        self.uncommitted.staged.added.get(index - modified_len).cloned()
                    } else {
                        self.uncommitted.staged.deleted.get(index - modified_len - added_len).cloned()
                    };

                    // Update viewer for uncommitted file (Oid::zero indicates workdir)
                    self.update_viewer(Oid::zero());
                    self.viewport = Viewport::Viewer;
                }
            },
            Focus::StatusBottom => {
                // If uncommitted unstaged changes exist in bottom status view
                if self.graph_selected == 0 && self.uncommitted.is_unstaged {
                    let modified_len = self.uncommitted.unstaged.modified.len();
                    let added_len = self.uncommitted.unstaged.added.len();
                    let index = self.status_bottom_selected;

                    // Select the file name from unstaged changes depending on index
                    self.file_name = if index < modified_len {
                        self.uncommitted.unstaged.modified.get(index).cloned()
                    } else if index < modified_len + added_len {
                        self.uncommitted.unstaged.added.get(index - modified_len).cloned()
                    } else {
                        self.uncommitted.unstaged.deleted.get(index - modified_len - added_len).cloned()
                    };

                    // Update viewer for uncommitted file
                    self.update_viewer(Oid::zero());
                    self.viewport = Viewport::Viewer;
                }
            },
            _ => {},
        }
    }

    pub fn update_viewer(&mut self, oid: Oid) {
        // Clone the current file name
        let filename = self.file_name.clone().unwrap();

        // Decide whether to use committed version or uncommitted (workdir)
        let (original_lines, hunks) = if oid == Oid::zero() {
            (
                get_file_at_workdir(&self.repo, &filename),                          // get current file in workdir
                get_file_diff_at_workdir(&self.repo, &filename).unwrap_or_default(), // get diff for workdir
            )
        } else {
            (
                get_file_at_oid(&self.repo, oid, &filename),                          // get file at commit
                get_file_diff_at_oid(&self.repo, oid, &filename).unwrap_or_default(), // get diff for commit
            )
        };

        self.viewer_lines.clear(); // Clear current viewer lines
        self.viewer_hunks.clear(); // Clear current viewer hunks
        let mut current_line: usize = 0; // Current line in new file
        let mut current_line_old: usize = 0; // Current line in old file

        // Check last origin to detect change in hunks
        let mut last_origin: Option<char> = None;

        for hunk in hunks.iter() {
            // Proceed with hunk processing
            let header = &hunk.header;
            let old_start_idx: usize = header.old_start.saturating_sub(1) as usize; // Convert to 0-based index

            // Add unchanged lines before this hunk
            while current_line < old_start_idx && current_line < original_lines.len() {
                // Wrap line to fit viewport width
                let wrapped = wrap_words(original_lines[current_line].clone(), (self.layout.graph.width as usize).saturating_sub(8));
                for (idx, line) in wrapped.into_iter().enumerate() {
                    // Push each wrapped line into viewer with line numbers
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

            // Process lines in the hunk
            for line in hunk.lines.iter().filter(|l| l.origin != 'H') {
                // Remove trailing newline
                let text = line.content.trim_end_matches('\n');

                // Detect transition: push hunk navigation if origin changed
                if let Some(prev) = last_origin
                    && prev != line.origin
                {
                    self.viewer_hunks.push(self.viewer_lines.len());
                }
                last_origin = Some(line.origin);

                // Determine styling, prefix, color, and line number based on line origin
                let (style, prefix, side, fg, count) = match line.origin {
                    '-' => (Style::default().bg(self.theme.COLOR_DARK_RED).fg(self.theme.COLOR_RED), "- ".to_string(), self.theme.COLOR_RED, self.theme.COLOR_RED, current_line_old + 1),
                    '+' => (Style::default().bg(self.theme.COLOR_LIGHT_GREEN_900).fg(self.theme.COLOR_GREEN), "+ ".to_string(), self.theme.COLOR_GREEN, self.theme.COLOR_GREEN, current_line + 1),
                    ' ' => (Style::default(), "".to_string(), self.theme.COLOR_BORDER, self.theme.COLOR_GREY_500, current_line + 1),
                    _ => (Style::default(), "".to_string(), self.theme.COLOR_BORDER, self.theme.COLOR_GREY_500, 0),
                };

                // Wrap the line to viewport width
                let wrapped = wrap_words(format!("{}{}", prefix, text), (self.layout.graph.width as usize).saturating_sub(9));
                for (idx, line) in wrapped.into_iter().enumerate() {
                    // Push each wrapped line into the viewer
                    self.viewer_lines.push(
                        ListItem::new(Line::from(vec![
                            Span::styled((if idx == 0 { format!("{:3}  ", count) } else { "     ".to_string() }).to_string(), Style::default().fg(side)),
                            Span::styled(line.to_string(), Style::default().fg(fg)),
                        ]))
                        .style(style),
                    );
                }

                // Update line counters depending on origin
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

        // Add remaining lines after the last hunk (if any)
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

        self.viewer_selected = self.viewer_hunks.first().copied().unwrap_or(0);
    }
}
