use super::*;
use crate::git::test_support::temp_json_path;
use std::{fs, path::PathBuf};

fn temp_recent_path(name: &str) -> PathBuf {
    temp_json_path("guitar-recent", name)
}

#[test]
fn save_recent_writes_pretty_json_and_round_trips() {
    let path = temp_recent_path("pretty");
    let recent = vec!["/repo/a".to_string(), "/repo/b".to_string()];

    save_recent_to_path(&path, &recent);

    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.contains('\n'), "{contents}");
    assert!(contents.contains("\n  \"/repo/a\""), "{contents}");

    let loaded = facet_json::from_str::<Vec<String>>(&contents).unwrap();
    assert_eq!(loaded, recent);
}
