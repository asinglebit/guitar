use crate::app::{
    app::{App, Focus, Viewport},
    state::defaults::ViewerMode,
};
use crate::helpers::layout::{LAYOUT_WIDTH_MIN_CENTER, LAYOUT_WIDTH_MIN_SIDE_PANE, add_scrollbar, extend_up, inset_bottom, inset_top, load_layout_config, save_layout_config, shrink_width};
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::{Direction, Layout as RatatuiLayout, Rect};
use std::cell::Cell;

fn zero_rect() -> Rect {
    Rect { x: 0, y: 0, width: 0, height: 0 }
}

fn weighted_constraint(active: bool, weight: u16, total_weight: u16) -> Constraint {
    if active { Constraint::Ratio(weight.max(1) as u32, total_weight.max(1) as u32) } else { Constraint::Length(0) }
}

fn total_active_weight(sections: &[(bool, u16)]) -> u16 {
    sections.iter().filter_map(|(active, weight)| active.then_some((*weight).max(1))).sum::<u16>().max(1)
}

fn divider_rect(active: bool, x: u16, y: u16, width: u16) -> Rect {
    if active && width > 0 { Rect { x, y, width, height: 1 } } else { zero_rect() }
}

fn vertical_divider_rect(active: bool, x: u16, y: u16, height: u16) -> Rect {
    if active && height > 0 { Rect { x, y, width: 1, height } } else { zero_rect() }
}

pub(crate) fn viewer_split_rects(active: bool, area: Rect) -> (Rect, Rect, Rect) {
    if !active || area.width < 3 || area.height == 0 {
        return (zero_rect(), zero_rect(), zero_rect());
    }

    let chunks = RatatuiLayout::default().direction(Direction::Horizontal).constraints([Constraint::Ratio(1, 2), Constraint::Length(1), Constraint::Ratio(1, 2)]).split(area);

    (chunks[0], chunks[1], chunks[2])
}

fn side_pane_width(width: u16, total_width: u16, other_width: u16) -> u16 {
    let max_width = total_width.saturating_sub(other_width).saturating_sub(LAYOUT_WIDTH_MIN_CENTER);
    width.max(LAYOUT_WIDTH_MIN_SIDE_PANE).min(max_width.max(LAYOUT_WIDTH_MIN_SIDE_PANE))
}

fn left_stack_rects(area: Rect, has_previous_active: bool) -> (Rect, Rect) {
    let mut scrollbar = add_scrollbar(area);
    let mut content = area;
    if has_previous_active {
        scrollbar = extend_up(scrollbar, 1);
    } else {
        content = inset_top(content, 1);
    }
    (content, scrollbar)
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Layout {
    pub title_left: Rect,
    pub title_right: Rect,
    pub app: Rect,
    pub pane_left: Rect,
    pub pane_right: Rect,
    pub pane_branches: Rect,
    pub pane_tags: Rect,
    pub pane_stashes: Rect,
    pub pane_reflogs: Rect,
    pub pane_worktrees: Rect,
    pub pane_search: Rect,
    pub pane_inspector: Rect,
    pub pane_status: Rect,
    pub pane_status_top: Rect,
    pub pane_status_bottom: Rect,
    pub branches: Rect,
    pub branches_scrollbar: Rect,
    pub tags: Rect,
    pub tags_scrollbar: Rect,
    pub stashes: Rect,
    pub stashes_scrollbar: Rect,
    pub reflogs: Rect,
    pub reflogs_scrollbar: Rect,
    pub worktrees: Rect,
    pub worktrees_scrollbar: Rect,
    pub search: Rect,
    pub search_scrollbar: Rect,
    pub graph: Rect,
    pub graph_scrollbar: Rect,
    pub viewer_split_left: Rect,
    pub viewer_split_right: Rect,
    pub inspector: Rect,
    pub inspector_scrollbar: Rect,
    pub status_top: Rect,
    pub status_top_scrollbar: Rect,
    pub status_bottom: Rect,
    pub status_bottom_scrollbar: Rect,
    pub divider_left: Rect,
    pub divider_right: Rect,
    pub divider_branches_tags: Rect,
    pub divider_branches_stashes: Rect,
    pub divider_branches_worktrees: Rect,
    pub divider_branches_reflogs: Rect,
    pub divider_branches_search: Rect,
    pub divider_tags_stashes: Rect,
    pub divider_tags_worktrees: Rect,
    pub divider_tags_reflogs: Rect,
    pub divider_stashes_worktrees: Rect,
    pub divider_stashes_reflogs: Rect,
    pub divider_reflogs_worktrees: Rect,
    pub divider_tags_search: Rect,
    pub divider_stashes_search: Rect,
    pub divider_reflogs_search: Rect,
    pub divider_worktrees_search: Rect,
    pub divider_inspector_status: Rect,
    pub divider_status_files: Rect,
    pub divider_viewer_split: Rect,
    pub statusbar_left: Rect,
    pub statusbar_right: Rect,
}

impl App {
    pub fn layout(&mut self, frame: &mut Frame) {
        let is_zen = self.layout_config.is_zen;
        let is_settings = self.viewport == Viewport::Splash || self.viewport == Viewport::Settings;
        let is_inspector = !is_settings && self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts);
        let is_status = !is_settings && self.layout_config.is_status;
        let is_right_pane = is_inspector || is_status;
        let is_left_pane = (self.layout_config.is_branches
            || self.layout_config.is_tags
            || self.layout_config.is_stashes
            || self.layout_config.is_reflogs
            || self.layout_config.is_worktrees
            || self.layout_config.is_search)
            && !is_settings;
        let is_viewer_split = self.viewport == Viewport::Viewer && self.viewer_mode == ViewerMode::Split;

        // Split title, main area, and status bar before pane-specific decisions.
        let chunks_vertical = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.layout_config.is_minimal { 0 } else { 1 }), // Title bar.
                Constraint::Min(0),                                                    // Main app area.
                Constraint::Length(if self.layout_config.is_minimal { 0 } else { 1 }), // Status bar.
            ])
            .split(frame.area());

        // The right title segment is reserved for path and mode metadata.
        let chunks_title_bar = RatatuiLayout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(chunks_vertical[0]);

        // Use saved side pane widths when there is room, otherwise fall back to weighted ratios.
        let total_width = chunks_vertical[1].width;
        let width_right_pane = if is_right_pane { side_pane_width(self.layout_config.width_right_pane, total_width, 0) } else { 0 };
        let width_left_pane = if is_left_pane { side_pane_width(self.layout_config.width_left_pane, total_width, width_right_pane) } else { 0 };
        let min_required = width_left_pane + width_right_pane + LAYOUT_WIDTH_MIN_CENTER;
        let constraints = if total_width < min_required {
            let left_weight = if is_left_pane { width_left_pane.max(1) } else { 0 };
            let right_weight = if is_right_pane { width_right_pane.max(1) } else { 0 };
            let total_weight = left_weight + LAYOUT_WIDTH_MIN_CENTER + right_weight;
            [
                Constraint::Ratio(left_weight as u32, total_weight.max(1) as u32),             // Left pane.
                Constraint::Ratio(LAYOUT_WIDTH_MIN_CENTER as u32, total_weight.max(1) as u32), // Center pane.
                Constraint::Ratio(right_weight as u32, total_weight.max(1) as u32),            // Right pane.
            ]
        } else {
            [
                Constraint::Length(width_left_pane),  // Left pane.
                Constraint::Min(0),                   // Center pane.
                Constraint::Length(width_right_pane), // Right pane.
            ]
        };
        let chunks_horizontal = RatatuiLayout::default().direction(Direction::Horizontal).constraints(constraints).split(chunks_vertical[1]);

        // Inactive left sections get zero height while active sections share the column.
        let left_sections =
            [self.layout_config.is_branches, self.layout_config.is_tags, self.layout_config.is_stashes, self.layout_config.is_reflogs, self.layout_config.is_worktrees, self.layout_config.is_search];
        let left_weights = [
            self.layout_config.weight_branches,
            self.layout_config.weight_tags,
            self.layout_config.weight_stashes,
            self.layout_config.weight_reflogs,
            self.layout_config.weight_worktrees,
            self.layout_config.weight_search,
        ];
        let left_weight_total = total_active_weight(&left_sections.into_iter().zip(left_weights).collect::<Vec<_>>());
        let chunks_pane_left = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints(left_sections.into_iter().zip(left_weights).map(|(active, weight)| weighted_constraint(active, weight, left_weight_total)))
            .split(chunks_horizontal[0]);

        // Inspector and status split the right column only when both are active.
        let right_sections = [is_inspector, is_status];
        let right_weights = [self.layout_config.weight_inspector, self.layout_config.weight_status];
        let right_weight_total = total_active_weight(&right_sections.into_iter().zip(right_weights).collect::<Vec<_>>());
        let chunks_pane_right = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints(right_sections.into_iter().zip(right_weights).map(|(active, weight)| weighted_constraint(active, weight, right_weight_total)))
            .split(chunks_horizontal[2]);

        // The bottom status pane is only needed for unstaged changes on the pseudo-row.
        let status_sections = [true, self.graph_selected == 0];
        let status_weights = [self.layout_config.weight_status_top, self.layout_config.weight_status_bottom];
        let status_weight_total = total_active_weight(&status_sections.into_iter().zip(status_weights).collect::<Vec<_>>());
        let chunks_status = RatatuiLayout::default()
            .direction(Direction::Vertical)
            .constraints(status_sections.into_iter().zip(status_weights).map(|(active, weight)| weighted_constraint(active, weight, status_weight_total)))
            .split(chunks_pane_right[1]);

        // Status bar mirrors the title bar's wide-left, narrow-right split.
        let chunks_status_bar = RatatuiLayout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(80), Constraint::Percentage(20)]).split(chunks_vertical[2]);

        // Scrollbars use the untrimmed pane rectangle while content is inset below headers.
        let (branches, branches_scrollbar) = left_stack_rects(chunks_pane_left[0], false);
        let (tags, tags_scrollbar) = left_stack_rects(chunks_pane_left[1], self.layout_config.is_branches);
        let (stashes, stashes_scrollbar) = left_stack_rects(chunks_pane_left[2], self.layout_config.is_branches || self.layout_config.is_tags);
        let (reflogs, reflogs_scrollbar) = left_stack_rects(chunks_pane_left[3], self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes);
        let (worktrees, worktrees_scrollbar) =
            left_stack_rects(chunks_pane_left[4], self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs);
        let (search, search_scrollbar) = left_stack_rects(
            chunks_pane_left[5],
            self.layout_config.is_branches || self.layout_config.is_tags || self.layout_config.is_stashes || self.layout_config.is_reflogs || self.layout_config.is_worktrees,
        );

        // The graph leaves one row for its header and one for the status line.
        let graph_scrollbar = chunks_horizontal[1];
        let graph = inset_bottom(inset_top(chunks_horizontal[1], 1), 1);
        let (viewer_split_left, divider_viewer_split, viewer_split_right) = viewer_split_rects(is_viewer_split, graph);

        // Inspector content starts below its header.
        let inspector_scrollbar = chunks_pane_right[0];
        let inspector = inset_top(chunks_pane_right[0], 1);

        // Status can merge with inspector to avoid double borders between stacked panes.
        let mut status_top_scrollbar = chunks_status[0];
        let mut status_top = chunks_status[0];
        let merge_with_inspector = self.layout_config.is_inspector && (self.graph_selected != 0 || self.uncommitted.has_conflicts);
        if merge_with_inspector {
            status_top_scrollbar = extend_up(status_top_scrollbar, 1);
            status_top = extend_up(status_top, 1);
        } else {
            status_top = inset_top(status_top, 1);
        }
        status_top = shrink_width(status_top, 1);

        // Bottom status is extended upward so its scrollbar aligns with the shared border.
        let status_bottom_scrollbar = extend_up(chunks_status[1], 1);
        let mut status_bottom = extend_up(chunks_status[1], 1);
        status_bottom = shrink_width(status_bottom, 1);

        let divider_left = vertical_divider_rect(is_left_pane, chunks_horizontal[1].x, chunks_horizontal[1].y, chunks_horizontal[1].height);
        let divider_right = vertical_divider_rect(is_right_pane, chunks_horizontal[1].x + chunks_horizontal[1].width.saturating_sub(1), chunks_horizontal[1].y, chunks_horizontal[1].height);
        let divider_branches_tags =
            divider_rect(self.layout_config.is_branches && self.layout_config.is_tags, chunks_pane_left[1].x, chunks_pane_left[1].y.saturating_sub(1), chunks_pane_left[1].width);
        let divider_branches_stashes = divider_rect(
            self.layout_config.is_branches && !self.layout_config.is_tags && self.layout_config.is_stashes,
            chunks_pane_left[2].x,
            chunks_pane_left[2].y.saturating_sub(1),
            chunks_pane_left[2].width,
        );
        let divider_branches_reflogs = divider_rect(
            self.layout_config.is_branches && !self.layout_config.is_tags && !self.layout_config.is_stashes && self.layout_config.is_reflogs,
            chunks_pane_left[3].x,
            chunks_pane_left[3].y.saturating_sub(1),
            chunks_pane_left[3].width,
        );
        let divider_branches_worktrees = divider_rect(
            self.layout_config.is_branches && !self.layout_config.is_tags && !self.layout_config.is_stashes && !self.layout_config.is_reflogs && self.layout_config.is_worktrees,
            chunks_pane_left[4].x,
            chunks_pane_left[4].y.saturating_sub(1),
            chunks_pane_left[4].width,
        );
        let divider_branches_search = divider_rect(
            self.layout_config.is_branches
                && !self.layout_config.is_tags
                && !self.layout_config.is_stashes
                && !self.layout_config.is_reflogs
                && !self.layout_config.is_worktrees
                && self.layout_config.is_search,
            chunks_pane_left[5].x,
            chunks_pane_left[5].y.saturating_sub(1),
            chunks_pane_left[5].width,
        );
        let divider_tags_stashes = divider_rect(self.layout_config.is_tags && self.layout_config.is_stashes, chunks_pane_left[2].x, chunks_pane_left[2].y.saturating_sub(1), chunks_pane_left[2].width);
        let divider_tags_reflogs = divider_rect(
            self.layout_config.is_tags && !self.layout_config.is_stashes && self.layout_config.is_reflogs,
            chunks_pane_left[3].x,
            chunks_pane_left[3].y.saturating_sub(1),
            chunks_pane_left[3].width,
        );
        let divider_tags_worktrees = divider_rect(
            self.layout_config.is_tags && !self.layout_config.is_stashes && !self.layout_config.is_reflogs && self.layout_config.is_worktrees,
            chunks_pane_left[4].x,
            chunks_pane_left[4].y.saturating_sub(1),
            chunks_pane_left[4].width,
        );
        let divider_tags_search = divider_rect(
            self.layout_config.is_tags && !self.layout_config.is_stashes && !self.layout_config.is_reflogs && !self.layout_config.is_worktrees && self.layout_config.is_search,
            chunks_pane_left[5].x,
            chunks_pane_left[5].y.saturating_sub(1),
            chunks_pane_left[5].width,
        );
        let divider_stashes_reflogs =
            divider_rect(self.layout_config.is_stashes && self.layout_config.is_reflogs, chunks_pane_left[3].x, chunks_pane_left[3].y.saturating_sub(1), chunks_pane_left[3].width);
        let divider_stashes_worktrees = divider_rect(
            self.layout_config.is_stashes && !self.layout_config.is_reflogs && self.layout_config.is_worktrees,
            chunks_pane_left[4].x,
            chunks_pane_left[4].y.saturating_sub(1),
            chunks_pane_left[4].width,
        );
        let divider_stashes_search = divider_rect(
            self.layout_config.is_stashes && !self.layout_config.is_reflogs && !self.layout_config.is_worktrees && self.layout_config.is_search,
            chunks_pane_left[5].x,
            chunks_pane_left[5].y.saturating_sub(1),
            chunks_pane_left[5].width,
        );
        let divider_reflogs_worktrees =
            divider_rect(self.layout_config.is_reflogs && self.layout_config.is_worktrees, chunks_pane_left[4].x, chunks_pane_left[4].y.saturating_sub(1), chunks_pane_left[4].width);
        let divider_reflogs_search = divider_rect(
            self.layout_config.is_reflogs && !self.layout_config.is_worktrees && self.layout_config.is_search,
            chunks_pane_left[5].x,
            chunks_pane_left[5].y.saturating_sub(1),
            chunks_pane_left[5].width,
        );
        let divider_worktrees_search =
            divider_rect(self.layout_config.is_worktrees && self.layout_config.is_search, chunks_pane_left[5].x, chunks_pane_left[5].y.saturating_sub(1), chunks_pane_left[5].width);
        let divider_inspector_status = divider_rect(is_inspector && is_status, chunks_pane_right[1].x, chunks_pane_right[1].y.saturating_sub(1), chunks_pane_right[1].width);
        let divider_status_files = divider_rect(is_status && self.graph_selected == 0, chunks_status[1].x, chunks_status[1].y.saturating_sub(1), chunks_status[1].width);

        if is_zen {
            let zen = chunks_vertical[1];
            let zero = zero_rect();

            let graph = if matches!(
                self.focus,
                Focus::Viewport
                    | Focus::ModalCheckout
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
            ) {
                zen
            } else {
                zero
            };
            let (viewer_split_left, divider_viewer_split, viewer_split_right) = viewer_split_rects(is_viewer_split && graph.width > 0, graph);

            self.layout = Layout {
                // Title and status keep their normal positions in zen mode.
                app: zen,
                title_left: chunks_title_bar[0],
                title_right: chunks_title_bar[1],
                statusbar_left: chunks_status_bar[0],
                statusbar_right: chunks_status_bar[1],
                pane_left: zero,
                pane_right: zero,
                pane_branches: if matches!(self.focus, Focus::Branches) { zen } else { zero },
                pane_tags: if matches!(self.focus, Focus::Tags) { zen } else { zero },
                pane_stashes: if matches!(self.focus, Focus::Stashes) { zen } else { zero },
                pane_reflogs: if matches!(self.focus, Focus::Reflogs) { zen } else { zero },
                pane_worktrees: if matches!(self.focus, Focus::Worktrees) { zen } else { zero },
                pane_search: if matches!(self.focus, Focus::Search) { zen } else { zero },
                pane_inspector: if matches!(self.focus, Focus::Inspector) { zen } else { zero },
                pane_status: zero,
                pane_status_top: if matches!(self.focus, Focus::StatusTop) { zen } else { zero },
                pane_status_bottom: if matches!(self.focus, Focus::StatusBottom) { zen } else { zero },

                // Only the focused pane receives the entire main rectangle.
                branches: if matches!(self.focus, Focus::Branches) { zen } else { zero },
                tags: if matches!(self.focus, Focus::Tags) { zen } else { zero },
                stashes: if matches!(self.focus, Focus::Stashes) { zen } else { zero },
                reflogs: if matches!(self.focus, Focus::Reflogs) { zen } else { zero },
                worktrees: if matches!(self.focus, Focus::Worktrees) { zen } else { zero },
                search: if matches!(self.focus, Focus::Search) { zen } else { zero },
                graph,
                viewer_split_left,
                viewer_split_right,
                inspector: if matches!(self.focus, Focus::Inspector) { zen } else { zero },
                status_top: if matches!(self.focus, Focus::StatusTop) { zen } else { zero },
                status_bottom: if matches!(self.focus, Focus::StatusBottom) { zen } else { zero },

                // Scrollbar rectangles mirror their pane visibility in zen mode.
                branches_scrollbar: if matches!(self.focus, Focus::Branches) { zen } else { zero },
                tags_scrollbar: if matches!(self.focus, Focus::Tags) { zen } else { zero },
                stashes_scrollbar: if matches!(self.focus, Focus::Stashes) { zen } else { zero },
                reflogs_scrollbar: if matches!(self.focus, Focus::Reflogs) { zen } else { zero },
                worktrees_scrollbar: if matches!(self.focus, Focus::Worktrees) { zen } else { zero },
                search_scrollbar: if matches!(self.focus, Focus::Search) { zen } else { zero },
                graph_scrollbar: if matches!(
                    self.focus,
                    Focus::Viewport
                        | Focus::ModalCheckout
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
                ) {
                    zen
                } else {
                    zero
                },
                inspector_scrollbar: if matches!(self.focus, Focus::Inspector) { zen } else { zero },
                status_top_scrollbar: if matches!(self.focus, Focus::StatusTop) { zen } else { zero },
                status_bottom_scrollbar: if matches!(self.focus, Focus::StatusBottom) { zen } else { zero },

                divider_left: zero,
                divider_right: zero,
                divider_branches_tags: zero,
                divider_branches_stashes: zero,
                divider_branches_worktrees: zero,
                divider_branches_reflogs: zero,
                divider_branches_search: zero,
                divider_tags_stashes: zero,
                divider_tags_worktrees: zero,
                divider_tags_reflogs: zero,
                divider_stashes_worktrees: zero,
                divider_stashes_reflogs: zero,
                divider_reflogs_worktrees: zero,
                divider_tags_search: zero,
                divider_stashes_search: zero,
                divider_reflogs_search: zero,
                divider_worktrees_search: zero,
                divider_inspector_status: zero,
                divider_status_files: zero,
                divider_viewer_split,
            };

            return;
        }

        self.layout = Layout {
            // Top chrome.
            title_left: chunks_title_bar[0],
            title_right: chunks_title_bar[1],

            // Outer main area used for the app border.
            app: chunks_vertical[1],
            pane_left: chunks_horizontal[0],
            pane_right: chunks_horizontal[2],
            pane_branches: chunks_pane_left[0],
            pane_tags: chunks_pane_left[1],
            pane_stashes: chunks_pane_left[2],
            pane_reflogs: chunks_pane_left[3],
            pane_worktrees: chunks_pane_left[4],
            pane_search: chunks_pane_left[5],
            pane_inspector: chunks_pane_right[0],
            pane_status: chunks_pane_right[1],
            pane_status_top: chunks_status[0],
            pane_status_bottom: chunks_status[1],

            // Pane content and scrollbar rectangles.
            branches,
            branches_scrollbar,
            tags,
            tags_scrollbar,
            stashes,
            stashes_scrollbar,
            reflogs,
            reflogs_scrollbar,
            worktrees,
            worktrees_scrollbar,
            search,
            search_scrollbar,
            graph,
            graph_scrollbar,
            viewer_split_left,
            viewer_split_right,
            inspector,
            inspector_scrollbar,
            status_top,
            status_top_scrollbar,
            status_bottom,
            status_bottom_scrollbar,
            divider_left,
            divider_right,
            divider_branches_tags,
            divider_branches_stashes,
            divider_branches_worktrees,
            divider_branches_reflogs,
            divider_branches_search,
            divider_tags_stashes,
            divider_tags_worktrees,
            divider_tags_reflogs,
            divider_stashes_worktrees,
            divider_stashes_reflogs,
            divider_reflogs_worktrees,
            divider_tags_search,
            divider_stashes_search,
            divider_reflogs_search,
            divider_worktrees_search,
            divider_inspector_status,
            divider_status_files,
            divider_viewer_split,

            // Bottom chrome.
            statusbar_left: chunks_status_bar[0],
            statusbar_right: chunks_status_bar[1],
        }
    }

    pub fn trap_selection(&self, selected: usize, scroll: &Cell<usize>, total_lines: usize, visible_height: usize) {
        if visible_height == 0 || total_lines == 0 {
            scroll.set(0);
            return;
        }

        // Maximum scroll offset still leaves a full viewport when there are enough rows.
        let max_scroll = total_lines.saturating_sub(visible_height);

        // Clamp both scroll and selection before comparing them.
        let mut scroll_val = scroll.get().min(max_scroll);
        let sel = selected.min(total_lines.saturating_sub(1));

        // Move the viewport up when the selected row is above it.
        if sel < scroll_val {
            scroll_val = sel;
            scroll.set(scroll_val);
            return;
        }

        // Move the viewport down until the selected row is the last visible line.
        if sel >= scroll_val + visible_height {
            let desired = sel.saturating_sub(visible_height).saturating_add(1);
            scroll_val = desired.min(max_scroll);
            scroll.set(scroll_val);
            return;
        }

        // Selection is already visible, so only the clamped value matters.
        scroll.set(scroll_val);
    }

    pub fn save_layout(&self) {
        save_layout_config(&self.layout_config.normalized());
    }

    pub fn load_layout(&mut self) {
        self.layout_config = load_layout_config();
    }
}
