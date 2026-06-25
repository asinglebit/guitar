use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn temp_json_path(prefix: &str, name: &str) -> PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{name}-{id}.json"))
}

pub fn read_to_string(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}
