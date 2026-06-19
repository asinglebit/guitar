use super::*;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_language_path(name: &str) -> std::path::PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("guitar-language-{name}-{id}.json"))
}

#[test]
fn language_ids_and_aliases_parse() {
    assert_eq!(Language::from_id("english"), Some(Language::English));
    assert_eq!(Language::from_id("Español"), Some(Language::Spanish));
    assert_eq!(Language::from_id("fr_FR"), Some(Language::French));
    assert_eq!(Language::from_id("русский"), Some(Language::Russian));
    assert_eq!(Language::from_id("Türkçe"), Some(Language::Turkish));
    assert_eq!(Language::from_id("zh-Hans"), None);
    assert_eq!(Language::from_id("mandarin"), None);
    assert_eq!(Language::from_id("klingon"), None);
}

#[test]
fn language_save_and_load_uses_json_string() {
    let path = temp_language_path("save-load");

    save_language_to_path(&path, Language::Spanish).unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "\"spanish\"");
    assert_eq!(load_language_from_path(&path), Language::Spanish);
}

#[test]
fn invalid_language_file_falls_back_to_english() {
    let path = temp_language_path("invalid");
    fs::write(&path, "\"nope\"").unwrap();

    assert_eq!(load_language_from_path(&path), Language::English);
}

#[test]
fn active_language_changes_localised_text() {
    set_active_language(Language::English);
    assert_eq!(menu::SETTINGS(), "Settings");

    set_active_language(Language::Spanish);
    assert_eq!(menu::SETTINGS(), "Configuración");

    set_active_language(Language::English);
}

#[test]
fn formatted_messages_keep_runtime_values() {
    set_active_language(Language::Turkish);
    assert!(network::pushing("main", "origin").contains("main"));
    assert!(network::pushing("main", "origin").contains("origin"));

    set_active_language(Language::English);
}
