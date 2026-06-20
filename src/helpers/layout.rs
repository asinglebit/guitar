use facet::Facet;
use ratatui::layout::Rect;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const LAYOUT_WIDTH_LEFT_PANE: u16 = 45;
pub const LAYOUT_WIDTH_RIGHT_PANE: u16 = 46;
pub const LAYOUT_WIDTH_MIN_CENTER: u16 = 20;
pub const LAYOUT_WIDTH_MIN_SIDE_PANE: u16 = 16;
pub const LAYOUT_HEIGHT_MIN_STACKED_PANE: u16 = 3;
pub const LAYOUT_WEIGHT_DEFAULT: u16 = 100;
pub const GRAPH_LANE_LIMIT_DEFAULT: usize = 50;

pub fn inset_top(mut r: Rect, n: u16) -> Rect {
    r.y += n;
    r.height = r.height.saturating_sub(n);
    r
}

pub fn inset_bottom(mut r: Rect, n: u16) -> Rect {
    r.height = r.height.saturating_sub(n);
    r
}

pub fn add_scrollbar(mut r: Rect) -> Rect {
    r.width += 1;
    r
}

pub fn extend_up(mut r: Rect, n: u16) -> Rect {
    r.y = r.y.saturating_sub(n);
    r.height += n;
    r
}

pub fn shrink_width(mut r: Rect, n: u16) -> Rect {
    r.width = r.width.saturating_sub(n);
    r
}

pub fn scrollbar_content_length(total_lines: usize, visible_height: usize) -> usize {
    if total_lines > visible_height { total_lines.saturating_sub(visible_height).saturating_add(1) } else { 0 }
}

fn layout_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("layout.json");
    path
}

#[derive(Facet, Clone, Copy)]
pub struct LayoutConfig {
    pub is_shas: bool,
    pub is_minimal: bool,
    pub is_branches: bool,
    pub is_tags: bool,
    pub is_stashes: bool,
    #[facet(default = false)]
    pub is_reflogs: bool,
    #[facet(default = false)]
    pub is_search: bool,
    #[facet(default = true)]
    pub is_graph_reflogs: bool,
    #[facet(default = false)]
    pub is_graph_dates: bool,
    #[facet(default = false)]
    pub is_graph_committers: bool,
    #[facet(default = true)]
    pub is_graph_refs: bool,
    #[facet(default = false)]
    pub is_worktrees: bool,
    #[facet(default = false)]
    pub is_submodules: bool,
    pub is_status: bool,
    pub is_inspector: bool,
    pub is_zen: bool,
    #[facet(default = LAYOUT_WIDTH_LEFT_PANE)]
    pub width_left_pane: u16,
    #[facet(default = LAYOUT_WIDTH_RIGHT_PANE)]
    pub width_right_pane: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_branches: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_tags: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_stashes: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_reflogs: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_worktrees: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_submodules: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_search: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_inspector: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_status: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_status_top: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_status_bottom: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_viewer_split_left: u16,
    #[facet(default = LAYOUT_WEIGHT_DEFAULT)]
    pub weight_viewer_split_right: u16,
    #[facet(default = GRAPH_LANE_LIMIT_DEFAULT)]
    pub graph_lane_limit: usize,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            is_shas: true,
            is_minimal: false,
            is_branches: true,
            is_tags: false,
            is_stashes: false,
            is_reflogs: false,
            is_search: false,
            is_graph_reflogs: false,
            is_graph_dates: false,
            is_graph_committers: false,
            is_graph_refs: true,
            is_worktrees: false,
            is_submodules: false,
            is_status: true,
            is_inspector: true,
            is_zen: false,
            width_left_pane: LAYOUT_WIDTH_LEFT_PANE,
            width_right_pane: LAYOUT_WIDTH_RIGHT_PANE,
            weight_branches: LAYOUT_WEIGHT_DEFAULT,
            weight_tags: LAYOUT_WEIGHT_DEFAULT,
            weight_stashes: LAYOUT_WEIGHT_DEFAULT,
            weight_reflogs: LAYOUT_WEIGHT_DEFAULT,
            weight_worktrees: LAYOUT_WEIGHT_DEFAULT,
            weight_submodules: LAYOUT_WEIGHT_DEFAULT,
            weight_search: LAYOUT_WEIGHT_DEFAULT,
            weight_inspector: LAYOUT_WEIGHT_DEFAULT,
            weight_status: LAYOUT_WEIGHT_DEFAULT,
            weight_status_top: LAYOUT_WEIGHT_DEFAULT,
            weight_status_bottom: LAYOUT_WEIGHT_DEFAULT,
            weight_viewer_split_left: LAYOUT_WEIGHT_DEFAULT,
            weight_viewer_split_right: LAYOUT_WEIGHT_DEFAULT,
            graph_lane_limit: GRAPH_LANE_LIMIT_DEFAULT,
        }
    }
}

impl LayoutConfig {
    pub fn normalized(mut self) -> Self {
        self.width_left_pane = self.width_left_pane.max(LAYOUT_WIDTH_MIN_SIDE_PANE);
        self.width_right_pane = self.width_right_pane.max(LAYOUT_WIDTH_MIN_SIDE_PANE);
        self.weight_branches = self.weight_branches.max(1);
        self.weight_tags = self.weight_tags.max(1);
        self.weight_stashes = self.weight_stashes.max(1);
        self.weight_reflogs = self.weight_reflogs.max(1);
        self.weight_worktrees = self.weight_worktrees.max(1);
        self.weight_submodules = self.weight_submodules.max(1);
        self.weight_search = self.weight_search.max(1);
        self.weight_inspector = self.weight_inspector.max(1);
        self.weight_status = self.weight_status.max(1);
        self.weight_status_top = self.weight_status_top.max(1);
        self.weight_status_bottom = self.weight_status_bottom.max(1);
        self.weight_viewer_split_left = self.weight_viewer_split_left.max(1);
        self.weight_viewer_split_right = self.weight_viewer_split_right.max(1);
        self.graph_lane_limit = self.graph_lane_limit.max(1);
        self
    }
}

pub fn load_layout_config() -> LayoutConfig {
    let path = layout_path();
    if path.exists() {
        let contents = fs::read_to_string(&path).unwrap();
        facet_json::from_str::<LayoutConfig>(&contents).unwrap_or_default().normalized()
    } else {
        let config = LayoutConfig::default();
        save_layout_config(&config);
        config
    }
}

pub fn save_layout_config(config: &LayoutConfig) {
    let path = layout_path();
    save_layout_config_to_path(&path, config);
}

fn save_layout_config_to_path(path: &Path, config: &LayoutConfig) {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(config_string) = facet_json::to_string_pretty(config) {
        let _ = fs::write(path, &config_string);
    }
}

#[cfg(test)]
#[path = "../tests/helpers/layout.rs"]
mod tests;
