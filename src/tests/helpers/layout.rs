use super::*;
use std::path::PathBuf;

#[path = "test_support.rs"]
mod test_support;
use test_support::{read_to_string, temp_json_path};

fn temp_layout_path(name: &str) -> PathBuf {
    temp_json_path("guitar-layout", name)
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
    assert!(!config.is_graph_dates);
    assert!(!config.is_graph_committers);
    assert!(config.is_graph_refs);
    assert!(!config.is_worktrees);
    assert!(!config.is_submodules);
    assert!(!config.is_search);
    assert!(!config.is_zen);
    assert_eq!(config.graph_lane_limit, GRAPH_LANE_LIMIT_DEFAULT);
}

#[test]
fn save_layout_config_writes_pretty_json_and_round_trips() {
    let path = temp_layout_path("pretty");
    let config = LayoutConfig { is_shas: false, is_tags: true, width_left_pane: 52, weight_status: 3, graph_lane_limit: 12, ..Default::default() };

    save_layout_config_to_path(&path, &config);

    let contents = read_to_string(&path);
    assert!(contents.contains('\n'), "{contents}");
    assert!(contents.contains("\n  \"is_shas\""), "{contents}");
    assert!(contents.contains("\n  \"width_left_pane\""), "{contents}");
    assert!(contents.contains("\n  \"graph_lane_limit\""), "{contents}");

    let loaded = facet_json::from_str::<LayoutConfig>(&contents).unwrap();
    assert!(!loaded.is_shas);
    assert!(loaded.is_tags);
    assert_eq!(loaded.width_left_pane, 52);
    assert_eq!(loaded.weight_status, 3);
    assert_eq!(loaded.graph_lane_limit, 12);
}

#[test]
fn layout_config_normalizes_graph_lane_limit_to_positive_value() {
    let config = LayoutConfig { graph_lane_limit: 0, ..Default::default() }.normalized();

    assert_eq!(config.graph_lane_limit, 1);
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
