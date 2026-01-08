use crate::app::app::{App, Viewport};
use ratatui::Frame;
use ratatui::layout::Rect;
use serde_json::json;
use std::cell::Cell;
use std::fs;

#[derive(Default)]
pub struct Layout {
    pub title_left: Rect,
    pub title_right: Rect,
    pub app: Rect,
    pub branches: Rect,
    pub branches_scrollbar: Rect,
    pub tags: Rect,
    pub tags_scrollbar: Rect,
    pub stashes: Rect,
    pub stashes_scrollbar: Rect,
    pub graph: Rect,
    pub graph_scrollbar: Rect,
    pub inspector: Rect,
    pub inspector_scrollbar: Rect,
    pub status_top: Rect,
    pub status_top_scrollbar: Rect,
    pub status_bottom: Rect,
    pub status_bottom_scrollbar: Rect,
    pub statusbar_left: Rect,
    pub statusbar_right: Rect,
}

impl App {
    pub fn layout(&mut self, frame: &mut Frame) {
        let is_settings = self.viewport == Viewport::Splash || self.viewport == Viewport::Settings;
        let is_inspector = !is_settings && self.is_inspector && self.graph_selected != 0;
        let is_status = !is_settings && self.is_status;
        let is_right_pane = is_inspector || is_status;

        let chunks_vertical = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(if self.is_minimal { 0 } else { 1 }),
                ratatui::layout::Constraint::Percentage(100),
                ratatui::layout::Constraint::Length(if self.is_minimal { 0 } else { 1 }),
            ])
            .split(frame.area());

        let chunks_title_bar = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(80),
                ratatui::layout::Constraint::Percentage(20),
            ])
            .split(chunks_vertical[0]);

        let chunks_horizontal = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Length(
                    if (self.is_branches || self.is_tags || self.is_stashes) && !is_settings {
                        45
                    } else {
                        0
                    },
                ),
                ratatui::layout::Constraint::Max(500),
                ratatui::layout::Constraint::Length(if is_right_pane { 46 } else { 0 }),
            ])
            .split(chunks_vertical[1]);

        let chunks_pane_left = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Percentage(if self.is_branches {
                    if !self.is_tags && !self.is_stashes {
                        100
                    } else if self.is_tags && self.is_stashes {
                        33
                    } else {
                        50
                    }
                } else {
                    0
                }),
                ratatui::layout::Constraint::Percentage(if self.is_tags {
                    if !self.is_branches && !self.is_stashes {
                        100
                    } else if self.is_branches && self.is_stashes {
                        33
                    } else {
                        50
                    }
                } else {
                    0
                }),
                ratatui::layout::Constraint::Percentage(if self.is_stashes {
                    if !self.is_branches && !self.is_tags {
                        100
                    } else if self.is_branches && self.is_tags {
                        33
                    } else {
                        50
                    }
                } else {
                    0
                }),
            ])
            .split(chunks_horizontal[0]);

        let chunks_pane_right = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Percentage(if is_inspector {
                    if !is_status { 100 } else { 50 }
                } else {
                    0
                }),
                ratatui::layout::Constraint::Percentage(if is_status {
                    if !is_inspector { 100 } else { 50 }
                } else {
                    0
                }),
            ])
            .split(chunks_horizontal[2]);

        let chunks_status = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Percentage(if self.graph_selected == 0 {
                    50
                } else {
                    100
                }),
                ratatui::layout::Constraint::Percentage(if self.graph_selected == 0 {
                    50
                } else {
                    0
                }),
            ])
            .split(chunks_pane_right[1]);

        let chunks_status_bar = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(80),
                ratatui::layout::Constraint::Percentage(20),
            ])
            .split(chunks_vertical[2]);

        // Branches
        let mut branches_scrollbar = chunks_pane_left[0];
        branches_scrollbar.width += 1;
        let mut branches = chunks_pane_left[0];
        branches.y += 1;

        // Tags
        let mut tags_scrollbar = chunks_pane_left[1];
        tags_scrollbar.width += 1;
        let mut tags = chunks_pane_left[1];
        if !self.is_branches {
            tags.y += 1;
        } else {
            tags_scrollbar.height += 1;
            tags_scrollbar.y -= 1;
        }

        // Stashes
        let mut stashes_scrollbar = chunks_pane_left[2];
        stashes_scrollbar.width += 1;
        let mut stashes = chunks_pane_left[2];
        if !self.is_branches && !self.is_tags {
            stashes.y += 1;
        } else {
            stashes_scrollbar.height += 1;
            stashes_scrollbar.y -= 1;
        }

        // Graph
        let graph_scrollbar = chunks_horizontal[1];
        let mut graph = chunks_horizontal[1];
        graph.y += 1;
        graph.height = graph.height.saturating_sub(2);

        // Inspector
        let inspector_scrollbar = chunks_pane_right[0];
        let mut inspector = chunks_pane_right[0];
        inspector.y += 1;

        // Status top
        let mut status_top_scrollbar = chunks_status[0];
        if self.is_inspector && self.graph_selected != 0 {
            status_top_scrollbar.y -= 1;
            status_top_scrollbar.height += 1;
        }
        let mut status_top = chunks_status[0];
        status_top.y = if self.is_inspector && self.graph_selected != 0 {
            status_top.y - 1
        } else {
            status_top.y + 1
        };
        status_top.height = if self.is_inspector && self.graph_selected != 0 {
            status_top.height + 1
        } else {
            status_top.height
        };
        status_top.width = status_top.width.saturating_sub(1);

        // Status bottom
        let mut status_bottom_scrollbar = chunks_status[1];
        status_bottom_scrollbar.y = status_bottom_scrollbar.y.saturating_sub(1);
        status_bottom_scrollbar.height += 1;
        let mut status_bottom = chunks_status[1];
        status_bottom.y = status_bottom.y.saturating_sub(1);
        status_bottom.height += 1;
        status_bottom.width = status_bottom.width.saturating_sub(1);

        self.layout = Layout {
            title_left: chunks_title_bar[0],
            title_right: chunks_title_bar[1],
            app: chunks_vertical[1],
            branches,
            branches_scrollbar,
            tags,
            tags_scrollbar,
            stashes,
            stashes_scrollbar,
            graph,
            graph_scrollbar,
            inspector,
            inspector_scrollbar,
            status_top,
            status_top_scrollbar,
            status_bottom,
            status_bottom_scrollbar,
            statusbar_left: chunks_status_bar[0],
            statusbar_right: chunks_status_bar[1],
        }
    }

    pub fn trap_selection(
        &self,
        selected: usize,
        scroll: &Cell<usize>,
        total_lines: usize,
        visible_height: usize,
    ) {
        if visible_height == 0 || total_lines == 0 {
            scroll.set(0);
            return;
        }

        // Max scroll offset so that a full page fits (if total_lines < visible_height, max_scroll = 0)
        let max_scroll = total_lines.saturating_sub(visible_height);

        // Get current scroll and clamp it to max_scroll
        let mut scroll_val = scroll.get().min(max_scroll);
        let sel = selected.min(total_lines.saturating_sub(1));

        // If selection is above the viewport -> jump scroll up
        if sel < scroll_val {
            scroll_val = sel;
            scroll.set(scroll_val);
            return;
        }

        // If selection is below the viewport -> jump scroll down so selection is the last visible line
        if sel >= scroll_val + visible_height {
            let desired = sel.saturating_sub(visible_height).saturating_add(1);
            scroll_val = desired.min(max_scroll);
            scroll.set(scroll_val);
            return;
        }

        // Otherwise selection is already visible; ensure scroll is clamped
        scroll.set(scroll_val);
    }

    pub fn save_layout(&self) {
        let mut pathbuf = dirs::config_dir().unwrap();
        pathbuf.push("guitar");
        pathbuf.push("layout.json");
        let path = pathbuf.as_path();

        let value = json!({
            "is_shas": self.is_shas,
            "is_minimal": self.is_minimal,
            "is_branches": self.is_branches,
            "is_tags": self.is_tags,
            "is_stashes": self.is_stashes,
            "is_status": self.is_status,
            "is_inspector": self.is_inspector,
        });

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let _ = fs::write(path, serde_json::to_string_pretty(&value).unwrap());
    }

    pub fn load_layout(&mut self) {
        let mut pathbuf = dirs::config_dir().unwrap();
        pathbuf.push("guitar");
        pathbuf.push("layout.json");
        let path = pathbuf.as_path();

        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => {
                self.save_layout();
                return;
            }
        };

        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                self.save_layout();
                return;
            }
        };

        if let Some(v) = value.get("is_shas").and_then(|v| v.as_bool()) {
            self.is_shas = v;
        }
        if let Some(v) = value.get("is_minimal").and_then(|v| v.as_bool()) {
            self.is_minimal = v;
        }
        if let Some(v) = value.get("is_branches").and_then(|v| v.as_bool()) {
            self.is_branches = v;
        }
        if let Some(v) = value.get("is_tags").and_then(|v| v.as_bool()) {
            self.is_tags = v;
        }
        if let Some(v) = value.get("is_stashes").and_then(|v| v.as_bool()) {
            self.is_stashes = v;
        }
        if let Some(v) = value.get("is_status").and_then(|v| v.as_bool()) {
            self.is_status = v;
        }
        if let Some(v) = value.get("is_inspector").and_then(|v| v.as_bool()) {
            self.is_inspector = v;
        }
    }
}
