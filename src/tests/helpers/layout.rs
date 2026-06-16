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
    assert!(!config.is_reflogs);
    assert!(config.is_graph_reflogs);
    assert_eq!(config.weight_reflogs, LAYOUT_WEIGHT_DEFAULT);
    assert_eq!(config.weight_worktrees, LAYOUT_WEIGHT_DEFAULT);
    assert!(!config.is_search);
    assert_eq!(config.weight_search, LAYOUT_WEIGHT_DEFAULT);
    assert_eq!(config.weight_viewer_split_left, LAYOUT_WEIGHT_DEFAULT);
    assert_eq!(config.weight_viewer_split_right, LAYOUT_WEIGHT_DEFAULT);
}

#[test]
fn default_layout_shows_primary_workflow_panes() {
    let config = LayoutConfig::default();

    assert!(config.is_branches);
    assert!(config.is_status);
    assert!(config.is_inspector);
    assert!(config.is_shas);
    assert!(!config.is_tags);
    assert!(!config.is_stashes);
    assert!(!config.is_reflogs);
    assert!(!config.is_graph_reflogs);
    assert!(!config.is_worktrees);
    assert!(!config.is_search);
    assert!(!config.is_zen);
}

#[test]
fn split_viewer_divider_is_centered() {
    let (left, divider, right) = crate::app::state::layout::viewer_split_rects(true, ratatui::layout::Rect::new(4, 2, 41, 10));

    assert_eq!(divider.width, 1);
    assert_eq!(left.x, 4);
    assert_eq!(divider.x, 24);
    assert_eq!(right.x, 25);
    assert!(left.width.abs_diff(right.width) <= 1);
}
