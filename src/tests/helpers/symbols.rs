use super::*;
use std::path::PathBuf;

#[path = "test_support.rs"]
mod test_support;
use test_support::{read_to_string, temp_json_path};

fn temp_symbols_path(name: &str) -> PathBuf {
    temp_json_path("guitar-symbols", name)
}

#[test]
fn main_matches_representative_current_symbols() {
    let theme = SymbolTheme::main();

    assert_eq!(theme.name, SymbolThemeName::Main);
    assert_eq!(theme.branch.local_visible, "●");
    assert_eq!(theme.branch.remote_visible, "◆");
    assert_eq!(theme.border.rounded_top_left, "╭");
    assert_eq!(theme.border.rounded_bottom_left, "╰");
    assert_eq!(theme.entity.folder, "");
    assert_eq!(theme.graph.branch_up_right, "╰");
    assert_eq!(theme.graph.horizontal_dotted, "┄");
    assert_eq!(theme.graph.merge, "•");
    assert_eq!(theme.graph.uncommitted, "◌");
    assert_eq!(theme.heatmap.many, "⣿");
    assert_eq!(theme.status.renamed_arrow_spaced, "→ ");
    assert_eq!(theme.splash.logo_compact, "guita╭");
}

#[test]
fn ascii_theme_uses_ascii_for_every_symbol_value() {
    let theme = SymbolTheme::ascii();

    assert_eq!(theme.name, SymbolThemeName::Ascii);
    assert!(theme.values().iter().all(|value| value.is_ascii()), "{:?}", theme.values());
    assert_eq!(theme.border.horizontal, "-");
    assert_eq!(theme.border.vertical, "|");
    assert_eq!(theme.border.rounded_top_left, "+");
    assert_eq!(theme.branch.local_visible, "*");
    assert_eq!(theme.graph.horizontal_dotted, ".");
    assert_eq!(theme.graph.vertical_dotted, ":");
    assert_eq!(theme.graph.branch_up_right, "+");
    assert_eq!(theme.heatmap.many, "@");
    assert_eq!(theme.form.checkbox_on, "[x]");
    assert_eq!(theme.status.renamed_arrow_spaced, "> ");
    assert_eq!(theme.splash.selected_left, "> ");
}

#[test]
fn missing_and_malformed_symbols_configs_load_main_and_rewrite_full_file() {
    for (name, contents) in [("missing", None), ("malformed", Some("{ definitely not json"))] {
        let path = temp_symbols_path(name);
        if let Some(contents) = contents {
            std::fs::write(&path, contents).unwrap();
        }

        let theme = load_symbol_theme_from_path(&path);
        let contents = read_to_string(&path);

        assert_eq!(theme, SymbolTheme::main());
        assert!(contents.contains("\"label\": \"main\""));
        assert!(contents.contains("\"rounded_bottom_left\""));
        assert!(contents.contains("\"branch_up_right\""));
        assert!(contents.contains("\"horizontal_dotted\""));
        assert!(contents.contains("\"symbols\""));
        if name == "missing" {
            assert!(contents.contains('\n'), "{contents}");
            assert!(contents.contains("\n  \"label\""), "{contents}");
            assert!(contents.contains("\n    \"branch\""), "{contents}");
        } else {
            assert!(!contents.contains("definitely not json"));
        }
    }
}

#[test]
fn known_preset_config_loads_without_rewriting_full_file() {
    let path = temp_symbols_path("preset");
    save_symbol_theme_to_path(&path, &SymbolTheme::ascii());
    let original = read_to_string(&path);

    let theme = load_symbol_theme_from_path(&path);
    let contents = read_to_string(&path);

    assert_eq!(theme, SymbolTheme::ascii());
    assert_eq!(contents, original);
    assert!(contents.contains("\"label\": \"ascii\""));
    assert!(contents.contains("\"renamed_arrow_spaced\": \"> \""));
}

#[test]
fn partial_overrides_preserve_unspecified_preset_values_without_rewriting() {
    let path = temp_symbols_path("partial");
    let original = r#"{
  "label": "ascii",
  "symbols": {
    "branch": {
      "local_visible": "@"
    }
  }
}"#;
    std::fs::write(&path, original).unwrap();

    let theme = load_symbol_theme_from_path(&path);
    let contents = read_to_string(&path);

    assert_eq!(theme.name, SymbolThemeName::Custom);
    assert_eq!(theme.label(), "ascii");
    assert_eq!(theme.branch.local_visible, "@");
    assert_eq!(theme.branch.local_hidden, "o");
    assert_eq!(theme.border.horizontal, "-");
    assert_eq!(theme.graph.horizontal_dotted, ".");
    assert_eq!(contents, original);
}

#[test]
fn unknown_labels_load_as_custom_using_main_as_base() {
    let path = temp_symbols_path("custom");
    fs::write(
        &path,
        r#"{
  "label": "my symbols",
  "symbols": {
    "empty_state": {
      "mark": "?"
    }
  }
}"#,
    )
    .unwrap();

    let theme = load_symbol_theme_from_path(&path);

    assert_eq!(theme.name, SymbolThemeName::Custom);
    assert_eq!(theme.label(), "my symbols");
    assert_eq!(theme.empty_state.mark, "?");
    assert_eq!(theme.branch.local_visible, SymbolTheme::main().branch.local_visible);
}
