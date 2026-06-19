use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_recent_path(name: &str) -> PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("guitar-recent-{name}-{id}.json"))
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
