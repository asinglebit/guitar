use crate::{
    app::app::{App, Direction, Focus, LayoutDrag, MouseDrag, MouseSelectionTarget, ScrollbarDrag, ScrollbarTarget, SettingsSelectionKind, SharedMouseDrag, Viewport},
    helpers::{
        layout::{LAYOUT_HEIGHT_MIN_STACKED_PANE, LAYOUT_WIDTH_MIN_CENTER, LAYOUT_WIDTH_MIN_SIDE_PANE, scrollbar_content_length},
        text::{empty_state_top_padding, sanitize, wrap_words},
    },
};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};
use std::{
    io,
    time::{Duration, Instant},
};

const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);
const MODAL_ESC_TITLE_WIDTH: u16 = 7;

#[derive(Clone, Copy)]
enum GraphPaneClickKind {
    Branches,
    Tags,
    Stashes,
    Reflogs,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeDirection {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StackPane {
    Branches,
    Tags,
    Stashes,
    Reflogs,
    Worktrees,
    Submodules,
    Search,
    Inspector,
    Status,
    StatusTop,
    StatusBottom,
}

#[derive(Clone, Copy)]
struct ScrollbarInfo {
    target: ScrollbarTarget,
    rect: Rect,
    total_lines: usize,
    visible_height: usize,
    scroll: usize,
    max_scroll: usize,
}

#[derive(Clone, Copy)]
struct ScrollbarMetrics {
    track_top: u16,
    track_len: usize,
    thumb_start: usize,
    thumb_len: usize,
    max_viewport_position: usize,
}

impl App {
    pub fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if matches!(key_event.kind, KeyEventKind::Press) => {
                self.handle_key_event(key_event);
            },
            Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
            },
            _ => {},
        };
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Right) => {
                self.open_context_menu(mouse_event.column, mouse_event.row);
            },
            MouseEventKind::Down(MouseButton::Left) => {
                if self.handle_modal_left_click(mouse_event.column, mouse_event.row) {
                    return;
                }
                if !self.handle_context_menu_left_click(mouse_event.column, mouse_event.row) {
                    self.handle_mouse_down(mouse_event.column, mouse_event.row);
                }
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.context_menu.is_none() {
                    self.handle_mouse_drag(mouse_event.column, mouse_event.row);
                }
            },
            MouseEventKind::Up(MouseButton::Left) => {
                if self.context_menu.is_none() {
                    self.finish_mouse_drag();
                }
            },
            MouseEventKind::ScrollUp => {
                if self.context_menu.is_none() {
                    self.handle_mouse_scroll(mouse_event.column, mouse_event.row, Direction::Up);
                }
            },
            MouseEventKind::ScrollDown => {
                if self.context_menu.is_none() {
                    self.handle_mouse_scroll(mouse_event.column, mouse_event.row, Direction::Down);
                }
            },
            _ => {},
        }
    }

    fn handle_modal_left_click(&mut self, column: u16, row: u16) -> bool {
        if !self.is_modal_focus() {
            return false;
        }

        self.mouse_drag = None;
        self.last_mouse_click = None;

        if self.modal_escape_hitbox_contains(column, row) {
            self.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        }

        true
    }

    fn modal_escape_hitbox_contains(&self, column: u16, row: u16) -> bool {
        let Some(area) = self.modal_area else {
            return false;
        };
        if row != area.y || area.width <= 2 {
            return false;
        }

        let left = area.x.saturating_add(area.width.saturating_sub(MODAL_ESC_TITLE_WIDTH.saturating_add(1)));
        let right = area.x.saturating_add(area.width.saturating_sub(1));
        column >= left && column < right
    }

    fn handle_mouse_down(&mut self, column: u16, row: u16) {
        let scrollbar_drag = self.scrollbar_drag_at(column, row);
        let layout_drag = self.layout_drag_at(column, row);

        match (scrollbar_drag, layout_drag) {
            (Some(scrollbar), Some(layout)) => {
                self.mouse_drag = Some(MouseDrag::Shared(SharedMouseDrag { layout, scrollbar, start_column: column, start_row: row }));
                self.last_mouse_click = None;
                return;
            },
            (Some(scrollbar), None) => {
                self.apply_scrollbar_drag(scrollbar, row);
                self.mouse_drag = Some(MouseDrag::Scrollbar(scrollbar));
                self.last_mouse_click = None;
                return;
            },
            (None, Some(layout)) => {
                self.mouse_drag = Some(MouseDrag::Layout(layout));
                self.last_mouse_click = None;
                return;
            },
            (None, None) => {},
        }

        self.mouse_drag = None;

        let Some(target) = self.mouse_selection_target_at(column, row) else {
            self.last_mouse_click = None;
            return;
        };

        let now = Instant::now();
        let is_double_click = self.last_mouse_click.is_some_and(|(previous, at)| previous == target && now.duration_since(at) <= DOUBLE_CLICK_THRESHOLD);
        let is_single_click_activation = self.mouse_target_activates_on_single_click(target);

        self.select_mouse_target(target);

        if is_single_click_activation {
            if is_double_click {
                self.last_mouse_click = None;
            } else {
                self.on_select();
                self.last_mouse_click = Some((target, now));
            }
        } else if is_double_click {
            self.last_mouse_click = None;
            if Self::mouse_target_activates_on_double_click(target) {
                self.on_select();
            }
        } else {
            self.last_mouse_click = Some((target, now));
        }
    }

    fn handle_mouse_drag(&mut self, column: u16, row: u16) {
        match self.mouse_drag {
            Some(MouseDrag::Layout(drag)) => {
                self.apply_layout_drag(drag, column, row);
            },
            Some(MouseDrag::Scrollbar(drag)) => {
                self.apply_scrollbar_drag(drag, row);
            },
            Some(MouseDrag::Shared(drag)) => {
                let dx = column.abs_diff(drag.start_column);
                let dy = row.abs_diff(drag.start_row);
                if dx == 0 && dy == 0 {
                    return;
                }

                if dx > dy {
                    self.mouse_drag = Some(MouseDrag::Layout(drag.layout));
                    self.apply_layout_drag(drag.layout, column, row);
                } else {
                    self.mouse_drag = Some(MouseDrag::Scrollbar(drag.scrollbar));
                    self.apply_scrollbar_drag(drag.scrollbar, row);
                }
            },
            None => {},
        }
    }

    fn finish_mouse_drag(&mut self) {
        let Some(drag) = self.mouse_drag.take() else {
            return;
        };

        match drag {
            MouseDrag::Layout(_) => {
                self.save_layout();
                self.mark_viewer_layout_dirty();
            },
            MouseDrag::Scrollbar(_) => {},
            MouseDrag::Shared(shared) => {
                self.apply_scrollbar_drag(shared.scrollbar, shared.start_row);
            },
        }
    }

    fn mouse_target_activates_on_single_click(&self, target: MouseSelectionTarget) -> bool {
        match target {
            MouseSelectionTarget::Settings(index) => self.settings_selections.iter().any(|selection| selection.line == index && matches!(selection.kind, SettingsSelectionKind::LayoutCommand(_))),
            _ => false,
        }
    }

    fn mouse_target_activates_on_double_click(target: MouseSelectionTarget) -> bool {
        matches!(
            target,
            MouseSelectionTarget::Branches(_)
                | MouseSelectionTarget::Tags(_)
                | MouseSelectionTarget::Stashes(_)
                | MouseSelectionTarget::Reflogs(_)
                | MouseSelectionTarget::Worktrees(_)
                | MouseSelectionTarget::Submodules(_)
                | MouseSelectionTarget::Search(_)
                | MouseSelectionTarget::StatusTop(_)
                | MouseSelectionTarget::StatusBottom(_)
                | MouseSelectionTarget::Settings(_)
        )
    }

    pub(crate) fn select_mouse_target(&mut self, target: MouseSelectionTarget) {
        match target {
            MouseSelectionTarget::Graph(index) => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Graph;
                self.select_graph_index(index);
            },
            MouseSelectionTarget::Viewer(index) => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Viewer;
                self.viewer_selected = index;
            },
            MouseSelectionTarget::Branches(index) => {
                self.focus = Focus::Branches;
                self.branches_selected = index;
            },
            MouseSelectionTarget::Tags(index) => {
                self.focus = Focus::Tags;
                self.tags_selected = index;
            },
            MouseSelectionTarget::Stashes(index) => {
                self.focus = Focus::Stashes;
                self.stashes_selected = index;
            },
            MouseSelectionTarget::Reflogs(index) => {
                self.focus = Focus::Reflogs;
                self.reflogs_selected = index;
            },
            MouseSelectionTarget::Worktrees(index) => {
                self.focus = Focus::Worktrees;
                self.worktrees_selected = index;
            },
            MouseSelectionTarget::Submodules(index) => {
                self.focus = Focus::Submodules;
                self.submodules_selected = index;
            },
            MouseSelectionTarget::Search(index) => {
                self.focus = Focus::Search;
                self.search_selected = index;
            },
            MouseSelectionTarget::Inspector(index) => {
                self.focus = Focus::Inspector;
                self.inspector_selected = index;
            },
            MouseSelectionTarget::StatusTop(index) => {
                self.focus = Focus::StatusTop;
                self.status_top_selected = index;
            },
            MouseSelectionTarget::StatusBottom(index) => {
                self.focus = Focus::StatusBottom;
                self.status_bottom_selected = index;
            },
            MouseSelectionTarget::Splash(index) => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Splash;
                self.splash_selected = index;
            },
            MouseSelectionTarget::Settings(index) => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Settings;
                self.settings_selected = index;
            },
            MouseSelectionTarget::SettingsTab(tab) => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Settings;
                self.switch_settings_tab(tab);
            },
        }
    }

    pub(crate) fn mouse_selection_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        if self.is_modal_focus() {
            return None;
        }

        match self.viewport {
            Viewport::Splash => return self.splash_mouse_target_at(column, row),
            Viewport::Settings => return self.settings_mouse_target_at(column, row),
            Viewport::Graph => {},
            Viewport::Viewer => {
                if let Some(target) = self.left_pane_mouse_target_at(column, row) {
                    return Some(target);
                }
                if let Some(target) = self.right_pane_mouse_target_at(column, row) {
                    return Some(target);
                }
                return self.viewer_mouse_target_at(column, row);
            },
        }

        if let Some(target) = self.left_pane_mouse_target_at(column, row) {
            return Some(target);
        }
        if let Some(target) = self.right_pane_mouse_target_at(column, row) {
            return Some(target);
        }
        self.graph_mouse_target_at(column, row)
    }

    fn graph_mouse_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };
        let row_offset = self.row_offset_in_content(self.layout.graph, column, row, visible_height, self.layout_config.is_zen)?;
        let index = self.graph_scroll.get().saturating_add(row_offset);
        if index >= self.graph_commit_count() {
            return None;
        }
        if self.graph_tx.is_some() && self.graph_row_at(index).is_none() {
            return None;
        }
        Some(MouseSelectionTarget::Graph(index))
    }

    fn viewer_mouse_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };
        let row_offset = self.row_offset_in_content(self.layout.graph, column, row, visible_height, self.layout_config.is_zen)?;
        let index = self.viewer_scroll.get().saturating_add(row_offset);
        (index < self.viewer_row_count()).then_some(MouseSelectionTarget::Viewer(index))
    }

    fn left_pane_mouse_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        if self.layout_config.is_branches {
            let visible_height = self.layout.branches.height.saturating_sub(2) as usize;
            if let Some(index) = self.scrolled_row_index(self.layout.branches, column, row, visible_height, self.layout_config.is_zen, self.branches_scroll.get(), self.branch_clickable_count()) {
                return self.graph_pane_clickable(index, GraphPaneClickKind::Branches).then_some(MouseSelectionTarget::Branches(index));
            }
        }

        if self.layout_config.is_tags {
            let visible_height = if self.layout_config.is_zen {
                self.layout.tags.height.saturating_sub(2) as usize
            } else {
                self.layout.tags.height.saturating_sub(if self.layout_config.is_branches { 1 } else { 2 }) as usize
            };
            if let Some(index) = self.scrolled_row_index(self.layout.tags, column, row, visible_height, self.layout_config.is_zen, self.tags_scroll.get(), self.tag_clickable_count()) {
                return self.graph_pane_clickable(index, GraphPaneClickKind::Tags).then_some(MouseSelectionTarget::Tags(index));
            }
        }

        if self.layout_config.is_stashes {
            let visible_height = if self.layout_config.is_zen {
                self.layout.stashes.height.saturating_sub(2) as usize
            } else {
                self.layout.stashes.height.saturating_sub(if self.layout_config.is_branches || self.layout_config.is_tags { 1 } else { 2 }) as usize
            };
            if let Some(index) = self.scrolled_row_index(self.layout.stashes, column, row, visible_height, self.layout_config.is_zen, self.stashes_scroll.get(), self.stash_clickable_count()) {
                return self.graph_pane_clickable(index, GraphPaneClickKind::Stashes).then_some(MouseSelectionTarget::Stashes(index));
            }
        }

        if self.layout_config.is_reflogs {
            let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes;
            let visible_height =
                if self.layout_config.is_zen { self.layout.reflogs.height.saturating_sub(2) as usize } else { self.layout.reflogs.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };
            if let Some(index) = self.scrolled_row_index(self.layout.reflogs, column, row, visible_height, self.layout_config.is_zen, self.reflogs_scroll.get(), self.reflog_clickable_count()) {
                return self.graph_pane_clickable(index, GraphPaneClickKind::Reflogs).then_some(MouseSelectionTarget::Reflogs(index));
            }
        }

        if self.layout_config.is_worktrees {
            let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs;
            let visible_height = if self.layout_config.is_zen {
                self.layout.worktrees.height.saturating_sub(2) as usize
            } else {
                self.layout.worktrees.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize
            };
            if let Some(index) = self.scrolled_row_index(self.layout.worktrees, column, row, visible_height, self.layout_config.is_zen, self.worktrees_scroll.get(), self.worktrees.entries.len()) {
                return Some(MouseSelectionTarget::Worktrees(index));
            }
        }

        if self.layout_config.is_submodules {
            let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees;
            let visible_height = if self.layout_config.is_zen {
                self.layout.submodules.height.saturating_sub(2) as usize
            } else {
                self.layout.submodules.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize
            };
            if let Some(index) = self.scrolled_row_index(self.layout.submodules, column, row, visible_height, self.layout_config.is_zen, self.submodules_scroll.get(), self.submodules.entries.len()) {
                return Some(MouseSelectionTarget::Submodules(index));
            }
        }

        if self.layout_config.is_search {
            let has_previous = self.layout_config.is_branches
                || self.layout_config.is_tags
                || self.layout_config.is_stashes
                || self.layout_config.is_reflogs
                || self.layout_config.is_worktrees
                || self.layout_config.is_submodules;
            let visible_height =
                if self.layout_config.is_zen { self.layout.search.height.saturating_sub(2) as usize } else { self.layout.search.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize };
            if let Some(index) = self.scrolled_row_index(self.layout.search, column, row, visible_height, self.layout_config.is_zen, self.search_scroll.get(), self.search_clickable_count()) {
                return Some(MouseSelectionTarget::Search(index));
            }
        }

        None
    }

    fn right_pane_mouse_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) {
            let visible_height = if self.layout_config.is_zen { self.layout.inspector.height.saturating_sub(2) as usize } else { self.layout.inspector.height.saturating_sub(1) as usize };
            if let Some(index) = self.scrolled_row_index(self.layout.inspector, column, row, visible_height, self.layout_config.is_zen, self.inspector_scroll.get(), usize::MAX)
                && self.inspector_clickable()
            {
                return Some(MouseSelectionTarget::Inspector(index));
            }
        }

        if self.layout_config.is_status {
            let top_count = self.status_top_clickable_count();
            let top_visible_height = self.layout.status_top.height.saturating_sub(2) as usize;
            let top_has_border = self.layout_config.is_zen || (self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts));
            if let Some(index) = self.scrolled_row_index(self.layout.status_top, column, row, top_visible_height, top_has_border, self.status_top_scroll.get(), top_count) {
                return Some(MouseSelectionTarget::StatusTop(index));
            }

            if self.graph_selected == 0 {
                let bottom_count = self.status_bottom_clickable_count();
                let bottom_visible_height = self.layout.status_bottom.height.saturating_sub(2) as usize;
                if let Some(index) = self.scrolled_row_index(self.layout.status_bottom, column, row, bottom_visible_height, true, self.status_bottom_scroll.get(), bottom_count) {
                    return Some(MouseSelectionTarget::StatusBottom(index));
                }
            }
        }

        None
    }

    fn settings_mouse_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };
        let row_offset = self.row_offset_in_content(self.layout.graph, column, row, visible_height, self.layout_config.is_zen)?;
        let index = self.settings_scroll.get().saturating_add(row_offset);

        if let Some(hitbox) = self.settings_tab_hitboxes.iter().find(|hitbox| hitbox.line == index && column >= hitbox.start && column < hitbox.end) {
            return Some(MouseSelectionTarget::SettingsTab(hitbox.tab));
        }

        self.settings_selections.iter().any(|selection| selection.line == index).then_some(MouseSelectionTarget::Settings(index))
    }

    fn splash_mouse_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
        if self.spinner.is_running() || self.recent.is_empty() || !Self::rect_contains(self.layout.app, column, row) {
            return None;
        }

        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(4) as usize } else { self.layout.graph.height.saturating_sub(2) as usize };
        let content_rows = 5usize.saturating_add(self.recent.len());
        let logo_rows: usize = if self.layout.app.width < 80 {
            1
        } else if self.layout.app.width < 120 {
            9
        } else {
            11
        };
        let splash_rows = logo_rows.saturating_add(content_rows);
        let dummies = visible_height.saturating_sub(splash_rows).saturating_div(2);
        let first_recent_row = dummies.saturating_add(logo_rows).saturating_add(5);
        let row_offset = row.saturating_sub(self.layout.app.y) as usize;

        if row_offset < first_recent_row {
            return None;
        }

        let index = row_offset - first_recent_row;
        (index < self.recent.len()).then_some(MouseSelectionTarget::Splash(index))
    }

    fn row_offset_in_content(&self, rect: Rect, column: u16, row: u16, visible_height: usize, has_top_border: bool) -> Option<usize> {
        if visible_height == 0 || !Self::rect_contains(rect, column, row) {
            return None;
        }
        let top = rect.y.saturating_add(u16::from(has_top_border));
        if row < top {
            return None;
        }
        let offset = row.saturating_sub(top) as usize;
        (offset < visible_height).then_some(offset)
    }

    fn scrolled_row_index(&self, rect: Rect, column: u16, row: u16, visible_height: usize, has_top_border: bool, scroll: usize, total: usize) -> Option<usize> {
        if total == 0 {
            return None;
        }
        let offset = self.row_offset_in_content(rect, column, row, visible_height, has_top_border)?;
        let index = scroll.saturating_add(offset);
        (index < total).then_some(index)
    }

    fn branch_clickable_count(&self) -> usize {
        self.graph.branches_window.as_ref().map(|window| window.total).unwrap_or_else(|| self.branches.sorted.len())
    }

    fn tag_clickable_count(&self) -> usize {
        self.graph.tags_window.as_ref().map(|window| window.total).unwrap_or_else(|| self.tags.sorted.len())
    }

    fn stash_clickable_count(&self) -> usize {
        self.graph.stashes_window.as_ref().map(|window| window.total).unwrap_or(self.oids.stashes.len())
    }

    fn reflog_clickable_count(&self) -> usize {
        self.graph.reflogs_window.as_ref().map(|window| window.total).unwrap_or(self.reflogs.entries.len())
    }

    fn search_clickable_count(&self) -> usize {
        self.search_rows.len()
    }

    fn graph_pane_clickable(&self, index: usize, kind: GraphPaneClickKind) -> bool {
        if self.graph_tx.is_none() {
            return true;
        }

        let window = match kind {
            GraphPaneClickKind::Branches => self.graph.branches_window.as_ref(),
            GraphPaneClickKind::Tags => self.graph.tags_window.as_ref(),
            GraphPaneClickKind::Stashes => self.graph.stashes_window.as_ref(),
            GraphPaneClickKind::Reflogs => self.graph.reflogs_window.as_ref(),
        };

        window.is_some_and(|window| index >= window.start && index < window.end && index - window.start < window.rows.len())
    }

    fn inspector_clickable(&self) -> bool {
        if self.graph_selected == 0 {
            return self.uncommitted.has_conflicts;
        }
        self.graph_identity_at(self.graph_selected).is_some()
    }

    fn status_top_clickable_count(&self) -> usize {
        if self.graph_selected == 0 {
            if !self.is_uncommitted_loaded || !self.uncommitted.is_staged {
                return 0;
            }
            self.uncommitted.conflicts.len() + self.uncommitted.staged.modified.len() + self.uncommitted.staged.added.len() + self.uncommitted.staged.deleted.len()
        } else if self.selected_commit_diff_is_loaded() {
            self.current_diff.len()
        } else {
            0
        }
    }

    fn status_bottom_clickable_count(&self) -> usize {
        if self.graph_selected != 0 || !self.is_uncommitted_loaded || !self.uncommitted.is_unstaged {
            return 0;
        }
        self.uncommitted.conflicts.len() + self.uncommitted.unstaged.modified.len() + self.uncommitted.unstaged.added.len() + self.uncommitted.unstaged.deleted.len()
    }

    fn scrollbar_drag_at(&mut self, column: u16, row: u16) -> Option<ScrollbarDrag> {
        let info = self.scrollbar_info_at(column, row)?;
        let metrics = Self::scrollbar_metrics(info)?;
        if row < metrics.track_top {
            return None;
        }
        let track_offset = row.saturating_sub(metrics.track_top) as usize;
        if track_offset >= metrics.track_len {
            return None;
        }

        let thumb_end = metrics.thumb_start.saturating_add(metrics.thumb_len);
        let grab_offset = if track_offset >= metrics.thumb_start && track_offset < thumb_end { track_offset.saturating_sub(metrics.thumb_start) } else { metrics.thumb_len / 2 };

        Some(ScrollbarDrag { target: info.target, grab_offset })
    }

    fn scrollbar_info_at(&mut self, column: u16, row: u16) -> Option<ScrollbarInfo> {
        if self.is_modal_focus() || self.viewport == Viewport::Splash {
            return None;
        }

        const TARGETS: [ScrollbarTarget; 13] = [
            ScrollbarTarget::Branches,
            ScrollbarTarget::Tags,
            ScrollbarTarget::Stashes,
            ScrollbarTarget::Reflogs,
            ScrollbarTarget::Worktrees,
            ScrollbarTarget::Submodules,
            ScrollbarTarget::Search,
            ScrollbarTarget::Inspector,
            ScrollbarTarget::StatusTop,
            ScrollbarTarget::StatusBottom,
            ScrollbarTarget::Graph,
            ScrollbarTarget::Viewer,
            ScrollbarTarget::Settings,
        ];

        for target in TARGETS {
            let Some(rect) = self.scrollbar_rect_for_target(target) else {
                continue;
            };
            if !Self::scrollbar_column_contains(rect, column, row) {
                continue;
            }
            let Some(info) = self.scrollbar_info_for_target(target, rect) else {
                continue;
            };
            let Some(metrics) = Self::scrollbar_metrics(info) else {
                continue;
            };
            if row >= metrics.track_top && (row.saturating_sub(metrics.track_top) as usize) < metrics.track_len {
                return Some(info);
            }
        }

        None
    }

    fn scrollbar_rect_for_target(&self, target: ScrollbarTarget) -> Option<Rect> {
        if self.viewport == Viewport::Splash {
            return None;
        }

        let is_settings = self.viewport == Viewport::Settings;
        let is_main_view = matches!(self.viewport, Viewport::Graph | Viewport::Viewer);

        let rect = match target {
            ScrollbarTarget::Graph if self.viewport == Viewport::Graph => self.layout.graph_scrollbar,
            ScrollbarTarget::Viewer if self.viewport == Viewport::Viewer => self.layout.graph_scrollbar,
            ScrollbarTarget::Settings if is_settings => self.layout.app,
            ScrollbarTarget::Branches if is_main_view && self.layout_config.is_branches => self.layout.branches_scrollbar,
            ScrollbarTarget::Tags if is_main_view && self.layout_config.is_tags => self.layout.tags_scrollbar,
            ScrollbarTarget::Stashes if is_main_view && self.layout_config.is_stashes => self.layout.stashes_scrollbar,
            ScrollbarTarget::Reflogs if is_main_view && self.layout_config.is_reflogs => self.layout.reflogs_scrollbar,
            ScrollbarTarget::Worktrees if is_main_view && self.layout_config.is_worktrees => self.layout.worktrees_scrollbar,
            ScrollbarTarget::Submodules if is_main_view && self.layout_config.is_submodules => self.layout.submodules_scrollbar,
            ScrollbarTarget::Search if is_main_view && self.layout_config.is_search => self.layout.search_scrollbar,
            ScrollbarTarget::Inspector if is_main_view && self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) => self.layout.inspector_scrollbar,
            ScrollbarTarget::StatusTop if is_main_view && self.layout_config.is_status => self.layout.status_top_scrollbar,
            ScrollbarTarget::StatusBottom if is_main_view && self.layout_config.is_status && self.graph_selected == 0 => self.layout.status_bottom_scrollbar,
            _ => return None,
        };

        (rect.width > 0 && rect.height > 0).then_some(rect)
    }

    fn scrollbar_info_for_target(&mut self, target: ScrollbarTarget, rect: Rect) -> Option<ScrollbarInfo> {
        let (total_lines, visible_height, scroll) = match target {
            ScrollbarTarget::Graph => (self.graph_commit_count(), self.viewport_visible_height(), self.graph_scroll.get()),
            ScrollbarTarget::Viewer => (self.viewer_row_count(), self.viewport_visible_height(), self.viewer_scroll.get()),
            ScrollbarTarget::Settings => (self.settings_scroll_line_count(), self.settings_visible_height(), self.settings_scroll.get()),
            ScrollbarTarget::Branches => (self.branch_clickable_count(), self.branches_visible_height(), self.branches_scroll.get()),
            ScrollbarTarget::Tags => (self.tag_clickable_count(), self.tags_visible_height(), self.tags_scroll.get()),
            ScrollbarTarget::Stashes => (self.stash_clickable_count(), self.stashes_visible_height(), self.stashes_scroll.get()),
            ScrollbarTarget::Reflogs => (self.reflog_clickable_count(), self.reflogs_visible_height(), self.reflogs_scroll.get()),
            ScrollbarTarget::Worktrees => (self.worktrees.entries.len(), self.worktrees_visible_height(), self.worktrees_scroll.get()),
            ScrollbarTarget::Submodules => (self.submodules.entries.len(), self.submodules_visible_height(), self.submodules_scroll.get()),
            ScrollbarTarget::Search => (self.search_clickable_count(), self.search_visible_height(), self.search_scroll.get()),
            ScrollbarTarget::Inspector => (self.inspector_line_count_for_scrollbar(), self.inspector_visible_height(), self.inspector_scroll.get()),
            ScrollbarTarget::StatusTop => (self.status_top_clickable_count(), self.status_top_visible_height(), self.status_top_scroll.get()),
            ScrollbarTarget::StatusBottom => (self.status_bottom_clickable_count(), self.status_bottom_visible_height(), self.status_bottom_scroll.get()),
        };

        if visible_height == 0 {
            return None;
        }

        let max_scroll = total_lines.saturating_sub(visible_height);
        if max_scroll == 0 {
            return None;
        }

        Some(ScrollbarInfo { target, rect, total_lines, visible_height, scroll: scroll.min(max_scroll), max_scroll })
    }

    fn scrollbar_column_contains(rect: Rect, column: u16, row: u16) -> bool {
        if rect.width == 0 || rect.height == 0 {
            return false;
        }
        let right = rect.x.saturating_add(rect.width.saturating_sub(1));
        column == right && row >= rect.y && row < rect.y.saturating_add(rect.height)
    }

    fn scrollbar_metrics(info: ScrollbarInfo) -> Option<ScrollbarMetrics> {
        let track_len = info.rect.height.saturating_sub(2) as usize;
        if track_len == 0 {
            return None;
        }

        let viewport_len = info.rect.height as usize;
        let content_len = scrollbar_content_length(info.total_lines, info.visible_height);
        let max_position = content_len.saturating_sub(1);
        let max_viewport_position = max_position.saturating_add(viewport_len);
        if max_viewport_position == 0 {
            return None;
        }

        let thumb_len = Self::rounding_divide(viewport_len.saturating_mul(track_len), max_viewport_position).clamp(1, track_len);
        let thumb_start = Self::rounding_divide(info.scroll.min(max_position).saturating_mul(track_len), max_viewport_position).clamp(0, track_len.saturating_sub(1));

        Some(ScrollbarMetrics { track_top: info.rect.y.saturating_add(1), track_len, thumb_start, thumb_len, max_viewport_position })
    }

    fn rounding_divide(numerator: usize, denominator: usize) -> usize {
        if denominator == 0 { 0 } else { numerator.saturating_add(denominator / 2) / denominator }
    }

    fn apply_scrollbar_drag(&mut self, drag: ScrollbarDrag, row: u16) {
        let Some(rect) = self.scrollbar_rect_for_target(drag.target) else {
            return;
        };
        let Some(info) = self.scrollbar_info_for_target(drag.target, rect) else {
            return;
        };
        let Some(metrics) = Self::scrollbar_metrics(info) else {
            return;
        };

        let track_offset = if row <= metrics.track_top { 0 } else { row.saturating_sub(metrics.track_top) as usize }.min(metrics.track_len.saturating_sub(1));
        let max_thumb_start = metrics.track_len.saturating_sub(metrics.thumb_len);
        let desired_thumb_start = track_offset.saturating_sub(drag.grab_offset).min(max_thumb_start);
        let next_scroll = if max_thumb_start > 0 && desired_thumb_start == max_thumb_start {
            info.max_scroll
        } else {
            Self::rounding_divide(desired_thumb_start.saturating_mul(metrics.max_viewport_position), metrics.track_len).min(info.max_scroll)
        };

        self.set_scrollbar_scroll(info, next_scroll);
    }

    fn set_scrollbar_scroll(&mut self, info: ScrollbarInfo, scroll: usize) {
        let scroll = scroll.min(info.max_scroll);
        self.focus_scrollbar_target(info.target);
        self.set_target_scroll(info.target, scroll);
        self.clamp_target_selection_to_scroll(info.target, scroll, info.total_lines, info.visible_height);
    }

    fn focus_scrollbar_target(&mut self, target: ScrollbarTarget) {
        match target {
            ScrollbarTarget::Graph => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Graph;
            },
            ScrollbarTarget::Viewer => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Viewer;
            },
            ScrollbarTarget::Settings => {
                self.focus = Focus::Viewport;
                self.viewport = Viewport::Settings;
                self.last_input_direction = None;
            },
            ScrollbarTarget::Branches => self.focus = Focus::Branches,
            ScrollbarTarget::Tags => self.focus = Focus::Tags,
            ScrollbarTarget::Stashes => self.focus = Focus::Stashes,
            ScrollbarTarget::Reflogs => self.focus = Focus::Reflogs,
            ScrollbarTarget::Worktrees => self.focus = Focus::Worktrees,
            ScrollbarTarget::Submodules => self.focus = Focus::Submodules,
            ScrollbarTarget::Search => self.focus = Focus::Search,
            ScrollbarTarget::Inspector => self.focus = Focus::Inspector,
            ScrollbarTarget::StatusTop => self.focus = Focus::StatusTop,
            ScrollbarTarget::StatusBottom => self.focus = Focus::StatusBottom,
        }
    }

    fn set_target_scroll(&self, target: ScrollbarTarget, scroll: usize) {
        match target {
            ScrollbarTarget::Graph => self.graph_scroll.set(scroll),
            ScrollbarTarget::Viewer => self.viewer_scroll.set(scroll),
            ScrollbarTarget::Settings => self.settings_scroll.set(scroll),
            ScrollbarTarget::Branches => self.branches_scroll.set(scroll),
            ScrollbarTarget::Tags => self.tags_scroll.set(scroll),
            ScrollbarTarget::Stashes => self.stashes_scroll.set(scroll),
            ScrollbarTarget::Reflogs => self.reflogs_scroll.set(scroll),
            ScrollbarTarget::Worktrees => self.worktrees_scroll.set(scroll),
            ScrollbarTarget::Submodules => self.submodules_scroll.set(scroll),
            ScrollbarTarget::Search => self.search_scroll.set(scroll),
            ScrollbarTarget::Inspector => self.inspector_scroll.set(scroll),
            ScrollbarTarget::StatusTop => self.status_top_scroll.set(scroll),
            ScrollbarTarget::StatusBottom => self.status_bottom_scroll.set(scroll),
        }
    }

    fn clamp_target_selection_to_scroll(&mut self, target: ScrollbarTarget, scroll: usize, total_lines: usize, visible_height: usize) {
        if visible_height == 0 || total_lines == 0 {
            self.set_target_selection(target, 0);
            return;
        }

        if target == ScrollbarTarget::Settings {
            self.clamp_settings_selection_to_scroll(scroll, total_lines, visible_height);
            return;
        }

        let first = scroll.min(total_lines.saturating_sub(1));
        let last = scroll.saturating_add(visible_height).saturating_sub(1).min(total_lines.saturating_sub(1));
        let current = self.target_selection(target).min(total_lines.saturating_sub(1));
        let selection = if current < first {
            first
        } else if current > last {
            last
        } else {
            current
        };
        self.set_target_selection(target, selection);
    }

    fn clamp_settings_selection_to_scroll(&mut self, scroll: usize, total_lines: usize, visible_height: usize) {
        if total_lines == 0 {
            self.settings_selected = 0;
            return;
        }

        let first = scroll.min(total_lines.saturating_sub(1));
        let end = scroll.saturating_add(visible_height).min(total_lines);
        if self.settings_selections.iter().any(|selection| selection.line == self.settings_selected && selection.line >= first && selection.line < end) {
            return;
        }

        let visible = self.settings_selections.iter().map(|selection| selection.line).find(|&line| line >= first && line < end);
        let nearest = visible.or_else(|| self.settings_selections.iter().map(|selection| selection.line).min_by_key(|&line| line.abs_diff(first)));
        self.settings_selected = nearest.unwrap_or(first);
    }

    fn target_selection(&self, target: ScrollbarTarget) -> usize {
        match target {
            ScrollbarTarget::Graph => self.graph_selected,
            ScrollbarTarget::Viewer => self.viewer_selected,
            ScrollbarTarget::Settings => self.settings_selected,
            ScrollbarTarget::Branches => self.branches_selected,
            ScrollbarTarget::Tags => self.tags_selected,
            ScrollbarTarget::Stashes => self.stashes_selected,
            ScrollbarTarget::Reflogs => self.reflogs_selected,
            ScrollbarTarget::Worktrees => self.worktrees_selected,
            ScrollbarTarget::Submodules => self.submodules_selected,
            ScrollbarTarget::Search => self.search_selected,
            ScrollbarTarget::Inspector => self.inspector_selected,
            ScrollbarTarget::StatusTop => self.status_top_selected,
            ScrollbarTarget::StatusBottom => self.status_bottom_selected,
        }
    }

    fn set_target_selection(&mut self, target: ScrollbarTarget, selection: usize) {
        match target {
            ScrollbarTarget::Graph => self.select_graph_index(selection),
            ScrollbarTarget::Viewer => self.viewer_selected = selection,
            ScrollbarTarget::Settings => self.settings_selected = selection,
            ScrollbarTarget::Branches => self.branches_selected = selection,
            ScrollbarTarget::Tags => self.tags_selected = selection,
            ScrollbarTarget::Stashes => self.stashes_selected = selection,
            ScrollbarTarget::Reflogs => self.reflogs_selected = selection,
            ScrollbarTarget::Worktrees => self.worktrees_selected = selection,
            ScrollbarTarget::Submodules => self.submodules_selected = selection,
            ScrollbarTarget::Search => self.search_selected = selection,
            ScrollbarTarget::Inspector => self.inspector_selected = selection,
            ScrollbarTarget::StatusTop => self.status_top_selected = selection,
            ScrollbarTarget::StatusBottom => self.status_bottom_selected = selection,
        }
    }

    fn viewport_visible_height(&self) -> usize {
        if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize }
    }

    fn settings_visible_height(&self) -> usize {
        if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize }
    }

    fn branches_visible_height(&self) -> usize {
        self.layout.branches.height.saturating_sub(2) as usize
    }

    fn tags_visible_height(&self) -> usize {
        if self.layout_config.is_zen { self.layout.tags.height.saturating_sub(2) as usize } else { self.layout.tags.height.saturating_sub(if self.layout_config.is_branches { 1 } else { 2 }) as usize }
    }

    fn stashes_visible_height(&self) -> usize {
        if self.layout_config.is_zen {
            self.layout.stashes.height.saturating_sub(2) as usize
        } else {
            self.layout.stashes.height.saturating_sub(if self.layout_config.is_branches || self.layout_config.is_tags { 1 } else { 2 }) as usize
        }
    }

    fn reflogs_visible_height(&self) -> usize {
        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes;
        if self.layout_config.is_zen { self.layout.reflogs.height.saturating_sub(2) as usize } else { self.layout.reflogs.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize }
    }

    fn worktrees_visible_height(&self) -> usize {
        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs;
        if self.layout_config.is_zen { self.layout.worktrees.height.saturating_sub(2) as usize } else { self.layout.worktrees.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize }
    }

    fn submodules_visible_height(&self) -> usize {
        let has_previous = self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees;
        if self.layout_config.is_zen { self.layout.submodules.height.saturating_sub(2) as usize } else { self.layout.submodules.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize }
    }

    fn search_visible_height(&self) -> usize {
        let has_previous = self.layout_config.is_branches
            || self.layout_config.is_tags
            || self.layout_config.is_stashes
            || self.layout_config.is_reflogs
            || self.layout_config.is_worktrees
            || self.layout_config.is_submodules;
        if self.layout_config.is_zen { self.layout.search.height.saturating_sub(2) as usize } else { self.layout.search.height.saturating_sub(if has_previous { 1 } else { 2 }) as usize }
    }

    fn inspector_visible_height(&self) -> usize {
        if self.layout_config.is_zen { self.layout.inspector.height.saturating_sub(2) as usize } else { self.layout.inspector.height.saturating_sub(1) as usize }
    }

    fn status_top_visible_height(&self) -> usize {
        self.layout.status_top.height.saturating_sub(2) as usize
    }

    fn status_bottom_visible_height(&self) -> usize {
        self.layout.status_bottom.height.saturating_sub(2) as usize
    }

    fn settings_scroll_line_count(&mut self) -> usize {
        if let Some(repo) = self.repo.clone() {
            self.settings_lines(&repo).len()
        } else {
            self.settings_selections.iter().map(|selection| selection.line).max().map(|line| line.saturating_add(1)).unwrap_or(0)
        }
    }

    fn inspector_line_count_for_scrollbar(&self) -> usize {
        if self.graph_selected == 0 {
            return if self.uncommitted.has_conflicts { 8 } else { 0 };
        }

        let visible_height = self.inspector_visible_height();
        let max_text_width = self.layout.inspector.width.saturating_sub(1) as usize;
        let max_text_width = max_text_width.saturating_sub(2);
        let Some(repo) = self.repo.as_ref() else {
            return self.inspector_scroll.get().saturating_add(visible_height).saturating_add(1);
        };
        let Some(identity) = self.graph_identity_at(self.graph_selected) else {
            return empty_state_top_padding(visible_height).saturating_add(1);
        };
        let Ok(commit) = repo.find_commit(identity.oid) else {
            return empty_state_top_padding(visible_height).saturating_add(1);
        };

        let mut count = 4usize.saturating_add(commit.parent_count());
        if let Some(row) = self.graph_row_at(self.graph_selected)
            && !row.branches.is_empty()
        {
            count = count.saturating_add(2).saturating_add(row.branches.len());
        } else if let Some(branches) = self.branches.all.get(&identity.alias)
            && self.branches.colors.contains_key(&identity.alias)
        {
            let visible_branches = branches.iter().filter(|branch| !self.branches.hidden_branch_names.contains(*branch)).count();
            count = count.saturating_add(2).saturating_add(visible_branches);
        }

        if let Some(row) = self.graph_row_at(self.graph_selected)
            && let Some(entry) = &row.reflog
        {
            count = count.saturating_add(3).saturating_add(wrap_words(sanitize(entry.message.clone()), max_text_width).len());
        } else if let Some(entry) = self.reflogs.latest_for_alias(identity.alias) {
            count = count.saturating_add(4).saturating_add(wrap_words(sanitize(entry.message.clone()), max_text_width).len());
        }

        let summary = commit.summary().map(str::to_string).unwrap_or_else(|| format!("{} {}", self.symbols.empty_state.mark, crate::helpers::localisation::empty::NO_SUMMARY()));
        let body = commit.body().map(str::to_string).unwrap_or_else(|| format!("{} {}", self.symbols.empty_state.mark, crate::helpers::localisation::empty::NO_BODY()));
        count.saturating_add(10).saturating_add(wrap_words(sanitize(summary), max_text_width).len()).saturating_add(2).saturating_add(wrap_words(sanitize(body), max_text_width).len())
    }

    fn handle_mouse_scroll(&mut self, column: u16, row: u16, direction: Direction) {
        if self.mouse_drag.is_some() {
            return;
        }

        if self.is_modal_focus() {
            match direction {
                Direction::Up => self.on_scroll_up(),
                Direction::Down => self.on_scroll_down(),
            }
            return;
        }

        if let Some(focus) = self.scroll_focus_at(column, row) {
            self.focus = focus;
            match direction {
                Direction::Up => self.on_scroll_up(),
                Direction::Down => self.on_scroll_down(),
            }
        }
    }

    fn scroll_focus_at(&self, column: u16, row: u16) -> Option<Focus> {
        if self.layout_config.is_zen {
            return Self::rect_contains(self.layout.app, column, row).then_some(self.focus);
        }

        if matches!(self.viewport, Viewport::Splash | Viewport::Settings) {
            return Self::rect_contains(self.layout.app, column, row).then_some(Focus::Viewport);
        }

        if self.layout_config.is_branches && Self::rect_contains(self.layout.pane_branches, column, row) {
            return Some(Focus::Branches);
        }
        if self.layout_config.is_tags && Self::rect_contains(self.layout.pane_tags, column, row) {
            return Some(Focus::Tags);
        }
        if self.layout_config.is_stashes && Self::rect_contains(self.layout.pane_stashes, column, row) {
            return Some(Focus::Stashes);
        }
        if self.layout_config.is_reflogs && Self::rect_contains(self.layout.pane_reflogs, column, row) {
            return Some(Focus::Reflogs);
        }
        if self.layout_config.is_worktrees && Self::rect_contains(self.layout.pane_worktrees, column, row) {
            return Some(Focus::Worktrees);
        }
        if self.layout_config.is_submodules && Self::rect_contains(self.layout.pane_submodules, column, row) {
            return Some(Focus::Submodules);
        }
        if self.layout_config.is_search && Self::rect_contains(self.layout.pane_search, column, row) {
            return Some(Focus::Search);
        }
        if self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts) && Self::rect_contains(self.layout.pane_inspector, column, row) {
            return Some(Focus::Inspector);
        }
        if self.layout_config.is_status && self.graph_selected == 0 && Self::rect_contains(self.layout.pane_status_bottom, column, row) {
            return Some(Focus::StatusBottom);
        }
        if self.layout_config.is_status && Self::rect_contains(self.layout.pane_status_top, column, row) {
            return Some(Focus::StatusTop);
        }
        if Self::rect_contains(self.layout.graph_scrollbar, column, row) {
            return Some(Focus::Viewport);
        }

        None
    }

    fn layout_drag_at(&self, column: u16, row: u16) -> Option<LayoutDrag> {
        if matches!(self.viewport, Viewport::Splash | Viewport::Settings) || self.is_modal_focus() {
            return None;
        }

        if self.layout_config.is_zen {
            return None;
        }
        if Self::rect_contains(self.layout.divider_left, column, row) {
            return Some(LayoutDrag::LeftPane);
        }
        if Self::rect_contains(self.layout.divider_right, column, row) {
            return Some(LayoutDrag::RightPane);
        }
        if Self::rect_contains(self.layout.divider_branches_tags, column, row) {
            return Some(LayoutDrag::BranchesTags);
        }
        if Self::rect_contains(self.layout.divider_branches_stashes, column, row) {
            return Some(LayoutDrag::BranchesStashes);
        }
        if Self::rect_contains(self.layout.divider_branches_reflogs, column, row) {
            return Some(LayoutDrag::BranchesReflogs);
        }
        if Self::rect_contains(self.layout.divider_branches_worktrees, column, row) {
            return Some(LayoutDrag::BranchesWorktrees);
        }
        if Self::rect_contains(self.layout.divider_branches_submodules, column, row) {
            return Some(LayoutDrag::BranchesSubmodules);
        }
        if Self::rect_contains(self.layout.divider_branches_search, column, row) {
            return Some(LayoutDrag::BranchesSearch);
        }
        if Self::rect_contains(self.layout.divider_tags_stashes, column, row) {
            return Some(LayoutDrag::TagsStashes);
        }
        if Self::rect_contains(self.layout.divider_tags_reflogs, column, row) {
            return Some(LayoutDrag::TagsReflogs);
        }
        if Self::rect_contains(self.layout.divider_tags_worktrees, column, row) {
            return Some(LayoutDrag::TagsWorktrees);
        }
        if Self::rect_contains(self.layout.divider_tags_submodules, column, row) {
            return Some(LayoutDrag::TagsSubmodules);
        }
        if Self::rect_contains(self.layout.divider_tags_search, column, row) {
            return Some(LayoutDrag::TagsSearch);
        }
        if Self::rect_contains(self.layout.divider_stashes_reflogs, column, row) {
            return Some(LayoutDrag::StashesReflogs);
        }
        if Self::rect_contains(self.layout.divider_stashes_worktrees, column, row) {
            return Some(LayoutDrag::StashesWorktrees);
        }
        if Self::rect_contains(self.layout.divider_stashes_submodules, column, row) {
            return Some(LayoutDrag::StashesSubmodules);
        }
        if Self::rect_contains(self.layout.divider_stashes_search, column, row) {
            return Some(LayoutDrag::StashesSearch);
        }
        if Self::rect_contains(self.layout.divider_reflogs_worktrees, column, row) {
            return Some(LayoutDrag::ReflogsWorktrees);
        }
        if Self::rect_contains(self.layout.divider_reflogs_submodules, column, row) {
            return Some(LayoutDrag::ReflogsSubmodules);
        }
        if Self::rect_contains(self.layout.divider_worktrees_submodules, column, row) {
            return Some(LayoutDrag::WorktreesSubmodules);
        }
        if Self::rect_contains(self.layout.divider_reflogs_search, column, row) {
            return Some(LayoutDrag::ReflogsSearch);
        }
        if Self::rect_contains(self.layout.divider_worktrees_search, column, row) {
            return Some(LayoutDrag::WorktreesSearch);
        }
        if Self::rect_contains(self.layout.divider_submodules_search, column, row) {
            return Some(LayoutDrag::SubmodulesSearch);
        }
        if Self::rect_contains(self.layout.divider_inspector_status, column, row) {
            return Some(LayoutDrag::InspectorStatus);
        }
        if Self::rect_contains(self.layout.divider_status_files, column, row) {
            return Some(LayoutDrag::StatusFiles);
        }

        None
    }

    pub(crate) fn is_modal_focus(&self) -> bool {
        matches!(
            self.focus,
            Focus::ModalCheckout
                | Focus::ModalSolo
                | Focus::ModalCommit
                | Focus::ModalCherrypick
                | Focus::ModalRevert
                | Focus::ModalCreateBranch
                | Focus::ModalRenameBranch
                | Focus::ModalCreateWorktreeName
                | Focus::ModalCreateWorktreePath
                | Focus::ModalDeleteBranch
                | Focus::ModalWorktreeChooser
                | Focus::ModalRemoveWorktree
                | Focus::ModalLockWorktree
                | Focus::ModalRemoteAction
                | Focus::ModalRemoteDelete
                | Focus::ModalRemoteName
                | Focus::ModalRemoteUrl
                | Focus::ModalGrep
                | Focus::ModalFileSearch
                | Focus::ModalTag
                | Focus::ModalDeleteTag
                | Focus::ModalKeyCapture
                | Focus::ModalAuth
                | Focus::ModalNetworkProgress
                | Focus::ModalOperationProgress
                | Focus::ModalOperationConflict
                | Focus::ModalOperationSuccess
                | Focus::ModalError
        )
    }

    fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
        rect.width > 0 && rect.height > 0 && column >= rect.x && column < rect.x.saturating_add(rect.width) && row >= rect.y && row < rect.y.saturating_add(rect.height)
    }

    pub fn on_resize_pane_left(&mut self) {
        self.resize_pane_keyboard(ResizeDirection::Left);
    }

    pub fn on_resize_pane_down(&mut self) {
        self.resize_pane_keyboard(ResizeDirection::Down);
    }

    pub fn on_resize_pane_up(&mut self) {
        self.resize_pane_keyboard(ResizeDirection::Up);
    }

    pub fn on_resize_pane_right(&mut self) {
        self.resize_pane_keyboard(ResizeDirection::Right);
    }

    fn resize_pane_keyboard(&mut self, direction: ResizeDirection) {
        if matches!(self.viewport, Viewport::Splash | Viewport::Settings) || self.is_modal_focus() {
            return;
        }

        let changed = match direction {
            ResizeDirection::Left => {
                if self.layout_config.is_zen {
                    false
                } else {
                    match self.focus {
                        Focus::Branches | Focus::Tags | Focus::Stashes | Focus::Reflogs | Focus::Worktrees | Focus::Submodules | Focus::Search | Focus::Viewport => self.resize_left_column_by(-1),
                        Focus::Inspector | Focus::StatusTop | Focus::StatusBottom => self.resize_right_column_by(1),
                        _ => false,
                    }
                }
            },
            ResizeDirection::Right => {
                if self.layout_config.is_zen {
                    false
                } else {
                    match self.focus {
                        Focus::Branches | Focus::Tags | Focus::Stashes | Focus::Reflogs | Focus::Worktrees | Focus::Submodules | Focus::Search => self.resize_left_column_by(1),
                        Focus::Viewport | Focus::Inspector | Focus::StatusTop | Focus::StatusBottom => self.resize_right_column_by(-1),
                        _ => false,
                    }
                }
            },
            ResizeDirection::Up => {
                if self.layout_config.is_zen {
                    false
                } else {
                    self.resize_stack_pane(Direction::Up)
                }
            },
            ResizeDirection::Down => {
                if self.layout_config.is_zen {
                    false
                } else {
                    self.resize_stack_pane(Direction::Down)
                }
            },
        };

        if changed {
            self.save_layout();
            self.mark_viewer_layout_dirty();
        }
    }

    fn resize_left_column_by(&mut self, delta: i16) -> bool {
        if self.layout.pane_left.width == 0 {
            return false;
        }

        let total_width = self.layout.app.width;
        let other_width = self.layout.pane_right.width;
        let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
        if max_width < LAYOUT_WIDTH_MIN_SIDE_PANE {
            return false;
        }

        let current = self.layout_config.width_left_pane.clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
        let width = Self::u16_add_signed(current, delta).clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
        let changed = self.layout_config.width_left_pane != width;
        self.layout_config.width_left_pane = width;
        changed
    }

    fn resize_right_column_by(&mut self, delta: i16) -> bool {
        if self.layout.pane_right.width == 0 {
            return false;
        }

        let total_width = self.layout.app.width;
        let other_width = self.layout.pane_left.width;
        let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
        if max_width < LAYOUT_WIDTH_MIN_SIDE_PANE {
            return false;
        }

        let current = self.layout_config.width_right_pane.clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
        let width = Self::u16_add_signed(current, delta).clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
        let changed = self.layout_config.width_right_pane != width;
        self.layout_config.width_right_pane = width;
        changed
    }

    fn resize_stack_pane(&mut self, direction: Direction) -> bool {
        let Some(focused) = self.focus_stack_pane() else {
            return false;
        };
        if !matches!(focused, StackPane::Branches | StackPane::Tags | StackPane::Stashes | StackPane::Reflogs | StackPane::Worktrees | StackPane::Submodules | StackPane::Search) {
            return self.resize_right_stack_pane(focused, direction);
        }

        let stack = self.active_left_stack();
        if stack.len() < 2 {
            return false;
        }

        let Some(index) = stack.iter().position(|&pane| pane == focused) else {
            return false;
        };

        match direction {
            Direction::Up if index > 0 => self.adjust_stack_pair(stack[index - 1], stack[index], -1),
            Direction::Up if index + 1 < stack.len() => self.adjust_stack_pair(stack[index], stack[index + 1], -1),
            Direction::Down if index + 1 < stack.len() => self.adjust_stack_pair(stack[index], stack[index + 1], 1),
            Direction::Down if index > 0 => self.adjust_stack_pair(stack[index - 1], stack[index], 1),
            _ => false,
        }
    }

    fn resize_right_stack_pane(&mut self, focused: StackPane, direction: Direction) -> bool {
        let has_inspector = self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts);
        let has_status = self.layout_config.is_status;
        let has_status_bottom = has_status && self.graph_selected == 0;

        match (focused, direction) {
            (StackPane::Inspector, Direction::Up) if has_inspector && has_status => self.adjust_stack_pair(StackPane::Inspector, StackPane::Status, -1),
            (StackPane::Inspector, Direction::Down) if has_inspector && has_status => self.adjust_stack_pair(StackPane::Inspector, StackPane::Status, 1),
            (StackPane::StatusTop, Direction::Up) if has_status && has_inspector => self.adjust_stack_pair(StackPane::Inspector, StackPane::Status, -1),
            (StackPane::StatusTop, Direction::Up) if has_status && has_status_bottom => self.adjust_stack_pair(StackPane::StatusTop, StackPane::StatusBottom, -1),
            (StackPane::StatusTop, Direction::Down) if has_status && has_status_bottom => self.adjust_stack_pair(StackPane::StatusTop, StackPane::StatusBottom, 1),
            (StackPane::StatusTop, Direction::Down) if has_status && has_inspector => self.adjust_stack_pair(StackPane::Inspector, StackPane::Status, 1),
            (StackPane::StatusBottom, Direction::Up) if has_status_bottom => self.adjust_stack_pair(StackPane::StatusTop, StackPane::StatusBottom, -1),
            (StackPane::StatusBottom, Direction::Down) if has_status_bottom => self.adjust_stack_pair(StackPane::StatusTop, StackPane::StatusBottom, 1),
            _ => false,
        }
    }

    fn adjust_stack_pair(&mut self, first: StackPane, second: StackPane, delta_first: i16) -> bool {
        let before = (self.stack_pane_weight(first), self.stack_pane_weight(second));
        let Some((first_weight, second_weight)) = Self::resized_pair_weights_by_delta(delta_first, self.stack_pane_rect(first), self.stack_pane_rect(second), before.0, before.1) else {
            return false;
        };

        self.set_stack_pane_weight(first, first_weight);
        self.set_stack_pane_weight(second, second_weight);
        before != (first_weight, second_weight)
    }

    fn active_left_stack(&self) -> Vec<StackPane> {
        let mut stack = Vec::new();
        if self.layout_config.is_branches {
            stack.push(StackPane::Branches);
        }
        if self.layout_config.is_tags {
            stack.push(StackPane::Tags);
        }
        if self.layout_config.is_stashes {
            stack.push(StackPane::Stashes);
        }
        if self.layout_config.is_reflogs {
            stack.push(StackPane::Reflogs);
        }
        if self.layout_config.is_worktrees {
            stack.push(StackPane::Worktrees);
        }
        if self.layout_config.is_submodules {
            stack.push(StackPane::Submodules);
        }
        if self.layout_config.is_search {
            stack.push(StackPane::Search);
        }
        stack
    }

    fn focus_stack_pane(&self) -> Option<StackPane> {
        match self.focus {
            Focus::Branches => Some(StackPane::Branches),
            Focus::Tags => Some(StackPane::Tags),
            Focus::Stashes => Some(StackPane::Stashes),
            Focus::Reflogs => Some(StackPane::Reflogs),
            Focus::Worktrees => Some(StackPane::Worktrees),
            Focus::Submodules => Some(StackPane::Submodules),
            Focus::Search => Some(StackPane::Search),
            Focus::Inspector => Some(StackPane::Inspector),
            Focus::StatusTop => Some(StackPane::StatusTop),
            Focus::StatusBottom => Some(StackPane::StatusBottom),
            _ => None,
        }
    }

    fn stack_pane_rect(&self, pane: StackPane) -> Rect {
        match pane {
            StackPane::Branches => self.layout.pane_branches,
            StackPane::Tags => self.layout.pane_tags,
            StackPane::Stashes => self.layout.pane_stashes,
            StackPane::Reflogs => self.layout.pane_reflogs,
            StackPane::Worktrees => self.layout.pane_worktrees,
            StackPane::Submodules => self.layout.pane_submodules,
            StackPane::Search => self.layout.pane_search,
            StackPane::Inspector => self.layout.pane_inspector,
            StackPane::Status => self.layout.pane_status,
            StackPane::StatusTop => self.layout.pane_status_top,
            StackPane::StatusBottom => self.layout.pane_status_bottom,
        }
    }

    fn stack_pane_weight(&self, pane: StackPane) -> u16 {
        match pane {
            StackPane::Branches => self.layout_config.weight_branches,
            StackPane::Tags => self.layout_config.weight_tags,
            StackPane::Stashes => self.layout_config.weight_stashes,
            StackPane::Reflogs => self.layout_config.weight_reflogs,
            StackPane::Worktrees => self.layout_config.weight_worktrees,
            StackPane::Submodules => self.layout_config.weight_submodules,
            StackPane::Search => self.layout_config.weight_search,
            StackPane::Inspector => self.layout_config.weight_inspector,
            StackPane::Status => self.layout_config.weight_status,
            StackPane::StatusTop => self.layout_config.weight_status_top,
            StackPane::StatusBottom => self.layout_config.weight_status_bottom,
        }
    }

    fn set_stack_pane_weight(&mut self, pane: StackPane, weight: u16) {
        match pane {
            StackPane::Branches => self.layout_config.weight_branches = weight,
            StackPane::Tags => self.layout_config.weight_tags = weight,
            StackPane::Stashes => self.layout_config.weight_stashes = weight,
            StackPane::Reflogs => self.layout_config.weight_reflogs = weight,
            StackPane::Worktrees => self.layout_config.weight_worktrees = weight,
            StackPane::Submodules => self.layout_config.weight_submodules = weight,
            StackPane::Search => self.layout_config.weight_search = weight,
            StackPane::Inspector => self.layout_config.weight_inspector = weight,
            StackPane::Status => self.layout_config.weight_status = weight,
            StackPane::StatusTop => self.layout_config.weight_status_top = weight,
            StackPane::StatusBottom => self.layout_config.weight_status_bottom = weight,
        }
    }

    fn u16_add_signed(value: u16, delta: i16) -> u16 {
        if delta >= 0 { value.saturating_add(delta as u16) } else { value.saturating_sub((-delta) as u16) }
    }

    fn apply_layout_drag(&mut self, drag: LayoutDrag, column: u16, row: u16) {
        match drag {
            LayoutDrag::LeftPane => self.resize_left_pane(column),
            LayoutDrag::RightPane => self.resize_right_pane(column),
            LayoutDrag::BranchesTags => {
                if let Some((branches, tags)) = Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_tags, self.layout_config.weight_branches, self.layout_config.weight_tags) {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_tags = tags;
                }
            },
            LayoutDrag::BranchesStashes => {
                if let Some((branches, stashes)) =
                    Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_stashes, self.layout_config.weight_branches, self.layout_config.weight_stashes)
                {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_stashes = stashes;
                }
            },
            LayoutDrag::BranchesReflogs => {
                if let Some((branches, reflogs)) =
                    Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_reflogs, self.layout_config.weight_branches, self.layout_config.weight_reflogs)
                {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_reflogs = reflogs;
                }
            },
            LayoutDrag::BranchesWorktrees => {
                if let Some((branches, worktrees)) =
                    Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_worktrees, self.layout_config.weight_branches, self.layout_config.weight_worktrees)
                {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_worktrees = worktrees;
                }
            },
            LayoutDrag::BranchesSubmodules => {
                if let Some((branches, submodules)) =
                    Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_submodules, self.layout_config.weight_branches, self.layout_config.weight_submodules)
                {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_submodules = submodules;
                }
            },
            LayoutDrag::BranchesSearch => {
                if let Some((branches, search)) =
                    Self::resized_pair_weights(row, self.layout.pane_branches, self.layout.pane_search, self.layout_config.weight_branches, self.layout_config.weight_search)
                {
                    self.layout_config.weight_branches = branches;
                    self.layout_config.weight_search = search;
                }
            },
            LayoutDrag::TagsStashes => {
                if let Some((tags, stashes)) = Self::resized_pair_weights(row, self.layout.pane_tags, self.layout.pane_stashes, self.layout_config.weight_tags, self.layout_config.weight_stashes) {
                    self.layout_config.weight_tags = tags;
                    self.layout_config.weight_stashes = stashes;
                }
            },
            LayoutDrag::TagsReflogs => {
                if let Some((tags, reflogs)) = Self::resized_pair_weights(row, self.layout.pane_tags, self.layout.pane_reflogs, self.layout_config.weight_tags, self.layout_config.weight_reflogs) {
                    self.layout_config.weight_tags = tags;
                    self.layout_config.weight_reflogs = reflogs;
                }
            },
            LayoutDrag::TagsWorktrees => {
                if let Some((tags, worktrees)) = Self::resized_pair_weights(row, self.layout.pane_tags, self.layout.pane_worktrees, self.layout_config.weight_tags, self.layout_config.weight_worktrees)
                {
                    self.layout_config.weight_tags = tags;
                    self.layout_config.weight_worktrees = worktrees;
                }
            },
            LayoutDrag::TagsSubmodules => {
                if let Some((tags, submodules)) =
                    Self::resized_pair_weights(row, self.layout.pane_tags, self.layout.pane_submodules, self.layout_config.weight_tags, self.layout_config.weight_submodules)
                {
                    self.layout_config.weight_tags = tags;
                    self.layout_config.weight_submodules = submodules;
                }
            },
            LayoutDrag::TagsSearch => {
                if let Some((tags, search)) = Self::resized_pair_weights(row, self.layout.pane_tags, self.layout.pane_search, self.layout_config.weight_tags, self.layout_config.weight_search) {
                    self.layout_config.weight_tags = tags;
                    self.layout_config.weight_search = search;
                }
            },
            LayoutDrag::StashesWorktrees => {
                if let Some((stashes, worktrees)) =
                    Self::resized_pair_weights(row, self.layout.pane_stashes, self.layout.pane_worktrees, self.layout_config.weight_stashes, self.layout_config.weight_worktrees)
                {
                    self.layout_config.weight_stashes = stashes;
                    self.layout_config.weight_worktrees = worktrees;
                }
            },
            LayoutDrag::StashesSubmodules => {
                if let Some((stashes, submodules)) =
                    Self::resized_pair_weights(row, self.layout.pane_stashes, self.layout.pane_submodules, self.layout_config.weight_stashes, self.layout_config.weight_submodules)
                {
                    self.layout_config.weight_stashes = stashes;
                    self.layout_config.weight_submodules = submodules;
                }
            },
            LayoutDrag::StashesReflogs => {
                if let Some((stashes, reflogs)) =
                    Self::resized_pair_weights(row, self.layout.pane_stashes, self.layout.pane_reflogs, self.layout_config.weight_stashes, self.layout_config.weight_reflogs)
                {
                    self.layout_config.weight_stashes = stashes;
                    self.layout_config.weight_reflogs = reflogs;
                }
            },
            LayoutDrag::StashesSearch => {
                if let Some((stashes, search)) = Self::resized_pair_weights(row, self.layout.pane_stashes, self.layout.pane_search, self.layout_config.weight_stashes, self.layout_config.weight_search)
                {
                    self.layout_config.weight_stashes = stashes;
                    self.layout_config.weight_search = search;
                }
            },
            LayoutDrag::ReflogsWorktrees => {
                if let Some((reflogs, worktrees)) =
                    Self::resized_pair_weights(row, self.layout.pane_reflogs, self.layout.pane_worktrees, self.layout_config.weight_reflogs, self.layout_config.weight_worktrees)
                {
                    self.layout_config.weight_reflogs = reflogs;
                    self.layout_config.weight_worktrees = worktrees;
                }
            },
            LayoutDrag::ReflogsSubmodules => {
                if let Some((reflogs, submodules)) =
                    Self::resized_pair_weights(row, self.layout.pane_reflogs, self.layout.pane_submodules, self.layout_config.weight_reflogs, self.layout_config.weight_submodules)
                {
                    self.layout_config.weight_reflogs = reflogs;
                    self.layout_config.weight_submodules = submodules;
                }
            },
            LayoutDrag::WorktreesSubmodules => {
                if let Some((worktrees, submodules)) =
                    Self::resized_pair_weights(row, self.layout.pane_worktrees, self.layout.pane_submodules, self.layout_config.weight_worktrees, self.layout_config.weight_submodules)
                {
                    self.layout_config.weight_worktrees = worktrees;
                    self.layout_config.weight_submodules = submodules;
                }
            },
            LayoutDrag::ReflogsSearch => {
                if let Some((reflogs, search)) = Self::resized_pair_weights(row, self.layout.pane_reflogs, self.layout.pane_search, self.layout_config.weight_reflogs, self.layout_config.weight_search)
                {
                    self.layout_config.weight_reflogs = reflogs;
                    self.layout_config.weight_search = search;
                }
            },
            LayoutDrag::WorktreesSearch => {
                if let Some((worktrees, search)) =
                    Self::resized_pair_weights(row, self.layout.pane_worktrees, self.layout.pane_search, self.layout_config.weight_worktrees, self.layout_config.weight_search)
                {
                    self.layout_config.weight_worktrees = worktrees;
                    self.layout_config.weight_search = search;
                }
            },
            LayoutDrag::SubmodulesSearch => {
                if let Some((submodules, search)) =
                    Self::resized_pair_weights(row, self.layout.pane_submodules, self.layout.pane_search, self.layout_config.weight_submodules, self.layout_config.weight_search)
                {
                    self.layout_config.weight_submodules = submodules;
                    self.layout_config.weight_search = search;
                }
            },
            LayoutDrag::InspectorStatus => {
                if let Some((inspector, status)) =
                    Self::resized_pair_weights(row, self.layout.pane_inspector, self.layout.pane_status, self.layout_config.weight_inspector, self.layout_config.weight_status)
                {
                    self.layout_config.weight_inspector = inspector;
                    self.layout_config.weight_status = status;
                }
            },
            LayoutDrag::StatusFiles => {
                if let Some((top, bottom)) =
                    Self::resized_pair_weights(row, self.layout.pane_status_top, self.layout.pane_status_bottom, self.layout_config.weight_status_top, self.layout_config.weight_status_bottom)
                {
                    self.layout_config.weight_status_top = top;
                    self.layout_config.weight_status_bottom = bottom;
                }
            },
        }
    }

    fn resize_left_pane(&mut self, column: u16) {
        let total_width = self.layout.app.width;
        let other_width = self.layout.pane_right.width;
        let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
        if max_width < LAYOUT_WIDTH_MIN_SIDE_PANE {
            return;
        }

        let desired_width = column.saturating_sub(self.layout.app.x);
        self.layout_config.width_left_pane = desired_width.clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
    }

    fn resize_right_pane(&mut self, column: u16) {
        let total_width = self.layout.app.width;
        let other_width = self.layout.pane_left.width;
        let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
        if max_width < LAYOUT_WIDTH_MIN_SIDE_PANE {
            return;
        }

        let app_right = self.layout.app.x.saturating_add(self.layout.app.width);
        let desired_width = app_right.saturating_sub(column.saturating_add(1));
        self.layout_config.width_right_pane = desired_width.clamp(LAYOUT_WIDTH_MIN_SIDE_PANE, max_width);
    }

    fn resized_pair_weights(row: u16, first: Rect, second: Rect, first_weight: u16, second_weight: u16) -> Option<(u16, u16)> {
        let first_height = row.saturating_sub(first.y).saturating_add(1);
        Self::resized_pair_weights_for_first_height(first_height, first, second, first_weight, second_weight)
    }

    fn resized_pair_weights_by_delta(delta_first: i16, first: Rect, second: Rect, first_weight: u16, second_weight: u16) -> Option<(u16, u16)> {
        let first_height = Self::u16_add_signed(first.height, delta_first);
        Self::resized_pair_weights_for_first_height(first_height, first, second, first_weight, second_weight)
    }

    fn resized_pair_weights_for_first_height(first_height: u16, first: Rect, second: Rect, first_weight: u16, second_weight: u16) -> Option<(u16, u16)> {
        if first.height == 0 || second.height == 0 {
            return None;
        }

        let pair_top = first.y;
        let pair_bottom = second.y.saturating_add(second.height);
        let pair_height = pair_bottom.saturating_sub(pair_top);
        if pair_height < 2 {
            return None;
        }

        let min_height = LAYOUT_HEIGHT_MIN_STACKED_PANE.min(pair_height / 2).max(1);
        let max_first_height = pair_height.saturating_sub(min_height);
        let first_height = first_height.clamp(min_height, max_first_height);
        let total_weight = first_weight.max(1).saturating_add(second_weight.max(1));
        if total_weight < 2 {
            return None;
        }

        let first_weight = ((first_height as u32 * total_weight as u32) / pair_height as u32).clamp(1, total_weight.saturating_sub(1) as u32) as u16;
        Some((first_weight, total_weight.saturating_sub(first_weight)))
    }
}

#[cfg(test)]
#[path = "../../tests/app/input/events.rs"]
mod tests;
