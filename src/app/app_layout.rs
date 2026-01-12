use crate::app::app::{App, Focus, Viewport};
use crate::helpers::layout::{
    LAYOUT_PERCENTAGE_CENTER_PANE_CRAMPED, LAYOUT_PERCENTAGE_LEFT_PANE_CRAMPED, LAYOUT_PERCENTAGE_RIGHT_PANE_CRAMPED, LAYOUT_WIDTH_LEFT_PANE, LAYOUT_WIDTH_MIN_CENTER, LAYOUT_WIDTH_RIGHT_PANE,
    add_scrollbar, extend_up, inset_bottom, inset_top, load_layout_config, save_layout_config, shrink_width,
};
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::{Direction, Layout as RatatuiLayout, Rect};
use std::cell::Cell;

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
        let is_zen = self.layout_config.is_zen;
        let is_settings = self.viewport == Viewport::Splash || self.viewport == Viewport::Settings;
        let is_inspector = !is_settings && self.layout_config.is_inspector && self.graph_selected != 0;
        let is_status = !is_settings && self.layout_config.is_status;
        let is_right_pane = is_inspector || is_status;
        let is_left_pane = (self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes) && !is_settings;

        // Process the layout chunks

        // Main separation of the layout into vertical chunks
        let chunks_vertical = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.layout_config.is_minimal { 0 } else { 1 }), // Title bar
                Constraint::Min(0),                                                    // Main app area
                Constraint::Length(if self.layout_config.is_minimal { 0 } else { 1 }), // Status bar
            ])
            .split(frame.area());

        // Title bar
        let chunks_title_bar = RatatuiLayout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(chunks_vertical[0]);

        // Separate the main viewport into vertical panes
        let total_width = chunks_vertical[1].width;
        let width_left_pane = if is_left_pane { LAYOUT_WIDTH_LEFT_PANE } else { 0 };
        let width_right_pane = if is_right_pane { LAYOUT_WIDTH_RIGHT_PANE } else { 0 };
        let min_required = width_left_pane + width_right_pane + LAYOUT_WIDTH_MIN_CENTER;
        let constraints = if total_width < min_required {
            let percentage_left_pane = if is_left_pane && is_right_pane {
                LAYOUT_PERCENTAGE_LEFT_PANE_CRAMPED
            } else if is_left_pane && !is_right_pane {
                50
            } else {
                0
            };
            let percentage_center_pane = if is_left_pane && is_right_pane {
                LAYOUT_PERCENTAGE_CENTER_PANE_CRAMPED
            } else if !is_left_pane && !is_right_pane {
                0
            } else {
                50
            };
            let percentage_right_pane = if is_right_pane && is_left_pane {
                LAYOUT_PERCENTAGE_RIGHT_PANE_CRAMPED
            } else if is_right_pane && !is_left_pane {
                50
            } else {
                0
            };
            [
                Constraint::Percentage(percentage_left_pane),   // Left pane
                Constraint::Percentage(percentage_center_pane), // Center pane
                Constraint::Percentage(percentage_right_pane),  // Right pane
            ]
        } else {
            [
                Constraint::Length(width_left_pane),  // Left pane
                Constraint::Min(0),                   // Center pane
                Constraint::Length(width_right_pane), // Right pane
            ]
        };
        let chunks_horizontal = RatatuiLayout::default().direction(Direction::Horizontal).constraints(constraints).split(chunks_vertical[1]);

        // Left pane sections
        let left_sections = [self.layout_config.is_branches, self.layout_config.is_tags, self.layout_config.is_stashes];
        let active_count = left_sections.iter().filter(|&&v| v).count().max(1);
        let pct = 100 / active_count as u16;
        let chunks_pane_left =
            RatatuiLayout::default().direction(Direction::Vertical).constraints(left_sections.map(|active| Constraint::Percentage(if active { pct } else { 0 }))).split(chunks_horizontal[0]);

        // Right pane sections
        let right_sections = [is_inspector, is_status];
        let active_count = right_sections.iter().filter(|&&v| v).count().max(1);
        let pct = 100 / active_count as u16;
        let chunks_pane_right =
            RatatuiLayout::default().direction(Direction::Vertical).constraints(right_sections.map(|active| Constraint::Percentage(if active { pct } else { 0 }))).split(chunks_horizontal[2]);

        // Status pane subdivisions into top and bottom status sections
        let chunks_status = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(if self.graph_selected == 0 { 50 } else { 100 }), Constraint::Percentage(if self.graph_selected == 0 { 50 } else { 0 })])
            .split(chunks_pane_right[1]);

        // Status bar subdivisions into left and right sections
        let chunks_status_bar = RatatuiLayout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(chunks_vertical[2]);

        // Process the layout rectangles

        // Pane, branches
        let branches_scrollbar = add_scrollbar(chunks_pane_left[0]);
        let branches = inset_top(chunks_pane_left[0], 1);

        // Pane, tags
        let mut tags_scrollbar = add_scrollbar(chunks_pane_left[1]);
        let mut tags = chunks_pane_left[1];
        if !self.layout_config.is_branches {
            tags = inset_top(tags, 1);
        } else {
            tags_scrollbar = extend_up(tags_scrollbar, 1);
        }

        // Pane, stashes
        let mut stashes_scrollbar = add_scrollbar(chunks_pane_left[2]);
        let mut stashes = chunks_pane_left[2];
        if !self.layout_config.is_branches && !self.layout_config.is_tags {
            stashes = inset_top(stashes, 1);
        } else {
            stashes_scrollbar = extend_up(stashes_scrollbar, 1);
        }

        // Pane, graph
        let graph_scrollbar = chunks_horizontal[1];
        let graph = inset_bottom(inset_top(chunks_horizontal[1], 1), 1);

        // Pane, inspector
        let inspector_scrollbar = chunks_pane_right[0];
        let inspector = inset_top(chunks_pane_right[0], 1);

        // Pane, status top
        let mut status_top_scrollbar = chunks_status[0];
        let mut status_top = chunks_status[0];
        let merge_with_inspector = self.layout_config.is_inspector && self.graph_selected != 0;
        if merge_with_inspector {
            status_top_scrollbar = extend_up(status_top_scrollbar, 1);
            status_top = extend_up(status_top, 1);
        } else {
            status_top = inset_top(status_top, 1);
        }
        status_top = shrink_width(status_top, 1);

        // Pane, status bottom
        let status_bottom_scrollbar = extend_up(chunks_status[1], 1);
        let mut status_bottom = extend_up(chunks_status[1], 1);
        status_bottom = shrink_width(status_bottom, 1);

        if is_zen {
            let zen = chunks_vertical[1];

            let zero = Rect { x: 0, y: 0, width: 0, height: 0 };

            self.layout = Layout {
                // Keep title & status bar if you want
                title_left: chunks_title_bar[0],
                title_right: chunks_title_bar[1],
                app: zen,

                // Only the focused pane gets space
                branches: if matches!(self.focus, Focus::Branches) { zen } else { zero },
                tags: if matches!(self.focus, Focus::Tags) { zen } else { zero },
                stashes: if matches!(self.focus, Focus::Stashes) { zen } else { zero },
                graph: if matches!(
                    self.focus,
                    Focus::Viewport
                        | Focus::ModalCheckout
                        | Focus::ModalSolo
                        | Focus::ModalCommit
                        | Focus::ModalCreateBranch
                        | Focus::ModalDeleteBranch
                        | Focus::ModalGrep
                        | Focus::ModalTag
                        | Focus::ModalDeleteTag
                ) {
                    zen
                } else {
                    zero
                },
                inspector: if matches!(self.focus, Focus::Inspector) { zen } else { zero },
                status_top: if matches!(self.focus, Focus::StatusTop) { zen } else { zero },
                status_bottom: if matches!(self.focus, Focus::StatusBottom) { zen } else { zero },

                // Kill all scrollbars in zen
                branches_scrollbar: zero,
                tags_scrollbar: zero,
                stashes_scrollbar: zero,
                graph_scrollbar: zero,
                inspector_scrollbar: zero,
                status_top_scrollbar: zero,
                status_bottom_scrollbar: zero,

                // Keep status bar if you want
                statusbar_left: chunks_status_bar[0],
                statusbar_right: chunks_status_bar[1],
            };

            return;
        }

        self.layout = Layout {
            // Title bar
            title_left: chunks_title_bar[0],
            title_right: chunks_title_bar[1],

            // Main app area
            app: chunks_vertical[1],

            // Panes
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

            // Status bar
            statusbar_left: chunks_status_bar[0],
            statusbar_right: chunks_status_bar[1],
        }
    }

    pub fn trap_selection(&self, selected: usize, scroll: &Cell<usize>, total_lines: usize, visible_height: usize) {
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
        save_layout_config(&self.layout_config);
    }

    pub fn load_layout(&mut self) {
        self.layout_config = load_layout_config();
    }
}
