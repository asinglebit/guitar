use crate::{
    app::app::{App, Direction, Focus, LayoutDrag, Viewport},
    helpers::layout::{LAYOUT_HEIGHT_MIN_STACKED_PANE, LAYOUT_WIDTH_MIN_CENTER, LAYOUT_WIDTH_MIN_SIDE_PANE, LAYOUT_WIDTH_MIN_SPLIT_PANE},
};
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind, MouseButton, MouseEvent, MouseEventKind},
    layout::Rect,
};
use std::io;

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
                self.layout_drag = self.layout_drag_at(mouse_event.column, mouse_event.row);
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

        if Self::rect_contains(self.layout.divider_viewer_split, column, row) {
            return Some(LayoutDrag::ViewerSplit);
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
        if Self::rect_contains(self.layout.divider_branches_worktrees, column, row) {
            return Some(LayoutDrag::BranchesWorktrees);
        }
        if Self::rect_contains(self.layout.divider_tags_stashes, column, row) {
            return Some(LayoutDrag::TagsStashes);
        }
        if Self::rect_contains(self.layout.divider_tags_worktrees, column, row) {
            return Some(LayoutDrag::TagsWorktrees);
        }
        if Self::rect_contains(self.layout.divider_stashes_worktrees, column, row) {
            return Some(LayoutDrag::StashesWorktrees);
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
                | Focus::ModalOperationProgress
                | Focus::ModalOperationConflict
                | Focus::ModalOperationSuccess
                | Focus::ModalError
        )
    }

    fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
        rect.width > 0 && rect.height > 0 && column >= rect.x && column < rect.x.saturating_add(rect.width) && row >= rect.y && row < rect.y.saturating_add(rect.height)
    }

    fn apply_layout_drag(&mut self, drag: LayoutDrag, column: u16, row: u16) {
        match drag {
            LayoutDrag::LeftPane => self.resize_left_pane(column),
            LayoutDrag::RightPane => self.resize_right_pane(column),
            LayoutDrag::ViewerSplit => {
                if let Some((left, right)) = Self::resized_horizontal_pair_weights(
                    column,
                    self.layout.viewer_split_left,
                    self.layout.viewer_split_right,
                    self.layout_config.weight_viewer_split_left,
                    self.layout_config.weight_viewer_split_right,
                ) {
                    self.layout_config.weight_viewer_split_left = left;
                    self.layout_config.weight_viewer_split_right = right;
                }
            },
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
        let first_height = row.saturating_sub(pair_top).saturating_add(1).clamp(min_height, max_first_height);
        let total_weight = first_weight.max(1).saturating_add(second_weight.max(1));
        if total_weight < 2 {
            return None;
        }

        let first_weight = ((first_height as u32 * total_weight as u32) / pair_height as u32).clamp(1, total_weight.saturating_sub(1) as u32) as u16;
        Some((first_weight, total_weight.saturating_sub(first_weight)))
    }

    fn resized_horizontal_pair_weights(column: u16, first: Rect, second: Rect, first_weight: u16, second_weight: u16) -> Option<(u16, u16)> {
        if first.width == 0 || second.width == 0 {
            return None;
        }

        let pair_left = first.x;
        let pair_right = second.x.saturating_add(second.width);
        let pair_width = pair_right.saturating_sub(pair_left);
        if pair_width < 2 {
            return None;
        }

        let min_width = LAYOUT_WIDTH_MIN_SPLIT_PANE.min(pair_width / 2).max(1);
        let max_first_width = pair_width.saturating_sub(min_width);
        let first_width = column.saturating_sub(pair_left).clamp(min_width, max_first_width);
        let total_weight = first_weight.max(1).saturating_add(second_weight.max(1));
        if total_weight < 2 {
            return None;
        }

        let first_weight = ((first_width as u32 * total_weight as u32) / pair_width as u32).clamp(1, total_weight.saturating_sub(1) as u32) as u16;
        Some((first_weight, total_weight.saturating_sub(first_weight)))
    }
}
