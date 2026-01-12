use std::{fs, path::PathBuf};

fn layout_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("recent.json");
    path
}

pub fn load_recent() -> Vec<String> {
    let path = layout_path();
    if path.exists() {
        let contents = fs::read_to_string(&path).unwrap();
        facet_json::from_str(&contents).unwrap_or_default()
    } else {
        let recent = Vec::new();
        save_recent(&recent);
        recent
    }
}

pub fn save_recent(recent: &Vec<String>) {
    let path = layout_path();
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    let recent_string = facet_json::to_string(recent).unwrap();
    fs::write(&path, &recent_string).unwrap();
}
