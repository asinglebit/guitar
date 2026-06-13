use facet::Facet;
use ratatui::layout::Rect;
use std::{fs, path::PathBuf};

pub const LAYOUT_WIDTH_LEFT_PANE: u16 = 45;
pub const LAYOUT_WIDTH_RIGHT_PANE: u16 = 46;
pub const LAYOUT_WIDTH_MIN_CENTER: u16 = 20;
pub const LAYOUT_WIDTH_MIN_SIDE_PANE: u16 = 16;
pub const LAYOUT_WIDTH_MIN_SPLIT_PANE: u16 = 10;
pub const LAYOUT_HEIGHT_MIN_STACKED_PANE: u16 = 3;
pub const LAYOUT_WEIGHT_DEFAULT: u16 = 100;

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
    pub is_worktrees: bool,
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
    pub weight_worktrees: u16,
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
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            is_shas: false,
            is_minimal: false,
            is_branches: false,
            is_tags: false,
            is_stashes: false,
            is_worktrees: false,
            is_status: false,
            is_inspector: false,
            is_zen: false,
            width_left_pane: LAYOUT_WIDTH_LEFT_PANE,
            width_right_pane: LAYOUT_WIDTH_RIGHT_PANE,
            weight_branches: LAYOUT_WEIGHT_DEFAULT,
            weight_tags: LAYOUT_WEIGHT_DEFAULT,
            weight_stashes: LAYOUT_WEIGHT_DEFAULT,
            weight_worktrees: LAYOUT_WEIGHT_DEFAULT,
            weight_inspector: LAYOUT_WEIGHT_DEFAULT,
            weight_status: LAYOUT_WEIGHT_DEFAULT,
            weight_status_top: LAYOUT_WEIGHT_DEFAULT,
            weight_status_bottom: LAYOUT_WEIGHT_DEFAULT,
            weight_viewer_split_left: LAYOUT_WEIGHT_DEFAULT,
            weight_viewer_split_right: LAYOUT_WEIGHT_DEFAULT,
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
        self.weight_worktrees = self.weight_worktrees.max(1);
        self.weight_inspector = self.weight_inspector.max(1);
        self.weight_status = self.weight_status.max(1);
        self.weight_status_top = self.weight_status_top.max(1);
        self.weight_status_bottom = self.weight_status_bottom.max(1);
        self.weight_viewer_split_left = self.weight_viewer_split_left.max(1);
        self.weight_viewer_split_right = self.weight_viewer_split_right.max(1);
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
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    let config_string = facet_json::to_string(config).unwrap();
    fs::write(&path, &config_string).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_config_reads_old_boolean_only_config() {
        let old_config = r#"{"is_shas":false,"is_minimal":false,"is_branches":true,"is_tags":true,"is_stashes":false,"is_status":true,"is_inspector":true,"is_zen":false}"#;

        let config = facet_json::from_str::<LayoutConfig>(old_config).unwrap().normalized();

        assert!(config.is_branches);
        assert!(config.is_tags);
        assert!(config.is_status);
        assert!(!config.is_worktrees);
        assert_eq!(config.width_left_pane, LAYOUT_WIDTH_LEFT_PANE);
        assert_eq!(config.width_right_pane, LAYOUT_WIDTH_RIGHT_PANE);
        assert_eq!(config.weight_branches, LAYOUT_WEIGHT_DEFAULT);
        assert_eq!(config.weight_status_bottom, LAYOUT_WEIGHT_DEFAULT);
        assert_eq!(config.weight_worktrees, LAYOUT_WEIGHT_DEFAULT);
        assert_eq!(config.weight_viewer_split_left, LAYOUT_WEIGHT_DEFAULT);
        assert_eq!(config.weight_viewer_split_right, LAYOUT_WEIGHT_DEFAULT);
    }
}
