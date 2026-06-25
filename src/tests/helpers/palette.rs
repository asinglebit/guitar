use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn background_or_default_preserves_real_colors() {
    let theme = Theme::classic();

    assert_eq!(theme.COLOR_GREY_950, Color::Rgb(30, 30, 30));
    assert_eq!(theme.background_or_default(Color::Red), Color::Red);
    assert_eq!(theme.background_or_default(theme.COLOR_GREY_900), theme.COLOR_GREY_900);
}

#[test]
fn background_or_default_replaces_reset() {
    let theme = Theme::ansi();

    assert_eq!(theme.background_or_default(Color::Reset), theme.COLOR_GREY_950);
}

#[test]
fn clear_area_erases_symbols_and_paints_theme_background() {
    let theme = Theme::classic();
    let mut buffer = Buffer::with_lines(["abcde", "fghij", "klmno"]);
    let area = Rect::new(1, 1, 3, 1);

    theme.clear_area(area, &mut buffer);

    for x in 1..4 {
        let cell = &buffer[(x, 1)];
        assert_eq!(cell.symbol(), " ");
        assert_eq!(cell.bg, theme.COLOR_GREY_950);
    }

    assert_eq!(buffer[(0, 1)].symbol(), "f");
    assert_eq!(buffer[(4, 1)].symbol(), "j");
}

#[test]
fn presets_include_editor_theme_variants() {
    let expected = [
        "dracula dark",
        "dracula light",
        "monokai dark",
        "monokai light",
        "catppuccin dark",
        "catppuccin light",
        "atom dark",
        "atom light",
        "vscode dark",
        "vscode light",
        "solarized dark",
        "solarized light",
        "gruvbox dark",
        "gruvbox light",
        "nord",
        "tokyo night",
        "tokyo night storm",
        "tokyo night light",
        "github dark",
        "github light",
        "github dark dimmed",
        "night owl",
        "light owl",
        "ayu dark",
        "ayu mirage",
        "ayu light",
        "material",
        "palenight",
        "rose pine",
        "rose pine moon",
        "rose pine dawn",
        "kanagawa wave",
        "kanagawa dragon",
        "kanagawa lotus",
        "everforest dark",
        "everforest light",
        "zenburn",
        "horizon",
        "synthwave 84",
        "base16 tomorrow",
        "base16 ocean",
        "base16 eighties",
        "matrix",
    ];

    for label in expected {
        assert!(Theme::presets().iter().any(|preset| preset.label == label), "missing theme preset {label}");
    }

    assert!(Theme::presets().iter().all(|preset| preset.theme.background_color() != Color::Reset));
}

#[test]
fn dark_editor_theme_grey_900_is_lighter_than_grey_950() {
    for theme in [
        Theme::dracula_dark(),
        Theme::monokai_dark(),
        Theme::catppuccin_dark(),
        Theme::atom_dark(),
        Theme::solarized_dark(),
        Theme::gruvbox_dark(),
        Theme::tokyo_night(),
        Theme::github_dark(),
        Theme::night_owl(),
        Theme::everforest_dark(),
        Theme::matrix(),
    ] {
        assert!(rgb_brightness(theme.COLOR_GREY_900) > rgb_brightness(theme.COLOR_GREY_950), "{} grey 900 should be lighter than grey 950", theme.label());
    }
}

#[test]
fn preset_labels_resolve_to_their_themes() {
    for preset in Theme::presets() {
        assert_eq!(preset.theme.label(), preset.label);
        let resolved = Theme::from_label(preset.label).unwrap();
        assert!(resolved.name == preset.theme.name);
    }

    assert!(Theme::from_label("dracula-dark").unwrap().name == ThemeNames::DraculaDark);
    assert!(Theme::from_label("vscode_light").unwrap().name == ThemeNames::VsCodeLight);
    assert!(Theme::from_label("tokyo-night-storm").unwrap().name == ThemeNames::TokyoNightStorm);
    assert!(Theme::from_label("github_dark_dimmed").unwrap().name == ThemeNames::GithubDarkDimmed);
    assert!(Theme::from_label("matrix").unwrap().name == ThemeNames::Matrix);
}

#[test]
fn malformed_theme_config_falls_back_to_default_and_rewrites_full_json() {
    let path = temp_theme_path("malformed");
    fs::write(&path, "{ nope").unwrap();

    let theme = load_theme_from_path(&path);

    assert_eq!(theme.name, ThemeNames::Classic);

    let contents = fs::read_to_string(&path).unwrap();
    let config = facet_json::from_str::<ThemeConfig>(&contents).unwrap();
    assert_eq!(config.label, "classic");
}

#[test]
fn full_theme_config_loads_known_preset_without_custom_name() {
    let path = temp_theme_path("known-preset");
    save_theme_to_path(&path, &Theme::solarized_dark());

    let theme = load_theme_from_path(&path);

    assert_eq!(theme.name, ThemeNames::SolarizedDark);
    assert_eq!(theme.label(), "solarized dark");
    assert_eq!(theme.COLOR_GREY_950, Color::Rgb(0, 43, 54));
}

#[test]
fn partial_custom_theme_config_overrides_known_preset() {
    let path = temp_theme_path("partial-custom");
    fs::write(
        &path,
        r##"{
  "label": "solarized dark",
  "colors": {
    "red": "#010203",
    "blue": "light_blue",
    "grey_950": "#111213",
    "text": "white"
  }
}"##,
    )
    .unwrap();

    let theme = load_theme_from_path(&path);

    assert_eq!(theme.name, ThemeNames::Custom);
    assert_eq!(theme.label(), "solarized dark");
    assert_eq!(theme.COLOR_RED, Color::Rgb(1, 2, 3));
    assert_eq!(theme.COLOR_BLUE, Color::LightBlue);
    assert_eq!(theme.COLOR_GREY_950, Color::Rgb(17, 18, 19));
    assert_eq!(theme.COLOR_TEXT, Color::White);
    assert_eq!(theme.COLOR_GREEN, Theme::solarized_dark().COLOR_GREEN);
}

#[test]
fn unknown_custom_theme_starts_from_classic_and_ignores_invalid_colors() {
    let path = temp_theme_path("unknown-custom");
    fs::write(
        &path,
        r##"{
  "label": "my midnight",
  "colors": {
    "red": "not a color",
    "green": "#102030",
    "highlighted": "dark_gray"
  }
}"##,
    )
    .unwrap();

    let theme = load_theme_from_path(&path);

    assert_eq!(theme.name, ThemeNames::Custom);
    assert_eq!(theme.label(), "my midnight");
    assert_eq!(theme.COLOR_RED, Theme::classic().COLOR_RED);
    assert_eq!(theme.COLOR_GREEN, Color::Rgb(16, 32, 48));
    assert_eq!(theme.COLOR_HIGHLIGHTED, Color::DarkGray);
}

#[test]
fn save_theme_writes_full_theme_json() {
    let path = temp_theme_path("save-full");

    save_theme_to_path(&path, &Theme::ansi());

    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.contains('\n'), "{contents}");
    assert!(contents.contains("\n  \"label\""), "{contents}");
    assert!(contents.contains("\n    \"red\""), "{contents}");
    let config = facet_json::from_str::<ThemeConfig>(&contents).unwrap();
    assert_eq!(config.label, "ansi");
    assert_eq!(config.colors.red.unwrap(), "red");
    assert_eq!(config.colors.pink.unwrap(), "light_red");
    assert_eq!(config.colors.grey_950.unwrap(), "black");
    assert_eq!(config.colors.text.unwrap(), "white");
    assert_eq!(config.colors.highlighted.unwrap(), "reset");
}

fn rgb_brightness(color: Color) -> u16 {
    match color {
        Color::Rgb(r, g, b) => r as u16 + g as u16 + b as u16,
        other => panic!("expected RGB color, got {other:?}"),
    }
}

fn temp_theme_path(name: &str) -> PathBuf {
    let id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("guitar-palette-{name}-{id}"));
    fs::create_dir_all(&dir).unwrap();
    dir.join("theme.json")
}
