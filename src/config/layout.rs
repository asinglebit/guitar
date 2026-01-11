use std::{fs, path::PathBuf};

use facet::Facet;

fn layout_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("layout.json");
    path
}

#[derive(Facet, Default, Clone, Copy)]
pub struct LayoutConfig {
    pub is_shas: bool,
    pub is_minimal: bool,
    pub is_branches: bool,
    pub is_tags: bool,
    pub is_stashes: bool,
    pub is_status: bool,
    pub is_inspector: bool,
    pub is_zen: bool,
}

pub fn load_layout_config() -> LayoutConfig {
    let path = layout_path();
    if path.exists() {
        let contents = fs::read_to_string(&path).unwrap();
        facet_json::from_str(&contents).unwrap_or_default()
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
