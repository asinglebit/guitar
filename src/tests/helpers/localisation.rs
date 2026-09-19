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
    let _language_guard = crate::helpers::localisation::LANGUAGE_TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    set_active_language(Language::English);
    assert_eq!(menu::SETTINGS(), "Settings");

    set_active_language(Language::Spanish);
    assert_eq!(menu::SETTINGS(), "Configuración");

    set_active_language(Language::English);
}

#[test]
fn settings_general_performance_lane_limit_text_is_localised() {
    let _language_guard = crate::helpers::localisation::LANGUAGE_TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    for (language, general, performance, lane_limit, prompt) in [
        (Language::English, "general", " performance:", " graph lane limit:", "Enter graph lane limit"),
        (Language::Spanish, "general", " rendimiento:", " límite de carriles del grafo:", "Introduce límite de carriles del grafo"),
        (Language::French, "général", " performances :", " limite de voies du graphe :", "Saisir la limite de voies du graphe"),
        (Language::Russian, "общие", " производительность:", " лимит дорожек графа:", "Введите лимит дорожек графа"),
        (Language::Turkish, "genel", " performans:", " grafik şerit sınırı:", "Grafik şerit sınırını gir"),
    ] {
        set_active_language(language);
        assert_eq!(settings::GENERAL(), general);
        assert_eq!(settings::PERFORMANCE(), performance);
        assert_eq!(settings::GRAPH_LANE_LIMIT(), lane_limit);
        assert_eq!(modal::PROMPT_GRAPH_LANE_LIMIT(), prompt);
    }

    set_active_language(Language::English);
}

#[test]
fn graph_lane_limit_shortcut_command_labels_are_localised() {
    let _language_guard = crate::helpers::localisation::LANGUAGE_TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    use crate::helpers::keymap::{Command, command_to_visual_string};

    for (language, shrink, grow) in [
        (Language::English, "Shrink graph lane limit", "Grow graph lane limit"),
        (Language::Spanish, "Reducir límite de carriles del grafo", "Aumentar límite de carriles del grafo"),
        (Language::French, "Réduire la limite de voies du graphe", "Augmenter la limite de voies du graphe"),
        (Language::Russian, "Уменьшить лимит дорожек графа", "Увеличить лимит дорожек графа"),
        (Language::Turkish, "Grafik şerit sınırını azalt", "Grafik şerit sınırını artır"),
    ] {
        set_active_language(language);
        assert_eq!(command_to_visual_string(&Command::ShrinkGraphLaneLimit), shrink);
        assert_eq!(command_to_visual_string(&Command::GrowGraphLaneLimit), grow);
    }

    set_active_language(Language::English);
}

#[test]
fn formatted_messages_keep_runtime_values() {
    let _language_guard = crate::helpers::localisation::LANGUAGE_TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    set_active_language(Language::Turkish);
    assert!(network::pushing("main", "origin").contains("main"));
    assert!(network::pushing("main", "origin").contains("origin"));

    set_active_language(Language::English);
}

// These call the per-language tables directly rather than switching the process-wide active
// language, so they stay deterministic when the suite runs in parallel.
type Translate = fn(&'static str) -> &'static str;
const TRANSLATORS: [(Translate, &str); 4] = [(es, "es"), (fr, "fr"), (ru, "ru"), (tr_tr, "tr")];

#[test]
fn settings_background_section_text_is_translated_in_every_language() {
    for ((translate, tag), background, file_watcher) in [
        (TRANSLATORS[0], " segundo plano:", "monitor de archivos"),
        (TRANSLATORS[1], " arrière-plan :", "surveillance de fichiers"),
        (TRANSLATORS[2], " фоновые задачи:", "наблюдение за файлами"),
        (TRANSLATORS[3], " arka plan:", "dosya izleyici"),
    ] {
        assert_eq!(translate(" background:"), background, "{tag} background heading");
        assert_eq!(translate("file watcher"), file_watcher, "{tag} file watcher row");
    }
}

#[test]
fn background_toggle_command_labels_are_translated_in_every_language() {
    for ((translate, tag), watcher) in [
        (TRANSLATORS[0], "Alternar monitor de archivos"),
        (TRANSLATORS[1], "Basculer la surveillance de fichiers"),
        (TRANSLATORS[2], "Переключить наблюдение за файлами"),
        (TRANSLATORS[3], "Dosya izleyiciyi aç/kapat"),
    ] {
        assert_eq!(translate("Toggle file watcher"), watcher, "{tag} watcher command label");
    }
}

#[test]
fn background_strings_fall_back_to_english_for_unknown_keys() {
    // The fallback chain must never panic or return an empty string.
    for (translate, tag) in TRANSLATORS {
        assert_eq!(translate("definitely not a translated key"), "definitely not a translated key", "{tag} fallback");
    }
}
