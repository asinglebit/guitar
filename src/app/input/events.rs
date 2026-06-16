use crate::{
    app::app::{App, Direction, Focus, LayoutDrag, MouseSelectionTarget, SettingsSelectionKind, Viewport},
    helpers::layout::{LAYOUT_HEIGHT_MIN_STACKED_PANE, LAYOUT_WIDTH_MIN_CENTER, LAYOUT_WIDTH_MIN_SIDE_PANE},
};
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};
use std::{
    io,
    time::{Duration, Instant},
};

const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);

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
    Inspector,
    Status,
    StatusTop,
    StatusBottom,
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
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_mouse_down(mouse_event.column, mouse_event.row);
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag) = self.layout_drag {
                    self.apply_layout_drag(drag, mouse_event.column, mouse_event.row);
                }
            },
            MouseEventKind::Up(MouseButton::Left) => {
                if self.layout_drag.take().is_some() {
                    self.save_layout();
                    self.mark_viewer_layout_dirty();
                }
            },
            MouseEventKind::ScrollUp => {
                self.handle_mouse_scroll(mouse_event.column, mouse_event.row, Direction::Up);
            },
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(mouse_event.column, mouse_event.row, Direction::Down);
            },
            _ => {},
        }
    }

    fn handle_mouse_down(&mut self, column: u16, row: u16) {
        if let Some(drag) = self.layout_drag_at(column, row) {
            self.layout_drag = Some(drag);
            self.last_mouse_click = None;
            return;
        }

        self.layout_drag = None;

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
                | MouseSelectionTarget::StatusTop(_)
                | MouseSelectionTarget::StatusBottom(_)
                | MouseSelectionTarget::Settings(_)
        )
    }

    fn select_mouse_target(&mut self, target: MouseSelectionTarget) {
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
        }
    }

    fn mouse_selection_target_at(&self, column: u16, row: u16) -> Option<MouseSelectionTarget> {
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

    fn handle_mouse_scroll(&mut self, column: u16, row: u16, direction: Direction) {
        if self.layout_drag.is_some() {
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
        if Self::rect_contains(self.layout.divider_tags_stashes, column, row) {
            return Some(LayoutDrag::TagsStashes);
        }
        if Self::rect_contains(self.layout.divider_tags_reflogs, column, row) {
            return Some(LayoutDrag::TagsReflogs);
        }
        if Self::rect_contains(self.layout.divider_tags_worktrees, column, row) {
            return Some(LayoutDrag::TagsWorktrees);
        }
        if Self::rect_contains(self.layout.divider_stashes_reflogs, column, row) {
            return Some(LayoutDrag::StashesReflogs);
        }
        if Self::rect_contains(self.layout.divider_stashes_worktrees, column, row) {
            return Some(LayoutDrag::StashesWorktrees);
        }
        if Self::rect_contains(self.layout.divider_reflogs_worktrees, column, row) {
            return Some(LayoutDrag::ReflogsWorktrees);
        }
        if Self::rect_contains(self.layout.divider_inspector_status, column, row) {
            return Some(LayoutDrag::InspectorStatus);
        }
        if Self::rect_contains(self.layout.divider_status_files, column, row) {
            return Some(LayoutDrag::StatusFiles);
        }

        None
    }

    fn is_modal_focus(&self) -> bool {
        matches!(
            self.focus,
            Focus::ModalCheckout
                | Focus::ModalSolo
                | Focus::ModalCommit
                | Focus::ModalCherrypick
                | Focus::ModalRevert
                | Focus::ModalCreateBranch
                | Focus::ModalCreateWorktreeName
                | Focus::ModalCreateWorktreePath
                | Focus::ModalDeleteBranch
                | Focus::ModalWorktreeChooser
                | Focus::ModalRemoveWorktree
                | Focus::ModalLockWorktree
                | Focus::ModalGrep
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
                        Focus::Branches | Focus::Tags | Focus::Stashes | Focus::Reflogs | Focus::Worktrees | Focus::Viewport => self.resize_left_column_by(-1),
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
                        Focus::Branches | Focus::Tags | Focus::Stashes | Focus::Reflogs | Focus::Worktrees => self.resize_left_column_by(1),
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
        if !matches!(focused, StackPane::Branches | StackPane::Tags | StackPane::Stashes | StackPane::Reflogs | StackPane::Worktrees) {
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
        stack
    }

    fn focus_stack_pane(&self) -> Option<StackPane> {
        match self.focus {
            Focus::Branches => Some(StackPane::Branches),
            Focus::Tags => Some(StackPane::Tags),
            Focus::Stashes => Some(StackPane::Stashes),
            Focus::Reflogs => Some(StackPane::Reflogs),
            Focus::Worktrees => Some(StackPane::Worktrees),
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
            LayoutDrag::StashesWorktrees => {
                if let Some((stashes, worktrees)) =
                    Self::resized_pair_weights(row, self.layout.pane_stashes, self.layout.pane_worktrees, self.layout_config.weight_stashes, self.layout_config.weight_worktrees)
                {
                    self.layout_config.weight_stashes = stashes;
                    self.layout_config.weight_worktrees = worktrees;
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
            LayoutDrag::ReflogsWorktrees => {
                if let Some((reflogs, worktrees)) =
                    Self::resized_pair_weights(row, self.layout.pane_reflogs, self.layout.pane_worktrees, self.layout_config.weight_reflogs, self.layout_config.weight_worktrees)
                {
                    self.layout_config.weight_reflogs = reflogs;
                    self.layout_config.weight_worktrees = worktrees;
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
