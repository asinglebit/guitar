#![allow(non_snake_case)]

use facet::Facet;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeNames {
    Classic,
    Ansi,
    Monochrome,
    DraculaDark,
    DraculaLight,
    MonokaiDark,
    MonokaiLight,
    CatppuccinDark,
    CatppuccinLight,
    AtomDark,
    AtomLight,
    OneDarkWarmer,
    VsCodeDark,
    VsCodeLight,
    SolarizedDark,
    SolarizedLight,
    GruvboxDark,
    GruvboxLight,
    Nord,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight,
    GithubDark,
    GithubLight,
    GithubDarkDimmed,
    NightOwl,
    LightOwl,
    AyuDark,
    AyuMirage,
    AyuLight,
    Material,
    Palenight,
    RosePine,
    RosePineMoon,
    RosePineDawn,
    KanagawaWave,
    KanagawaDragon,
    KanagawaLotus,
    EverforestDark,
    EverforestLight,
    Zenburn,
    Horizon,
    Synthwave84,
    Base16Tomorrow,
    Base16Ocean,
    Base16Eighties,
    Matrix,
    Custom,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: ThemeNames,
    pub custom_label: Option<&'static str>,
    pub COLOR_RED: Color,
    pub COLOR_PINK: Color,
    pub COLOR_PURPLE: Color,
    pub COLOR_DURPLE: Color,
    pub COLOR_INDIGO: Color,
    pub COLOR_BLUE: Color,
    pub COLOR_CYAN: Color,
    pub COLOR_TEAL: Color,
    pub COLOR_GREEN: Color,
    pub COLOR_GRASS: Color,
    pub COLOR_LIME: Color,
    pub COLOR_YELLOW: Color,
    pub COLOR_AMBER: Color,
    pub COLOR_ORANGE: Color,
    pub COLOR_GRAPEFRUIT: Color,
    pub COLOR_BROWN: Color,
    pub COLOR_DARK_RED: Color,
    pub COLOR_LIGHT_GREEN_900: Color,
    pub COLOR_GREY_50: Color,
    pub COLOR_GREY_100: Color,
    pub COLOR_GREY_200: Color,
    pub COLOR_GREY_300: Color,
    pub COLOR_GREY_400: Color,
    pub COLOR_GREY_500: Color,
    pub COLOR_GREY_600: Color,
    pub COLOR_GREY_700: Color,
    pub COLOR_GREY_800: Color,
    pub COLOR_GREY_900: Color,
    pub COLOR_GREY_950: Color,
    pub COLOR_BORDER: Color,
    pub COLOR_TEXT: Color,
    pub COLOR_HIGHLIGHTED: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::classic()
    }
}

impl Theme {
    pub fn label(&self) -> &'static str {
        self.custom_label.unwrap_or_else(|| self.name.label())
    }

    pub fn from_label(label: &str) -> Option<Self> {
        let normalized = label.trim().to_ascii_lowercase().replace(['-', '_'], " ");
        Self::presets().iter().find(|preset| preset.label == normalized).map(|preset| preset.theme)
    }

    const fn from_colors(name: ThemeNames, accents: [Color; 18], surfaces: [Color; 14]) -> Self {
        Self {
            name,
            custom_label: None,
            COLOR_RED: accents[0],
            COLOR_PINK: accents[1],
            COLOR_PURPLE: accents[2],
            COLOR_DURPLE: accents[3],
            COLOR_INDIGO: accents[4],
            COLOR_BLUE: accents[5],
            COLOR_CYAN: accents[6],
            COLOR_TEAL: accents[7],
            COLOR_GREEN: accents[8],
            COLOR_GRASS: accents[9],
            COLOR_LIME: accents[10],
            COLOR_YELLOW: accents[11],
            COLOR_AMBER: accents[12],
            COLOR_ORANGE: accents[13],
            COLOR_GRAPEFRUIT: accents[14],
            COLOR_BROWN: accents[15],
            COLOR_DARK_RED: accents[16],
            COLOR_LIGHT_GREEN_900: accents[17],
            COLOR_GREY_50: surfaces[0],
            COLOR_GREY_100: surfaces[1],
            COLOR_GREY_200: surfaces[2],
            COLOR_GREY_300: surfaces[3],
            COLOR_GREY_400: surfaces[4],
            COLOR_GREY_500: surfaces[5],
            COLOR_GREY_600: surfaces[6],
            COLOR_GREY_700: surfaces[7],
            COLOR_GREY_800: surfaces[8],
            COLOR_GREY_900: surfaces[9],
            COLOR_GREY_950: surfaces[10],
            COLOR_BORDER: surfaces[11],
            COLOR_TEXT: surfaces[12],
            COLOR_HIGHLIGHTED: surfaces[13],
        }
    }

    fn custom(label: &str, mut theme: Self) -> Self {
        let label = if label.trim().is_empty() { "custom" } else { label.trim() };
        theme.name = ThemeNames::Custom;
        theme.custom_label = Some(Box::leak(label.to_string().into_boxed_str()));
        theme
    }

    fn colors_equal(&self, other: &Self) -> bool {
        self.COLOR_RED == other.COLOR_RED
            && self.COLOR_PINK == other.COLOR_PINK
            && self.COLOR_PURPLE == other.COLOR_PURPLE
            && self.COLOR_DURPLE == other.COLOR_DURPLE
            && self.COLOR_INDIGO == other.COLOR_INDIGO
            && self.COLOR_BLUE == other.COLOR_BLUE
            && self.COLOR_CYAN == other.COLOR_CYAN
            && self.COLOR_TEAL == other.COLOR_TEAL
            && self.COLOR_GREEN == other.COLOR_GREEN
            && self.COLOR_GRASS == other.COLOR_GRASS
            && self.COLOR_LIME == other.COLOR_LIME
            && self.COLOR_YELLOW == other.COLOR_YELLOW
            && self.COLOR_AMBER == other.COLOR_AMBER
            && self.COLOR_ORANGE == other.COLOR_ORANGE
            && self.COLOR_GRAPEFRUIT == other.COLOR_GRAPEFRUIT
            && self.COLOR_BROWN == other.COLOR_BROWN
            && self.COLOR_DARK_RED == other.COLOR_DARK_RED
            && self.COLOR_LIGHT_GREEN_900 == other.COLOR_LIGHT_GREEN_900
            && self.COLOR_GREY_50 == other.COLOR_GREY_50
            && self.COLOR_GREY_100 == other.COLOR_GREY_100
            && self.COLOR_GREY_200 == other.COLOR_GREY_200
            && self.COLOR_GREY_300 == other.COLOR_GREY_300
            && self.COLOR_GREY_400 == other.COLOR_GREY_400
            && self.COLOR_GREY_500 == other.COLOR_GREY_500
            && self.COLOR_GREY_600 == other.COLOR_GREY_600
            && self.COLOR_GREY_700 == other.COLOR_GREY_700
            && self.COLOR_GREY_800 == other.COLOR_GREY_800
            && self.COLOR_GREY_900 == other.COLOR_GREY_900
            && self.COLOR_GREY_950 == other.COLOR_GREY_950
            && self.COLOR_BORDER == other.COLOR_BORDER
            && self.COLOR_TEXT == other.COLOR_TEXT
            && self.COLOR_HIGHLIGHTED == other.COLOR_HIGHLIGHTED
    }

    pub const fn background_color(&self) -> Color {
        self.COLOR_GREY_950
    }

    pub const fn background_style(&self) -> Style {
        Style::new().bg(self.background_color())
    }

    pub const fn background_or_default(&self, color: Color) -> Color {
        match color {
            Color::Reset => self.background_color(),
            _ => color,
        }
    }

    pub fn clear_area(&self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(*buf.area());
        if area.is_empty() {
            return;
        }

        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                let cell = &mut buf[(x, y)];
                cell.reset();
                cell.set_bg(self.background_color());
            }
        }
    }

    pub const fn classic() -> Self {
        Self {
            name: ThemeNames::Classic,
            custom_label: None,
            COLOR_RED: Color::Rgb(239, 83, 80),
            COLOR_PINK: Color::Rgb(236, 64, 122),
            COLOR_PURPLE: Color::Rgb(171, 71, 188),
            COLOR_DURPLE: Color::Rgb(126, 87, 194),
            COLOR_INDIGO: Color::Rgb(92, 107, 192),
            COLOR_BLUE: Color::Rgb(66, 165, 245),
            COLOR_CYAN: Color::Rgb(38, 198, 218),
            COLOR_TEAL: Color::Rgb(38, 166, 154),
            COLOR_GREEN: Color::Rgb(102, 187, 106),
            COLOR_GRASS: Color::Rgb(156, 204, 101),
            COLOR_LIME: Color::Rgb(212, 225, 87),
            COLOR_YELLOW: Color::Rgb(255, 238, 88),
            COLOR_AMBER: Color::Rgb(255, 202, 40),
            COLOR_ORANGE: Color::Rgb(255, 167, 38),
            COLOR_GRAPEFRUIT: Color::Rgb(255, 112, 67),
            COLOR_BROWN: Color::Rgb(141, 110, 99),
            COLOR_DARK_RED: Color::Rgb(82, 31, 31),
            COLOR_LIGHT_GREEN_900: Color::Rgb(34, 57, 37),
            COLOR_GREY_50: Color::Rgb(250, 250, 250),
            COLOR_GREY_100: Color::Rgb(245, 245, 245),
            COLOR_GREY_200: Color::Rgb(238, 238, 238),
            COLOR_GREY_300: Color::Rgb(224, 224, 224),
            COLOR_GREY_400: Color::Rgb(189, 189, 189),
            COLOR_GREY_500: Color::Rgb(158, 158, 158),
            COLOR_GREY_600: Color::Rgb(117, 117, 117),
            COLOR_GREY_700: Color::Rgb(97, 97, 97),
            COLOR_GREY_800: Color::Rgb(66, 66, 66),
            COLOR_GREY_900: Color::Rgb(33, 33, 33),
            COLOR_GREY_950: Color::Rgb(30, 30, 30),
            COLOR_BORDER: Color::Rgb(66, 66, 66),
            COLOR_TEXT: Color::Rgb(97, 97, 97),
            COLOR_HIGHLIGHTED: Color::Rgb(158, 158, 158),
        }
    }
    pub const fn ansi() -> Self {
        Self {
            name: ThemeNames::Ansi,
            custom_label: None,
            COLOR_RED: Color::Red,
            COLOR_PINK: Color::LightRed,
            COLOR_PURPLE: Color::Magenta,
            COLOR_DURPLE: Color::LightMagenta,
            COLOR_INDIGO: Color::Blue,
            COLOR_BLUE: Color::LightBlue,
            COLOR_CYAN: Color::Cyan,
            COLOR_TEAL: Color::LightCyan,
            COLOR_GREEN: Color::Green,
            COLOR_GRASS: Color::LightGreen,
            COLOR_LIME: Color::Yellow,
            COLOR_YELLOW: Color::LightYellow,
            COLOR_AMBER: Color::Red,
            COLOR_ORANGE: Color::LightRed,
            COLOR_GRAPEFRUIT: Color::Magenta,
            COLOR_BROWN: Color::LightMagenta,
            COLOR_DARK_RED: Color::Reset,
            COLOR_LIGHT_GREEN_900: Color::Reset,
            COLOR_GREY_50: Color::Gray,
            COLOR_GREY_100: Color::Gray,
            COLOR_GREY_200: Color::Gray,
            COLOR_GREY_300: Color::Gray,
            COLOR_GREY_400: Color::DarkGray,
            COLOR_GREY_500: Color::DarkGray,
            COLOR_GREY_600: Color::DarkGray,
            COLOR_GREY_700: Color::DarkGray,
            COLOR_GREY_800: Color::DarkGray,
            COLOR_GREY_900: Color::Black,
            COLOR_GREY_950: Color::Black,
            COLOR_BORDER: Color::DarkGray,
            COLOR_TEXT: Color::White,
            COLOR_HIGHLIGHTED: Color::Reset,
        }
    }
    pub const fn monochrome() -> Self {
        Self {
            name: ThemeNames::Monochrome,
            custom_label: None,
            COLOR_RED: Color::White,
            COLOR_PINK: Color::White,
            COLOR_PURPLE: Color::White,
            COLOR_DURPLE: Color::White,
            COLOR_INDIGO: Color::White,
            COLOR_BLUE: Color::White,
            COLOR_CYAN: Color::White,
            COLOR_TEAL: Color::White,
            COLOR_GREEN: Color::White,
            COLOR_GRASS: Color::White,
            COLOR_LIME: Color::White,
            COLOR_YELLOW: Color::White,
            COLOR_AMBER: Color::White,
            COLOR_ORANGE: Color::White,
            COLOR_GRAPEFRUIT: Color::White,
            COLOR_BROWN: Color::White,
            COLOR_DARK_RED: Color::DarkGray,
            COLOR_LIGHT_GREEN_900: Color::DarkGray,
            COLOR_GREY_50: Color::Gray,
            COLOR_GREY_100: Color::Gray,
            COLOR_GREY_200: Color::Gray,
            COLOR_GREY_300: Color::Gray,
            COLOR_GREY_400: Color::DarkGray,
            COLOR_GREY_500: Color::DarkGray,
            COLOR_GREY_600: Color::DarkGray,
            COLOR_GREY_700: Color::DarkGray,
            COLOR_GREY_800: Color::DarkGray,
            COLOR_GREY_900: Color::Black,
            COLOR_GREY_950: Color::Black,
            COLOR_BORDER: Color::DarkGray,
            COLOR_TEXT: Color::White,
            COLOR_HIGHLIGHTED: Color::Reset,
        }
    }

    pub const fn dracula_dark() -> Self {
        Self {
            name: ThemeNames::DraculaDark,
            custom_label: None,
            COLOR_RED: Color::Rgb(255, 85, 85),
            COLOR_PINK: Color::Rgb(255, 121, 198),
            COLOR_PURPLE: Color::Rgb(189, 147, 249),
            COLOR_DURPLE: Color::Rgb(139, 133, 255),
            COLOR_INDIGO: Color::Rgb(98, 114, 164),
            COLOR_BLUE: Color::Rgb(139, 233, 253),
            COLOR_CYAN: Color::Rgb(139, 233, 253),
            COLOR_TEAL: Color::Rgb(82, 225, 211),
            COLOR_GREEN: Color::Rgb(80, 250, 123),
            COLOR_GRASS: Color::Rgb(105, 255, 137),
            COLOR_LIME: Color::Rgb(203, 255, 128),
            COLOR_YELLOW: Color::Rgb(241, 250, 140),
            COLOR_AMBER: Color::Rgb(255, 202, 102),
            COLOR_ORANGE: Color::Rgb(255, 184, 108),
            COLOR_GRAPEFRUIT: Color::Rgb(255, 110, 110),
            COLOR_BROWN: Color::Rgb(181, 140, 110),
            COLOR_DARK_RED: Color::Rgb(68, 35, 49),
            COLOR_LIGHT_GREEN_900: Color::Rgb(35, 68, 47),
            COLOR_GREY_50: Color::Rgb(248, 248, 242),
            COLOR_GREY_100: Color::Rgb(225, 225, 218),
            COLOR_GREY_200: Color::Rgb(191, 191, 186),
            COLOR_GREY_300: Color::Rgb(145, 148, 166),
            COLOR_GREY_400: Color::Rgb(98, 114, 164),
            COLOR_GREY_500: Color::Rgb(88, 92, 112),
            COLOR_GREY_600: Color::Rgb(68, 71, 90),
            COLOR_GREY_700: Color::Rgb(52, 55, 70),
            COLOR_GREY_800: Color::Rgb(68, 71, 90),
            COLOR_GREY_900: Color::Rgb(44, 46, 59),
            COLOR_GREY_950: Color::Rgb(40, 42, 54),
            COLOR_BORDER: Color::Rgb(68, 71, 90),
            COLOR_TEXT: Color::Rgb(248, 248, 242),
            COLOR_HIGHLIGHTED: Color::Rgb(248, 248, 242),
        }
    }

    pub const fn dracula_light() -> Self {
        Self {
            name: ThemeNames::DraculaLight,
            custom_label: None,
            COLOR_RED: Color::Rgb(214, 66, 66),
            COLOR_PINK: Color::Rgb(199, 63, 134),
            COLOR_PURPLE: Color::Rgb(124, 91, 191),
            COLOR_DURPLE: Color::Rgb(104, 89, 178),
            COLOR_INDIGO: Color::Rgb(77, 88, 132),
            COLOR_BLUE: Color::Rgb(45, 133, 168),
            COLOR_CYAN: Color::Rgb(8, 127, 149),
            COLOR_TEAL: Color::Rgb(24, 145, 132),
            COLOR_GREEN: Color::Rgb(47, 158, 68),
            COLOR_GRASS: Color::Rgb(64, 180, 77),
            COLOR_LIME: Color::Rgb(105, 166, 32),
            COLOR_YELLOW: Color::Rgb(160, 120, 0),
            COLOR_AMBER: Color::Rgb(184, 107, 19),
            COLOR_ORANGE: Color::Rgb(176, 100, 24),
            COLOR_GRAPEFRUIT: Color::Rgb(197, 79, 68),
            COLOR_BROWN: Color::Rgb(124, 89, 72),
            COLOR_DARK_RED: Color::Rgb(255, 230, 232),
            COLOR_LIGHT_GREEN_900: Color::Rgb(225, 248, 229),
            COLOR_GREY_50: Color::Rgb(40, 42, 54),
            COLOR_GREY_100: Color::Rgb(58, 54, 64),
            COLOR_GREY_200: Color::Rgb(78, 70, 82),
            COLOR_GREY_300: Color::Rgb(103, 94, 105),
            COLOR_GREY_400: Color::Rgb(128, 119, 128),
            COLOR_GREY_500: Color::Rgb(112, 102, 92),
            COLOR_GREY_600: Color::Rgb(151, 141, 126),
            COLOR_GREY_700: Color::Rgb(190, 183, 170),
            COLOR_GREY_800: Color::Rgb(221, 217, 207),
            COLOR_GREY_900: Color::Rgb(239, 238, 232),
            COLOR_GREY_950: Color::Rgb(248, 248, 242),
            COLOR_BORDER: Color::Rgb(214, 208, 198),
            COLOR_TEXT: Color::Rgb(40, 42, 54),
            COLOR_HIGHLIGHTED: Color::Rgb(40, 42, 54),
        }
    }

    pub const fn monokai_dark() -> Self {
        Self {
            name: ThemeNames::MonokaiDark,
            custom_label: None,
            COLOR_RED: Color::Rgb(249, 38, 114),
            COLOR_PINK: Color::Rgb(249, 38, 114),
            COLOR_PURPLE: Color::Rgb(174, 129, 255),
            COLOR_DURPLE: Color::Rgb(146, 118, 218),
            COLOR_INDIGO: Color::Rgb(102, 137, 204),
            COLOR_BLUE: Color::Rgb(102, 217, 239),
            COLOR_CYAN: Color::Rgb(102, 217, 239),
            COLOR_TEAL: Color::Rgb(95, 207, 192),
            COLOR_GREEN: Color::Rgb(166, 226, 46),
            COLOR_GRASS: Color::Rgb(190, 237, 76),
            COLOR_LIME: Color::Rgb(210, 235, 86),
            COLOR_YELLOW: Color::Rgb(230, 219, 116),
            COLOR_AMBER: Color::Rgb(244, 191, 117),
            COLOR_ORANGE: Color::Rgb(253, 151, 31),
            COLOR_GRAPEFRUIT: Color::Rgb(255, 97, 136),
            COLOR_BROWN: Color::Rgb(117, 113, 94),
            COLOR_DARK_RED: Color::Rgb(64, 25, 42),
            COLOR_LIGHT_GREEN_900: Color::Rgb(38, 62, 24),
            COLOR_GREY_50: Color::Rgb(248, 248, 242),
            COLOR_GREY_100: Color::Rgb(220, 220, 214),
            COLOR_GREY_200: Color::Rgb(190, 190, 180),
            COLOR_GREY_300: Color::Rgb(160, 160, 148),
            COLOR_GREY_400: Color::Rgb(117, 113, 94),
            COLOR_GREY_500: Color::Rgb(96, 94, 83),
            COLOR_GREY_600: Color::Rgb(73, 72, 62),
            COLOR_GREY_700: Color::Rgb(56, 56, 48),
            COLOR_GREY_800: Color::Rgb(73, 72, 62),
            COLOR_GREY_900: Color::Rgb(43, 44, 38),
            COLOR_GREY_950: Color::Rgb(39, 40, 34),
            COLOR_BORDER: Color::Rgb(73, 72, 62),
            COLOR_TEXT: Color::Rgb(248, 248, 242),
            COLOR_HIGHLIGHTED: Color::Rgb(248, 248, 242),
        }
    }

    pub const fn monokai_light() -> Self {
        Self {
            name: ThemeNames::MonokaiLight,
            custom_label: None,
            COLOR_RED: Color::Rgb(249, 38, 114),
            COLOR_PINK: Color::Rgb(214, 54, 128),
            COLOR_PURPLE: Color::Rgb(126, 87, 194),
            COLOR_DURPLE: Color::Rgb(113, 83, 180),
            COLOR_INDIGO: Color::Rgb(63, 99, 175),
            COLOR_BLUE: Color::Rgb(0, 132, 181),
            COLOR_CYAN: Color::Rgb(0, 145, 170),
            COLOR_TEAL: Color::Rgb(0, 145, 124),
            COLOR_GREEN: Color::Rgb(80, 150, 31),
            COLOR_GRASS: Color::Rgb(98, 165, 35),
            COLOR_LIME: Color::Rgb(130, 160, 20),
            COLOR_YELLOW: Color::Rgb(151, 128, 24),
            COLOR_AMBER: Color::Rgb(190, 112, 22),
            COLOR_ORANGE: Color::Rgb(210, 92, 20),
            COLOR_GRAPEFRUIT: Color::Rgb(216, 70, 88),
            COLOR_BROWN: Color::Rgb(115, 87, 69),
            COLOR_DARK_RED: Color::Rgb(255, 230, 239),
            COLOR_LIGHT_GREEN_900: Color::Rgb(232, 248, 218),
            COLOR_GREY_50: Color::Rgb(39, 40, 34),
            COLOR_GREY_100: Color::Rgb(65, 65, 58),
            COLOR_GREY_200: Color::Rgb(91, 88, 77),
            COLOR_GREY_300: Color::Rgb(122, 116, 100),
            COLOR_GREY_400: Color::Rgb(150, 145, 130),
            COLOR_GREY_500: Color::Rgb(94, 89, 82),
            COLOR_GREY_600: Color::Rgb(166, 163, 153),
            COLOR_GREY_700: Color::Rgb(208, 205, 194),
            COLOR_GREY_800: Color::Rgb(232, 230, 222),
            COLOR_GREY_900: Color::Rgb(245, 244, 239),
            COLOR_GREY_950: Color::Rgb(250, 250, 247),
            COLOR_BORDER: Color::Rgb(218, 216, 208),
            COLOR_TEXT: Color::Rgb(64, 62, 65),
            COLOR_HIGHLIGHTED: Color::Rgb(64, 62, 65),
        }
    }

    pub const fn catppuccin_dark() -> Self {
        Self {
            name: ThemeNames::CatppuccinDark,
            custom_label: None,
            COLOR_RED: Color::Rgb(243, 139, 168),
            COLOR_PINK: Color::Rgb(245, 194, 231),
            COLOR_PURPLE: Color::Rgb(203, 166, 247),
            COLOR_DURPLE: Color::Rgb(180, 190, 254),
            COLOR_INDIGO: Color::Rgb(137, 180, 250),
            COLOR_BLUE: Color::Rgb(137, 180, 250),
            COLOR_CYAN: Color::Rgb(137, 220, 235),
            COLOR_TEAL: Color::Rgb(148, 226, 213),
            COLOR_GREEN: Color::Rgb(166, 227, 161),
            COLOR_GRASS: Color::Rgb(166, 227, 161),
            COLOR_LIME: Color::Rgb(184, 232, 128),
            COLOR_YELLOW: Color::Rgb(249, 226, 175),
            COLOR_AMBER: Color::Rgb(250, 179, 135),
            COLOR_ORANGE: Color::Rgb(250, 179, 135),
            COLOR_GRAPEFRUIT: Color::Rgb(235, 160, 172),
            COLOR_BROWN: Color::Rgb(166, 148, 129),
            COLOR_DARK_RED: Color::Rgb(63, 39, 53),
            COLOR_LIGHT_GREEN_900: Color::Rgb(42, 61, 48),
            COLOR_GREY_50: Color::Rgb(205, 214, 244),
            COLOR_GREY_100: Color::Rgb(186, 194, 222),
            COLOR_GREY_200: Color::Rgb(166, 173, 200),
            COLOR_GREY_300: Color::Rgb(147, 153, 178),
            COLOR_GREY_400: Color::Rgb(127, 132, 156),
            COLOR_GREY_500: Color::Rgb(108, 112, 134),
            COLOR_GREY_600: Color::Rgb(88, 91, 112),
            COLOR_GREY_700: Color::Rgb(69, 71, 90),
            COLOR_GREY_800: Color::Rgb(49, 50, 68),
            COLOR_GREY_900: Color::Rgb(34, 34, 51),
            COLOR_GREY_950: Color::Rgb(30, 30, 46),
            COLOR_BORDER: Color::Rgb(69, 71, 90),
            COLOR_TEXT: Color::Rgb(205, 214, 244),
            COLOR_HIGHLIGHTED: Color::Rgb(205, 214, 244),
        }
    }

    pub const fn catppuccin_light() -> Self {
        Self {
            name: ThemeNames::CatppuccinLight,
            custom_label: None,
            COLOR_RED: Color::Rgb(210, 15, 57),
            COLOR_PINK: Color::Rgb(234, 118, 203),
            COLOR_PURPLE: Color::Rgb(136, 57, 239),
            COLOR_DURPLE: Color::Rgb(114, 135, 253),
            COLOR_INDIGO: Color::Rgb(30, 102, 245),
            COLOR_BLUE: Color::Rgb(30, 102, 245),
            COLOR_CYAN: Color::Rgb(4, 165, 229),
            COLOR_TEAL: Color::Rgb(23, 146, 153),
            COLOR_GREEN: Color::Rgb(64, 160, 43),
            COLOR_GRASS: Color::Rgb(64, 160, 43),
            COLOR_LIME: Color::Rgb(92, 145, 18),
            COLOR_YELLOW: Color::Rgb(223, 142, 29),
            COLOR_AMBER: Color::Rgb(254, 100, 11),
            COLOR_ORANGE: Color::Rgb(254, 100, 11),
            COLOR_GRAPEFRUIT: Color::Rgb(230, 69, 83),
            COLOR_BROWN: Color::Rgb(124, 90, 76),
            COLOR_DARK_RED: Color::Rgb(252, 226, 231),
            COLOR_LIGHT_GREEN_900: Color::Rgb(230, 246, 224),
            COLOR_GREY_50: Color::Rgb(76, 79, 105),
            COLOR_GREY_100: Color::Rgb(92, 95, 119),
            COLOR_GREY_200: Color::Rgb(108, 111, 133),
            COLOR_GREY_300: Color::Rgb(124, 127, 147),
            COLOR_GREY_400: Color::Rgb(140, 143, 161),
            COLOR_GREY_500: Color::Rgb(108, 111, 133),
            COLOR_GREY_600: Color::Rgb(172, 176, 190),
            COLOR_GREY_700: Color::Rgb(204, 208, 218),
            COLOR_GREY_800: Color::Rgb(220, 224, 232),
            COLOR_GREY_900: Color::Rgb(230, 233, 239),
            COLOR_GREY_950: Color::Rgb(239, 241, 245),
            COLOR_BORDER: Color::Rgb(204, 208, 218),
            COLOR_TEXT: Color::Rgb(76, 79, 105),
            COLOR_HIGHLIGHTED: Color::Rgb(76, 79, 105),
        }
    }

    pub const fn atom_dark() -> Self {
        Self {
            name: ThemeNames::AtomDark,
            custom_label: None,
            COLOR_RED: Color::Rgb(224, 108, 117),
            COLOR_PINK: Color::Rgb(198, 120, 221),
            COLOR_PURPLE: Color::Rgb(198, 120, 221),
            COLOR_DURPLE: Color::Rgb(171, 123, 216),
            COLOR_INDIGO: Color::Rgb(97, 175, 239),
            COLOR_BLUE: Color::Rgb(97, 175, 239),
            COLOR_CYAN: Color::Rgb(86, 182, 194),
            COLOR_TEAL: Color::Rgb(86, 182, 194),
            COLOR_GREEN: Color::Rgb(152, 195, 121),
            COLOR_GRASS: Color::Rgb(152, 195, 121),
            COLOR_LIME: Color::Rgb(183, 205, 129),
            COLOR_YELLOW: Color::Rgb(229, 192, 123),
            COLOR_AMBER: Color::Rgb(209, 154, 102),
            COLOR_ORANGE: Color::Rgb(209, 154, 102),
            COLOR_GRAPEFRUIT: Color::Rgb(224, 132, 122),
            COLOR_BROWN: Color::Rgb(145, 129, 111),
            COLOR_DARK_RED: Color::Rgb(64, 36, 42),
            COLOR_LIGHT_GREEN_900: Color::Rgb(42, 60, 39),
            COLOR_GREY_50: Color::Rgb(171, 178, 191),
            COLOR_GREY_100: Color::Rgb(157, 165, 180),
            COLOR_GREY_200: Color::Rgb(130, 137, 151),
            COLOR_GREY_300: Color::Rgb(112, 119, 133),
            COLOR_GREY_400: Color::Rgb(92, 99, 112),
            COLOR_GREY_500: Color::Rgb(78, 84, 96),
            COLOR_GREY_600: Color::Rgb(62, 68, 81),
            COLOR_GREY_700: Color::Rgb(53, 59, 71),
            COLOR_GREY_800: Color::Rgb(62, 68, 81),
            COLOR_GREY_900: Color::Rgb(44, 48, 57),
            COLOR_GREY_950: Color::Rgb(40, 44, 52),
            COLOR_BORDER: Color::Rgb(62, 68, 81),
            COLOR_TEXT: Color::Rgb(171, 178, 191),
            COLOR_HIGHLIGHTED: Color::Rgb(171, 178, 191),
        }
    }

    pub const fn atom_light() -> Self {
        Self {
            name: ThemeNames::AtomLight,
            custom_label: None,
            COLOR_RED: Color::Rgb(228, 86, 73),
            COLOR_PINK: Color::Rgb(166, 38, 164),
            COLOR_PURPLE: Color::Rgb(166, 38, 164),
            COLOR_DURPLE: Color::Rgb(128, 62, 170),
            COLOR_INDIGO: Color::Rgb(64, 120, 242),
            COLOR_BLUE: Color::Rgb(64, 120, 242),
            COLOR_CYAN: Color::Rgb(1, 132, 188),
            COLOR_TEAL: Color::Rgb(1, 135, 145),
            COLOR_GREEN: Color::Rgb(80, 161, 79),
            COLOR_GRASS: Color::Rgb(80, 161, 79),
            COLOR_LIME: Color::Rgb(110, 155, 42),
            COLOR_YELLOW: Color::Rgb(193, 132, 1),
            COLOR_AMBER: Color::Rgb(202, 118, 0),
            COLOR_ORANGE: Color::Rgb(202, 118, 0),
            COLOR_GRAPEFRUIT: Color::Rgb(211, 82, 70),
            COLOR_BROWN: Color::Rgb(138, 98, 75),
            COLOR_DARK_RED: Color::Rgb(255, 230, 227),
            COLOR_LIGHT_GREEN_900: Color::Rgb(231, 247, 230),
            COLOR_GREY_50: Color::Rgb(56, 58, 66),
            COLOR_GREY_100: Color::Rgb(75, 77, 86),
            COLOR_GREY_200: Color::Rgb(92, 95, 105),
            COLOR_GREY_300: Color::Rgb(122, 126, 138),
            COLOR_GREY_400: Color::Rgb(160, 161, 167),
            COLOR_GREY_500: Color::Rgb(105, 108, 119),
            COLOR_GREY_600: Color::Rgb(186, 188, 195),
            COLOR_GREY_700: Color::Rgb(218, 220, 226),
            COLOR_GREY_800: Color::Rgb(235, 237, 240),
            COLOR_GREY_900: Color::Rgb(245, 246, 247),
            COLOR_GREY_950: Color::Rgb(250, 250, 250),
            COLOR_BORDER: Color::Rgb(218, 220, 226),
            COLOR_TEXT: Color::Rgb(56, 58, 66),
            COLOR_HIGHLIGHTED: Color::Rgb(56, 58, 66),
        }
    }

    pub const fn one_dark_warmer() -> Self {
        Self::from_colors(
            ThemeNames::OneDarkWarmer,
            [
                Color::Rgb(222, 93, 104),
                Color::Rgb(209, 125, 235),
                Color::Rgb(187, 112, 210),
                Color::Rgb(159, 95, 179),
                Color::Rgb(74, 140, 195),
                Color::Rgb(87, 165, 229),
                Color::Rgb(81, 168, 179),
                Color::Rgb(69, 143, 152),
                Color::Rgb(122, 154, 98),
                Color::Rgb(143, 181, 115),
                Color::Rgb(160, 203, 129),
                Color::Rgb(245, 204, 127),
                Color::Rgb(219, 182, 113),
                Color::Rgb(196, 144, 96),
                Color::Rgb(249, 104, 116),
                Color::Rgb(142, 104, 69),
                Color::Rgb(42, 38, 38),
                Color::Rgb(40, 43, 38),
            ],
            [
                Color::Rgb(250, 250, 250),
                Color::Rgb(208, 210, 213),
                Color::Rgb(167, 170, 176),
                Color::Rgb(148, 150, 155),
                Color::Rgb(129, 131, 135),
                Color::Rgb(110, 111, 115),
                Color::Rgb(90, 91, 94),
                Color::Rgb(72, 73, 77),
                Color::Rgb(53, 54, 59),
                Color::Rgb(27, 28, 30),
                Color::Rgb(35, 35, 38),
                Color::Rgb(90, 91, 94),
                Color::Rgb(167, 170, 176),
                Color::Rgb(250, 250, 250),
            ],
        )
    }

    pub const fn vscode_dark() -> Self {
        Self {
            name: ThemeNames::VsCodeDark,
            custom_label: None,
            COLOR_RED: Color::Rgb(244, 71, 71),
            COLOR_PINK: Color::Rgb(197, 134, 192),
            COLOR_PURPLE: Color::Rgb(197, 134, 192),
            COLOR_DURPLE: Color::Rgb(156, 139, 207),
            COLOR_INDIGO: Color::Rgb(86, 156, 214),
            COLOR_BLUE: Color::Rgb(86, 156, 214),
            COLOR_CYAN: Color::Rgb(78, 201, 176),
            COLOR_TEAL: Color::Rgb(78, 201, 176),
            COLOR_GREEN: Color::Rgb(106, 153, 85),
            COLOR_GRASS: Color::Rgb(181, 206, 168),
            COLOR_LIME: Color::Rgb(181, 206, 168),
            COLOR_YELLOW: Color::Rgb(220, 220, 170),
            COLOR_AMBER: Color::Rgb(206, 145, 120),
            COLOR_ORANGE: Color::Rgb(206, 145, 120),
            COLOR_GRAPEFRUIT: Color::Rgb(216, 120, 120),
            COLOR_BROWN: Color::Rgb(156, 128, 102),
            COLOR_DARK_RED: Color::Rgb(68, 32, 37),
            COLOR_LIGHT_GREEN_900: Color::Rgb(41, 58, 38),
            COLOR_GREY_50: Color::Rgb(212, 212, 212),
            COLOR_GREY_100: Color::Rgb(190, 190, 190),
            COLOR_GREY_200: Color::Rgb(160, 160, 160),
            COLOR_GREY_300: Color::Rgb(128, 128, 128),
            COLOR_GREY_400: Color::Rgb(106, 106, 106),
            COLOR_GREY_500: Color::Rgb(86, 86, 86),
            COLOR_GREY_600: Color::Rgb(65, 65, 65),
            COLOR_GREY_700: Color::Rgb(51, 51, 51),
            COLOR_GREY_800: Color::Rgb(60, 60, 60),
            COLOR_GREY_900: Color::Rgb(37, 37, 38),
            COLOR_GREY_950: Color::Rgb(30, 30, 30),
            COLOR_BORDER: Color::Rgb(60, 60, 60),
            COLOR_TEXT: Color::Rgb(86, 86, 86),
            COLOR_HIGHLIGHTED: Color::Rgb(212, 212, 212),
        }
    }

    pub const fn vscode_light() -> Self {
        Self {
            name: ThemeNames::VsCodeLight,
            custom_label: None,
            COLOR_RED: Color::Rgb(163, 21, 21),
            COLOR_PINK: Color::Rgb(175, 0, 219),
            COLOR_PURPLE: Color::Rgb(175, 0, 219),
            COLOR_DURPLE: Color::Rgb(121, 94, 38),
            COLOR_INDIGO: Color::Rgb(0, 0, 255),
            COLOR_BLUE: Color::Rgb(0, 0, 255),
            COLOR_CYAN: Color::Rgb(38, 127, 153),
            COLOR_TEAL: Color::Rgb(0, 128, 128),
            COLOR_GREEN: Color::Rgb(0, 128, 0),
            COLOR_GRASS: Color::Rgb(9, 134, 88),
            COLOR_LIME: Color::Rgb(92, 145, 32),
            COLOR_YELLOW: Color::Rgb(121, 94, 38),
            COLOR_AMBER: Color::Rgb(181, 118, 20),
            COLOR_ORANGE: Color::Rgb(181, 118, 20),
            COLOR_GRAPEFRUIT: Color::Rgb(180, 75, 64),
            COLOR_BROWN: Color::Rgb(128, 96, 72),
            COLOR_DARK_RED: Color::Rgb(255, 232, 232),
            COLOR_LIGHT_GREEN_900: Color::Rgb(230, 248, 230),
            COLOR_GREY_50: Color::Rgb(30, 30, 30),
            COLOR_GREY_100: Color::Rgb(51, 51, 51),
            COLOR_GREY_200: Color::Rgb(85, 85, 85),
            COLOR_GREY_300: Color::Rgb(117, 117, 117),
            COLOR_GREY_400: Color::Rgb(150, 150, 150),
            COLOR_GREY_500: Color::Rgb(97, 97, 97),
            COLOR_GREY_600: Color::Rgb(198, 198, 198),
            COLOR_GREY_700: Color::Rgb(229, 229, 229),
            COLOR_GREY_800: Color::Rgb(238, 238, 238),
            COLOR_GREY_900: Color::Rgb(245, 245, 245),
            COLOR_GREY_950: Color::Rgb(255, 255, 255),
            COLOR_BORDER: Color::Rgb(229, 229, 229),
            COLOR_TEXT: Color::Rgb(30, 30, 30),
            COLOR_HIGHLIGHTED: Color::Rgb(30, 30, 30),
        }
    }

    pub const fn solarized_dark() -> Self {
        Self::from_colors(
            ThemeNames::SolarizedDark,
            [
                Color::Rgb(220, 50, 47),
                Color::Rgb(211, 54, 130),
                Color::Rgb(108, 113, 196),
                Color::Rgb(91, 91, 176),
                Color::Rgb(38, 139, 210),
                Color::Rgb(38, 139, 210),
                Color::Rgb(42, 161, 152),
                Color::Rgb(42, 161, 152),
                Color::Rgb(133, 153, 0),
                Color::Rgb(133, 153, 0),
                Color::Rgb(115, 138, 0),
                Color::Rgb(181, 137, 0),
                Color::Rgb(203, 75, 22),
                Color::Rgb(203, 75, 22),
                Color::Rgb(220, 90, 72),
                Color::Rgb(101, 83, 62),
                Color::Rgb(77, 31, 34),
                Color::Rgb(38, 61, 36),
            ],
            [
                Color::Rgb(253, 246, 227),
                Color::Rgb(238, 232, 213),
                Color::Rgb(147, 161, 161),
                Color::Rgb(131, 148, 150),
                Color::Rgb(101, 123, 131),
                Color::Rgb(88, 110, 117),
                Color::Rgb(7, 54, 66),
                Color::Rgb(5, 48, 59),
                Color::Rgb(7, 54, 66),
                Color::Rgb(1, 49, 63),
                Color::Rgb(0, 43, 54),
                Color::Rgb(7, 54, 66),
                Color::Rgb(131, 148, 150),
                Color::Rgb(147, 161, 161),
            ],
        )
    }

    pub const fn solarized_light() -> Self {
        Self::from_colors(
            ThemeNames::SolarizedLight,
            [
                Color::Rgb(220, 50, 47),
                Color::Rgb(211, 54, 130),
                Color::Rgb(108, 113, 196),
                Color::Rgb(91, 91, 176),
                Color::Rgb(38, 139, 210),
                Color::Rgb(38, 139, 210),
                Color::Rgb(42, 161, 152),
                Color::Rgb(42, 161, 152),
                Color::Rgb(133, 153, 0),
                Color::Rgb(133, 153, 0),
                Color::Rgb(115, 138, 0),
                Color::Rgb(181, 137, 0),
                Color::Rgb(203, 75, 22),
                Color::Rgb(203, 75, 22),
                Color::Rgb(220, 90, 72),
                Color::Rgb(101, 83, 62),
                Color::Rgb(253, 230, 226),
                Color::Rgb(232, 241, 210),
            ],
            [
                Color::Rgb(0, 43, 54),
                Color::Rgb(7, 54, 66),
                Color::Rgb(88, 110, 117),
                Color::Rgb(101, 123, 131),
                Color::Rgb(131, 148, 150),
                Color::Rgb(88, 110, 117),
                Color::Rgb(147, 161, 161),
                Color::Rgb(218, 211, 191),
                Color::Rgb(238, 232, 213),
                Color::Rgb(246, 239, 219),
                Color::Rgb(253, 246, 227),
                Color::Rgb(238, 232, 213),
                Color::Rgb(101, 123, 131),
                Color::Rgb(88, 110, 117),
            ],
        )
    }

    pub const fn gruvbox_dark() -> Self {
        Self::from_colors(
            ThemeNames::GruvboxDark,
            [
                Color::Rgb(251, 73, 52),
                Color::Rgb(211, 134, 155),
                Color::Rgb(211, 134, 155),
                Color::Rgb(177, 98, 134),
                Color::Rgb(131, 165, 152),
                Color::Rgb(131, 165, 152),
                Color::Rgb(142, 192, 124),
                Color::Rgb(142, 192, 124),
                Color::Rgb(184, 187, 38),
                Color::Rgb(184, 187, 38),
                Color::Rgb(215, 153, 33),
                Color::Rgb(250, 189, 47),
                Color::Rgb(254, 128, 25),
                Color::Rgb(254, 128, 25),
                Color::Rgb(251, 105, 84),
                Color::Rgb(146, 111, 83),
                Color::Rgb(69, 35, 31),
                Color::Rgb(48, 61, 33),
            ],
            [
                Color::Rgb(251, 241, 199),
                Color::Rgb(235, 219, 178),
                Color::Rgb(213, 196, 161),
                Color::Rgb(189, 174, 147),
                Color::Rgb(168, 153, 132),
                Color::Rgb(146, 131, 116),
                Color::Rgb(80, 73, 69),
                Color::Rgb(60, 56, 54),
                Color::Rgb(80, 73, 69),
                Color::Rgb(50, 48, 47),
                Color::Rgb(40, 40, 40),
                Color::Rgb(80, 73, 69),
                Color::Rgb(235, 219, 178),
                Color::Rgb(251, 241, 199),
            ],
        )
    }

    pub const fn gruvbox_light() -> Self {
        Self::from_colors(
            ThemeNames::GruvboxLight,
            [
                Color::Rgb(204, 36, 29),
                Color::Rgb(177, 98, 134),
                Color::Rgb(177, 98, 134),
                Color::Rgb(143, 63, 113),
                Color::Rgb(69, 133, 136),
                Color::Rgb(7, 102, 120),
                Color::Rgb(66, 123, 88),
                Color::Rgb(66, 123, 88),
                Color::Rgb(121, 116, 14),
                Color::Rgb(104, 157, 106),
                Color::Rgb(152, 151, 26),
                Color::Rgb(181, 118, 20),
                Color::Rgb(175, 58, 3),
                Color::Rgb(175, 58, 3),
                Color::Rgb(204, 73, 52),
                Color::Rgb(124, 91, 70),
                Color::Rgb(251, 229, 218),
                Color::Rgb(235, 244, 219),
            ],
            [
                Color::Rgb(60, 56, 54),
                Color::Rgb(80, 73, 69),
                Color::Rgb(102, 92, 84),
                Color::Rgb(124, 111, 100),
                Color::Rgb(146, 131, 116),
                Color::Rgb(124, 111, 100),
                Color::Rgb(189, 174, 147),
                Color::Rgb(213, 196, 161),
                Color::Rgb(235, 219, 178),
                Color::Rgb(242, 229, 188),
                Color::Rgb(251, 241, 199),
                Color::Rgb(213, 196, 161),
                Color::Rgb(60, 56, 54),
                Color::Rgb(40, 40, 40),
            ],
        )
    }

    pub const fn nord() -> Self {
        Self::from_colors(
            ThemeNames::Nord,
            [
                Color::Rgb(191, 97, 106),
                Color::Rgb(180, 142, 173),
                Color::Rgb(180, 142, 173),
                Color::Rgb(143, 129, 184),
                Color::Rgb(94, 129, 172),
                Color::Rgb(129, 161, 193),
                Color::Rgb(136, 192, 208),
                Color::Rgb(143, 188, 187),
                Color::Rgb(163, 190, 140),
                Color::Rgb(163, 190, 140),
                Color::Rgb(180, 200, 130),
                Color::Rgb(235, 203, 139),
                Color::Rgb(208, 135, 112),
                Color::Rgb(208, 135, 112),
                Color::Rgb(204, 112, 118),
                Color::Rgb(143, 112, 88),
                Color::Rgb(67, 43, 50),
                Color::Rgb(48, 66, 52),
            ],
            [
                Color::Rgb(236, 239, 244),
                Color::Rgb(229, 233, 240),
                Color::Rgb(216, 222, 233),
                Color::Rgb(191, 199, 214),
                Color::Rgb(165, 174, 194),
                Color::Rgb(121, 134, 162),
                Color::Rgb(76, 86, 106),
                Color::Rgb(67, 76, 94),
                Color::Rgb(59, 66, 82),
                Color::Rgb(51, 58, 72),
                Color::Rgb(46, 52, 64),
                Color::Rgb(76, 86, 106),
                Color::Rgb(216, 222, 233),
                Color::Rgb(236, 239, 244),
            ],
        )
    }

    pub const fn tokyo_night() -> Self {
        Self::from_colors(
            ThemeNames::TokyoNight,
            [
                Color::Rgb(247, 118, 142),
                Color::Rgb(255, 0, 124),
                Color::Rgb(187, 154, 247),
                Color::Rgb(157, 124, 216),
                Color::Rgb(122, 162, 247),
                Color::Rgb(122, 162, 247),
                Color::Rgb(125, 207, 255),
                Color::Rgb(115, 218, 202),
                Color::Rgb(158, 206, 106),
                Color::Rgb(158, 206, 106),
                Color::Rgb(180, 220, 101),
                Color::Rgb(224, 175, 104),
                Color::Rgb(255, 158, 100),
                Color::Rgb(255, 158, 100),
                Color::Rgb(255, 118, 142),
                Color::Rgb(150, 111, 87),
                Color::Rgb(69, 38, 52),
                Color::Rgb(44, 63, 46),
            ],
            [
                Color::Rgb(192, 202, 245),
                Color::Rgb(169, 177, 214),
                Color::Rgb(154, 165, 206),
                Color::Rgb(128, 139, 180),
                Color::Rgb(86, 95, 137),
                Color::Rgb(65, 72, 104),
                Color::Rgb(52, 59, 88),
                Color::Rgb(41, 46, 66),
                Color::Rgb(36, 40, 59),
                Color::Rgb(31, 35, 53),
                Color::Rgb(26, 27, 38),
                Color::Rgb(65, 72, 104),
                Color::Rgb(192, 202, 245),
                Color::Rgb(203, 211, 255),
            ],
        )
    }

    pub const fn tokyo_night_storm() -> Self {
        Self::from_colors(
            ThemeNames::TokyoNightStorm,
            [
                Color::Rgb(247, 118, 142),
                Color::Rgb(255, 0, 124),
                Color::Rgb(187, 154, 247),
                Color::Rgb(157, 124, 216),
                Color::Rgb(122, 162, 247),
                Color::Rgb(122, 162, 247),
                Color::Rgb(125, 207, 255),
                Color::Rgb(115, 218, 202),
                Color::Rgb(158, 206, 106),
                Color::Rgb(158, 206, 106),
                Color::Rgb(180, 220, 101),
                Color::Rgb(224, 175, 104),
                Color::Rgb(255, 158, 100),
                Color::Rgb(255, 158, 100),
                Color::Rgb(255, 118, 142),
                Color::Rgb(150, 111, 87),
                Color::Rgb(74, 43, 57),
                Color::Rgb(50, 69, 52),
            ],
            [
                Color::Rgb(192, 202, 245),
                Color::Rgb(169, 177, 214),
                Color::Rgb(154, 165, 206),
                Color::Rgb(128, 139, 180),
                Color::Rgb(86, 95, 137),
                Color::Rgb(68, 76, 113),
                Color::Rgb(57, 64, 99),
                Color::Rgb(49, 55, 82),
                Color::Rgb(41, 46, 66),
                Color::Rgb(36, 40, 59),
                Color::Rgb(31, 35, 53),
                Color::Rgb(65, 72, 104),
                Color::Rgb(192, 202, 245),
                Color::Rgb(203, 211, 255),
            ],
        )
    }

    pub const fn tokyo_night_light() -> Self {
        Self::from_colors(
            ThemeNames::TokyoNightLight,
            [
                Color::Rgb(140, 67, 81),
                Color::Rgb(166, 76, 159),
                Color::Rgb(92, 69, 172),
                Color::Rgb(82, 74, 176),
                Color::Rgb(52, 84, 138),
                Color::Rgb(52, 84, 138),
                Color::Rgb(22, 103, 126),
                Color::Rgb(51, 99, 92),
                Color::Rgb(72, 94, 48),
                Color::Rgb(72, 94, 48),
                Color::Rgb(105, 111, 38),
                Color::Rgb(143, 94, 21),
                Color::Rgb(150, 80, 39),
                Color::Rgb(150, 80, 39),
                Color::Rgb(158, 70, 82),
                Color::Rgb(119, 92, 72),
                Color::Rgb(250, 226, 232),
                Color::Rgb(231, 244, 220),
            ],
            [
                Color::Rgb(52, 59, 88),
                Color::Rgb(65, 72, 104),
                Color::Rgb(86, 95, 137),
                Color::Rgb(128, 139, 180),
                Color::Rgb(154, 165, 206),
                Color::Rgb(128, 139, 180),
                Color::Rgb(188, 194, 224),
                Color::Rgb(211, 216, 238),
                Color::Rgb(226, 230, 246),
                Color::Rgb(239, 241, 250),
                Color::Rgb(245, 247, 255),
                Color::Rgb(211, 216, 238),
                Color::Rgb(52, 59, 88),
                Color::Rgb(31, 35, 53),
            ],
        )
    }

    pub const fn github_dark() -> Self {
        Self::from_colors(
            ThemeNames::GithubDark,
            [
                Color::Rgb(255, 123, 114),
                Color::Rgb(255, 123, 206),
                Color::Rgb(188, 140, 255),
                Color::Rgb(161, 126, 232),
                Color::Rgb(88, 166, 255),
                Color::Rgb(88, 166, 255),
                Color::Rgb(57, 197, 182),
                Color::Rgb(57, 197, 182),
                Color::Rgb(63, 185, 80),
                Color::Rgb(63, 185, 80),
                Color::Rgb(126, 231, 135),
                Color::Rgb(210, 153, 34),
                Color::Rgb(255, 166, 87),
                Color::Rgb(255, 166, 87),
                Color::Rgb(255, 129, 117),
                Color::Rgb(151, 117, 87),
                Color::Rgb(74, 38, 39),
                Color::Rgb(37, 66, 43),
            ],
            [
                Color::Rgb(240, 246, 252),
                Color::Rgb(201, 209, 217),
                Color::Rgb(177, 186, 196),
                Color::Rgb(139, 148, 158),
                Color::Rgb(110, 118, 129),
                Color::Rgb(88, 96, 105),
                Color::Rgb(48, 54, 61),
                Color::Rgb(33, 38, 45),
                Color::Rgb(22, 27, 34),
                Color::Rgb(17, 23, 31),
                Color::Rgb(13, 17, 23),
                Color::Rgb(48, 54, 61),
                Color::Rgb(201, 209, 217),
                Color::Rgb(240, 246, 252),
            ],
        )
    }

    pub const fn github_light() -> Self {
        Self::from_colors(
            ThemeNames::GithubLight,
            [
                Color::Rgb(207, 34, 46),
                Color::Rgb(191, 57, 137),
                Color::Rgb(130, 80, 223),
                Color::Rgb(102, 57, 186),
                Color::Rgb(9, 105, 218),
                Color::Rgb(9, 105, 218),
                Color::Rgb(31, 136, 161),
                Color::Rgb(26, 127, 55),
                Color::Rgb(26, 127, 55),
                Color::Rgb(26, 127, 55),
                Color::Rgb(79, 140, 28),
                Color::Rgb(154, 103, 0),
                Color::Rgb(188, 76, 0),
                Color::Rgb(188, 76, 0),
                Color::Rgb(196, 72, 63),
                Color::Rgb(120, 92, 70),
                Color::Rgb(255, 232, 232),
                Color::Rgb(230, 246, 234),
            ],
            [
                Color::Rgb(36, 41, 47),
                Color::Rgb(87, 96, 106),
                Color::Rgb(101, 109, 118),
                Color::Rgb(140, 149, 159),
                Color::Rgb(175, 184, 193),
                Color::Rgb(101, 109, 118),
                Color::Rgb(208, 215, 222),
                Color::Rgb(230, 236, 242),
                Color::Rgb(246, 248, 250),
                Color::Rgb(250, 251, 252),
                Color::Rgb(255, 255, 255),
                Color::Rgb(208, 215, 222),
                Color::Rgb(36, 41, 47),
                Color::Rgb(9, 105, 218),
            ],
        )
    }

    pub const fn github_dark_dimmed() -> Self {
        Self::from_colors(
            ThemeNames::GithubDarkDimmed,
            [
                Color::Rgb(255, 147, 147),
                Color::Rgb(229, 126, 214),
                Color::Rgb(198, 156, 246),
                Color::Rgb(177, 139, 229),
                Color::Rgb(83, 155, 245),
                Color::Rgb(83, 155, 245),
                Color::Rgb(57, 197, 182),
                Color::Rgb(57, 197, 182),
                Color::Rgb(87, 171, 90),
                Color::Rgb(87, 171, 90),
                Color::Rgb(178, 208, 91),
                Color::Rgb(218, 172, 80),
                Color::Rgb(230, 126, 34),
                Color::Rgb(230, 126, 34),
                Color::Rgb(255, 147, 147),
                Color::Rgb(157, 126, 96),
                Color::Rgb(85, 49, 52),
                Color::Rgb(50, 72, 54),
            ],
            [
                Color::Rgb(201, 209, 217),
                Color::Rgb(173, 186, 199),
                Color::Rgb(144, 157, 171),
                Color::Rgb(118, 131, 144),
                Color::Rgb(99, 110, 123),
                Color::Rgb(84, 94, 105),
                Color::Rgb(68, 76, 86),
                Color::Rgb(55, 62, 71),
                Color::Rgb(45, 51, 59),
                Color::Rgb(40, 45, 52),
                Color::Rgb(34, 39, 46),
                Color::Rgb(68, 76, 86),
                Color::Rgb(173, 186, 199),
                Color::Rgb(201, 209, 217),
            ],
        )
    }

    pub const fn night_owl() -> Self {
        Self::from_colors(
            ThemeNames::NightOwl,
            [
                Color::Rgb(239, 83, 80),
                Color::Rgb(199, 146, 234),
                Color::Rgb(199, 146, 234),
                Color::Rgb(173, 124, 225),
                Color::Rgb(130, 170, 255),
                Color::Rgb(130, 170, 255),
                Color::Rgb(127, 219, 202),
                Color::Rgb(33, 199, 168),
                Color::Rgb(173, 219, 103),
                Color::Rgb(173, 219, 103),
                Color::Rgb(197, 231, 114),
                Color::Rgb(236, 196, 141),
                Color::Rgb(247, 140, 108),
                Color::Rgb(247, 140, 108),
                Color::Rgb(255, 99, 102),
                Color::Rgb(151, 115, 86),
                Color::Rgb(63, 36, 47),
                Color::Rgb(38, 63, 43),
            ],
            [
                Color::Rgb(214, 222, 235),
                Color::Rgb(190, 202, 219),
                Color::Rgb(127, 142, 163),
                Color::Rgb(99, 114, 135),
                Color::Rgb(91, 112, 145),
                Color::Rgb(72, 89, 112),
                Color::Rgb(45, 66, 92),
                Color::Rgb(31, 50, 74),
                Color::Rgb(10, 38, 62),
                Color::Rgb(4, 31, 53),
                Color::Rgb(1, 22, 39),
                Color::Rgb(45, 66, 92),
                Color::Rgb(214, 222, 235),
                Color::Rgb(255, 255, 255),
            ],
        )
    }

    pub const fn light_owl() -> Self {
        Self::from_colors(
            ThemeNames::LightOwl,
            [
                Color::Rgb(222, 61, 59),
                Color::Rgb(191, 57, 137),
                Color::Rgb(153, 76, 195),
                Color::Rgb(126, 78, 177),
                Color::Rgb(72, 118, 214),
                Color::Rgb(72, 118, 214),
                Color::Rgb(8, 145, 106),
                Color::Rgb(42, 162, 152),
                Color::Rgb(42, 162, 152),
                Color::Rgb(42, 162, 152),
                Color::Rgb(86, 150, 30),
                Color::Rgb(218, 170, 1),
                Color::Rgb(228, 118, 0),
                Color::Rgb(228, 118, 0),
                Color::Rgb(214, 74, 74),
                Color::Rgb(119, 88, 66),
                Color::Rgb(255, 232, 232),
                Color::Rgb(226, 246, 240),
            ],
            [
                Color::Rgb(64, 63, 83),
                Color::Rgb(83, 82, 103),
                Color::Rgb(103, 102, 123),
                Color::Rgb(130, 130, 150),
                Color::Rgb(166, 166, 184),
                Color::Rgb(103, 102, 123),
                Color::Rgb(207, 211, 220),
                Color::Rgb(226, 229, 235),
                Color::Rgb(240, 242, 246),
                Color::Rgb(247, 248, 250),
                Color::Rgb(251, 251, 251),
                Color::Rgb(207, 211, 220),
                Color::Rgb(64, 63, 83),
                Color::Rgb(36, 41, 57),
            ],
        )
    }

    pub const fn ayu_dark() -> Self {
        Self::from_colors(
            ThemeNames::AyuDark,
            [
                Color::Rgb(255, 51, 51),
                Color::Rgb(255, 119, 187),
                Color::Rgb(223, 191, 255),
                Color::Rgb(190, 162, 235),
                Color::Rgb(89, 194, 255),
                Color::Rgb(89, 194, 255),
                Color::Rgb(149, 230, 203),
                Color::Rgb(95, 215, 178),
                Color::Rgb(194, 217, 76),
                Color::Rgb(194, 217, 76),
                Color::Rgb(210, 230, 80),
                Color::Rgb(230, 180, 80),
                Color::Rgb(255, 143, 64),
                Color::Rgb(255, 143, 64),
                Color::Rgb(255, 111, 88),
                Color::Rgb(151, 114, 83),
                Color::Rgb(66, 31, 31),
                Color::Rgb(49, 63, 31),
            ],
            [
                Color::Rgb(230, 225, 207),
                Color::Rgb(179, 177, 173),
                Color::Rgb(143, 148, 150),
                Color::Rgb(112, 122, 126),
                Color::Rgb(86, 95, 101),
                Color::Rgb(76, 86, 94),
                Color::Rgb(45, 55, 63),
                Color::Rgb(31, 39, 48),
                Color::Rgb(19, 28, 37),
                Color::Rgb(13, 20, 29),
                Color::Rgb(10, 14, 20),
                Color::Rgb(45, 55, 63),
                Color::Rgb(179, 177, 173),
                Color::Rgb(230, 225, 207),
            ],
        )
    }

    pub const fn ayu_mirage() -> Self {
        Self::from_colors(
            ThemeNames::AyuMirage,
            [
                Color::Rgb(242, 135, 121),
                Color::Rgb(255, 173, 214),
                Color::Rgb(223, 191, 255),
                Color::Rgb(190, 162, 235),
                Color::Rgb(115, 208, 255),
                Color::Rgb(115, 208, 255),
                Color::Rgb(149, 230, 203),
                Color::Rgb(95, 215, 178),
                Color::Rgb(213, 255, 128),
                Color::Rgb(213, 255, 128),
                Color::Rgb(230, 255, 145),
                Color::Rgb(255, 209, 115),
                Color::Rgb(255, 173, 102),
                Color::Rgb(255, 173, 102),
                Color::Rgb(255, 145, 130),
                Color::Rgb(162, 126, 92),
                Color::Rgb(73, 44, 47),
                Color::Rgb(50, 71, 42),
            ],
            [
                Color::Rgb(242, 243, 240),
                Color::Rgb(203, 204, 198),
                Color::Rgb(171, 176, 179),
                Color::Rgb(140, 150, 157),
                Color::Rgb(112, 124, 135),
                Color::Rgb(91, 103, 116),
                Color::Rgb(62, 74, 89),
                Color::Rgb(49, 59, 74),
                Color::Rgb(39, 49, 63),
                Color::Rgb(35, 43, 56),
                Color::Rgb(31, 36, 48),
                Color::Rgb(62, 74, 89),
                Color::Rgb(203, 204, 198),
                Color::Rgb(242, 243, 240),
            ],
        )
    }

    pub const fn ayu_light() -> Self {
        Self::from_colors(
            ThemeNames::AyuLight,
            [
                Color::Rgb(240, 113, 120),
                Color::Rgb(166, 122, 204),
                Color::Rgb(163, 122, 204),
                Color::Rgb(135, 107, 188),
                Color::Rgb(54, 163, 217),
                Color::Rgb(54, 163, 217),
                Color::Rgb(76, 191, 153),
                Color::Rgb(76, 191, 153),
                Color::Rgb(134, 179, 0),
                Color::Rgb(134, 179, 0),
                Color::Rgb(111, 160, 0),
                Color::Rgb(242, 174, 73),
                Color::Rgb(250, 141, 62),
                Color::Rgb(250, 141, 62),
                Color::Rgb(230, 95, 95),
                Color::Rgb(124, 91, 69),
                Color::Rgb(255, 232, 232),
                Color::Rgb(233, 247, 219),
            ],
            [
                Color::Rgb(92, 103, 115),
                Color::Rgb(111, 121, 132),
                Color::Rgb(130, 139, 150),
                Color::Rgb(154, 162, 171),
                Color::Rgb(177, 184, 192),
                Color::Rgb(130, 139, 150),
                Color::Rgb(209, 214, 220),
                Color::Rgb(229, 232, 236),
                Color::Rgb(242, 244, 246),
                Color::Rgb(247, 248, 249),
                Color::Rgb(250, 250, 250),
                Color::Rgb(209, 214, 220),
                Color::Rgb(92, 103, 115),
                Color::Rgb(54, 64, 74),
            ],
        )
    }

    pub const fn material() -> Self {
        Self::from_colors(
            ThemeNames::Material,
            [
                Color::Rgb(255, 83, 112),
                Color::Rgb(255, 83, 112),
                Color::Rgb(199, 146, 234),
                Color::Rgb(171, 124, 225),
                Color::Rgb(130, 170, 255),
                Color::Rgb(130, 170, 255),
                Color::Rgb(137, 221, 255),
                Color::Rgb(100, 255, 218),
                Color::Rgb(195, 232, 141),
                Color::Rgb(195, 232, 141),
                Color::Rgb(221, 245, 151),
                Color::Rgb(255, 203, 107),
                Color::Rgb(247, 140, 108),
                Color::Rgb(247, 140, 108),
                Color::Rgb(255, 111, 128),
                Color::Rgb(151, 115, 86),
                Color::Rgb(70, 39, 47),
                Color::Rgb(45, 70, 45),
            ],
            [
                Color::Rgb(238, 255, 255),
                Color::Rgb(203, 214, 214),
                Color::Rgb(176, 190, 197),
                Color::Rgb(128, 151, 160),
                Color::Rgb(96, 125, 139),
                Color::Rgb(84, 110, 122),
                Color::Rgb(55, 71, 79),
                Color::Rgb(47, 61, 68),
                Color::Rgb(38, 50, 56),
                Color::Rgb(33, 43, 48),
                Color::Rgb(27, 36, 40),
                Color::Rgb(55, 71, 79),
                Color::Rgb(238, 255, 255),
                Color::Rgb(255, 255, 255),
            ],
        )
    }

    pub const fn palenight() -> Self {
        Self::from_colors(
            ThemeNames::Palenight,
            [
                Color::Rgb(255, 83, 112),
                Color::Rgb(255, 83, 112),
                Color::Rgb(199, 146, 234),
                Color::Rgb(171, 124, 225),
                Color::Rgb(130, 170, 255),
                Color::Rgb(130, 170, 255),
                Color::Rgb(137, 221, 255),
                Color::Rgb(100, 255, 218),
                Color::Rgb(195, 232, 141),
                Color::Rgb(195, 232, 141),
                Color::Rgb(221, 245, 151),
                Color::Rgb(255, 203, 107),
                Color::Rgb(247, 140, 108),
                Color::Rgb(247, 140, 108),
                Color::Rgb(255, 111, 128),
                Color::Rgb(151, 115, 86),
                Color::Rgb(74, 39, 52),
                Color::Rgb(49, 72, 50),
            ],
            [
                Color::Rgb(238, 255, 255),
                Color::Rgb(203, 214, 214),
                Color::Rgb(168, 181, 194),
                Color::Rgb(128, 142, 160),
                Color::Rgb(103, 114, 142),
                Color::Rgb(83, 92, 117),
                Color::Rgb(68, 76, 102),
                Color::Rgb(57, 63, 87),
                Color::Rgb(45, 50, 70),
                Color::Rgb(41, 45, 62),
                Color::Rgb(35, 39, 54),
                Color::Rgb(68, 76, 102),
                Color::Rgb(238, 255, 255),
                Color::Rgb(255, 255, 255),
            ],
        )
    }

    pub const fn rose_pine() -> Self {
        Self::from_colors(
            ThemeNames::RosePine,
            [
                Color::Rgb(235, 111, 146),
                Color::Rgb(235, 188, 186),
                Color::Rgb(196, 167, 231),
                Color::Rgb(156, 136, 205),
                Color::Rgb(49, 116, 143),
                Color::Rgb(49, 116, 143),
                Color::Rgb(156, 207, 216),
                Color::Rgb(156, 207, 216),
                Color::Rgb(82, 145, 118),
                Color::Rgb(82, 145, 118),
                Color::Rgb(118, 165, 90),
                Color::Rgb(246, 193, 119),
                Color::Rgb(234, 154, 151),
                Color::Rgb(234, 154, 151),
                Color::Rgb(235, 111, 146),
                Color::Rgb(150, 117, 92),
                Color::Rgb(70, 39, 54),
                Color::Rgb(39, 61, 48),
            ],
            [
                Color::Rgb(224, 222, 244),
                Color::Rgb(202, 198, 231),
                Color::Rgb(144, 140, 170),
                Color::Rgb(110, 106, 134),
                Color::Rgb(82, 79, 103),
                Color::Rgb(64, 61, 82),
                Color::Rgb(38, 35, 58),
                Color::Rgb(31, 29, 46),
                Color::Rgb(31, 29, 46),
                Color::Rgb(28, 26, 39),
                Color::Rgb(25, 23, 36),
                Color::Rgb(38, 35, 58),
                Color::Rgb(224, 222, 244),
                Color::Rgb(245, 244, 255),
            ],
        )
    }

    pub const fn rose_pine_moon() -> Self {
        Self::from_colors(
            ThemeNames::RosePineMoon,
            [
                Color::Rgb(235, 111, 146),
                Color::Rgb(234, 154, 151),
                Color::Rgb(196, 167, 231),
                Color::Rgb(156, 136, 205),
                Color::Rgb(62, 143, 176),
                Color::Rgb(62, 143, 176),
                Color::Rgb(156, 207, 216),
                Color::Rgb(156, 207, 216),
                Color::Rgb(82, 145, 118),
                Color::Rgb(82, 145, 118),
                Color::Rgb(118, 165, 90),
                Color::Rgb(246, 193, 119),
                Color::Rgb(234, 154, 151),
                Color::Rgb(234, 154, 151),
                Color::Rgb(235, 111, 146),
                Color::Rgb(150, 117, 92),
                Color::Rgb(73, 43, 58),
                Color::Rgb(43, 65, 52),
            ],
            [
                Color::Rgb(224, 222, 244),
                Color::Rgb(202, 198, 231),
                Color::Rgb(144, 140, 170),
                Color::Rgb(110, 106, 134),
                Color::Rgb(82, 79, 103),
                Color::Rgb(64, 61, 82),
                Color::Rgb(57, 53, 82),
                Color::Rgb(42, 39, 63),
                Color::Rgb(42, 39, 63),
                Color::Rgb(36, 33, 54),
                Color::Rgb(35, 33, 54),
                Color::Rgb(57, 53, 82),
                Color::Rgb(224, 222, 244),
                Color::Rgb(245, 244, 255),
            ],
        )
    }

    pub const fn rose_pine_dawn() -> Self {
        Self::from_colors(
            ThemeNames::RosePineDawn,
            [
                Color::Rgb(180, 99, 122),
                Color::Rgb(215, 130, 126),
                Color::Rgb(144, 122, 169),
                Color::Rgb(120, 102, 148),
                Color::Rgb(40, 105, 131),
                Color::Rgb(40, 105, 131),
                Color::Rgb(86, 148, 159),
                Color::Rgb(86, 148, 159),
                Color::Rgb(70, 129, 111),
                Color::Rgb(70, 129, 111),
                Color::Rgb(96, 135, 82),
                Color::Rgb(234, 157, 52),
                Color::Rgb(215, 130, 126),
                Color::Rgb(215, 130, 126),
                Color::Rgb(180, 99, 122),
                Color::Rgb(135, 96, 78),
                Color::Rgb(255, 229, 235),
                Color::Rgb(227, 244, 235),
            ],
            [
                Color::Rgb(87, 82, 121),
                Color::Rgb(107, 102, 139),
                Color::Rgb(121, 117, 147),
                Color::Rgb(152, 147, 165),
                Color::Rgb(188, 181, 196),
                Color::Rgb(152, 147, 165),
                Color::Rgb(220, 211, 205),
                Color::Rgb(242, 233, 225),
                Color::Rgb(250, 244, 237),
                Color::Rgb(255, 250, 243),
                Color::Rgb(250, 244, 237),
                Color::Rgb(220, 211, 205),
                Color::Rgb(87, 82, 121),
                Color::Rgb(40, 35, 60),
            ],
        )
    }

    pub const fn kanagawa_wave() -> Self {
        Self::from_colors(
            ThemeNames::KanagawaWave,
            [
                Color::Rgb(195, 64, 67),
                Color::Rgb(210, 126, 153),
                Color::Rgb(149, 127, 184),
                Color::Rgb(126, 107, 162),
                Color::Rgb(126, 156, 216),
                Color::Rgb(126, 156, 216),
                Color::Rgb(106, 149, 137),
                Color::Rgb(106, 149, 137),
                Color::Rgb(118, 148, 106),
                Color::Rgb(118, 148, 106),
                Color::Rgb(138, 168, 101),
                Color::Rgb(192, 163, 110),
                Color::Rgb(255, 160, 102),
                Color::Rgb(255, 160, 102),
                Color::Rgb(210, 126, 126),
                Color::Rgb(150, 116, 88),
                Color::Rgb(64, 34, 37),
                Color::Rgb(40, 56, 42),
            ],
            [
                Color::Rgb(220, 215, 186),
                Color::Rgb(199, 192, 165),
                Color::Rgb(169, 162, 138),
                Color::Rgb(139, 132, 112),
                Color::Rgb(114, 107, 90),
                Color::Rgb(92, 87, 74),
                Color::Rgb(54, 52, 63),
                Color::Rgb(42, 42, 55),
                Color::Rgb(34, 34, 44),
                Color::Rgb(30, 30, 39),
                Color::Rgb(31, 31, 40),
                Color::Rgb(54, 52, 63),
                Color::Rgb(220, 215, 186),
                Color::Rgb(255, 250, 220),
            ],
        )
    }

    pub const fn kanagawa_dragon() -> Self {
        Self::from_colors(
            ThemeNames::KanagawaDragon,
            [
                Color::Rgb(196, 116, 110),
                Color::Rgb(160, 122, 148),
                Color::Rgb(162, 146, 163),
                Color::Rgb(136, 122, 148),
                Color::Rgb(139, 164, 176),
                Color::Rgb(139, 164, 176),
                Color::Rgb(142, 164, 162),
                Color::Rgb(142, 164, 162),
                Color::Rgb(138, 154, 123),
                Color::Rgb(138, 154, 123),
                Color::Rgb(156, 164, 122),
                Color::Rgb(196, 178, 138),
                Color::Rgb(185, 141, 123),
                Color::Rgb(185, 141, 123),
                Color::Rgb(204, 126, 116),
                Color::Rgb(142, 116, 88),
                Color::Rgb(63, 38, 39),
                Color::Rgb(43, 57, 43),
            ],
            [
                Color::Rgb(197, 201, 197),
                Color::Rgb(178, 184, 178),
                Color::Rgb(148, 154, 148),
                Color::Rgb(120, 128, 120),
                Color::Rgb(94, 101, 94),
                Color::Rgb(76, 82, 76),
                Color::Rgb(54, 50, 50),
                Color::Rgb(42, 38, 38),
                Color::Rgb(34, 30, 30),
                Color::Rgb(28, 25, 25),
                Color::Rgb(24, 22, 22),
                Color::Rgb(54, 50, 50),
                Color::Rgb(197, 201, 197),
                Color::Rgb(230, 235, 230),
            ],
        )
    }

    pub const fn kanagawa_lotus() -> Self {
        Self::from_colors(
            ThemeNames::KanagawaLotus,
            [
                Color::Rgb(200, 64, 83),
                Color::Rgb(179, 91, 121),
                Color::Rgb(144, 122, 169),
                Color::Rgb(120, 102, 148),
                Color::Rgb(77, 105, 155),
                Color::Rgb(77, 105, 155),
                Color::Rgb(77, 105, 155),
                Color::Rgb(77, 130, 118),
                Color::Rgb(111, 137, 78),
                Color::Rgb(111, 137, 78),
                Color::Rgb(119, 113, 63),
                Color::Rgb(119, 113, 63),
                Color::Rgb(204, 109, 0),
                Color::Rgb(204, 109, 0),
                Color::Rgb(200, 82, 88),
                Color::Rgb(128, 96, 70),
                Color::Rgb(255, 229, 229),
                Color::Rgb(232, 243, 219),
            ],
            [
                Color::Rgb(84, 84, 100),
                Color::Rgb(100, 96, 116),
                Color::Rgb(120, 116, 132),
                Color::Rgb(145, 140, 150),
                Color::Rgb(172, 166, 166),
                Color::Rgb(120, 116, 132),
                Color::Rgb(211, 200, 165),
                Color::Rgb(226, 217, 185),
                Color::Rgb(242, 236, 188),
                Color::Rgb(248, 241, 208),
                Color::Rgb(242, 236, 188),
                Color::Rgb(211, 200, 165),
                Color::Rgb(84, 84, 100),
                Color::Rgb(45, 45, 60),
            ],
        )
    }

    pub const fn everforest_dark() -> Self {
        Self::from_colors(
            ThemeNames::EverforestDark,
            [
                Color::Rgb(230, 126, 128),
                Color::Rgb(214, 153, 182),
                Color::Rgb(214, 153, 182),
                Color::Rgb(214, 153, 182),
                Color::Rgb(127, 187, 179),
                Color::Rgb(127, 187, 179),
                Color::Rgb(131, 192, 146),
                Color::Rgb(131, 192, 146),
                Color::Rgb(167, 192, 128),
                Color::Rgb(167, 192, 128),
                Color::Rgb(167, 192, 128),
                Color::Rgb(219, 188, 127),
                Color::Rgb(219, 188, 127),
                Color::Rgb(230, 152, 117),
                Color::Rgb(230, 126, 128),
                Color::Rgb(230, 152, 117),
                Color::Rgb(81, 64, 69),
                Color::Rgb(66, 80, 71),
            ],
            [
                Color::Rgb(253, 246, 227),
                Color::Rgb(211, 198, 170),
                Color::Rgb(157, 169, 160),
                Color::Rgb(133, 146, 137),
                Color::Rgb(122, 132, 120),
                Color::Rgb(86, 99, 95),
                Color::Rgb(79, 88, 94),
                Color::Rgb(61, 72, 77),
                Color::Rgb(52, 63, 68),
                Color::Rgb(35, 42, 46),
                Color::Rgb(45, 53, 59),
                Color::Rgb(79, 88, 94),
                Color::Rgb(211, 198, 170),
                Color::Rgb(253, 246, 227),
            ],
        )
    }

    pub const fn everforest_light() -> Self {
        Self::from_colors(
            ThemeNames::EverforestLight,
            [
                Color::Rgb(248, 85, 82),
                Color::Rgb(223, 105, 186),
                Color::Rgb(223, 105, 186),
                Color::Rgb(190, 86, 168),
                Color::Rgb(58, 148, 197),
                Color::Rgb(58, 148, 197),
                Color::Rgb(53, 167, 124),
                Color::Rgb(53, 167, 124),
                Color::Rgb(141, 161, 1),
                Color::Rgb(141, 161, 1),
                Color::Rgb(126, 150, 0),
                Color::Rgb(223, 160, 0),
                Color::Rgb(245, 125, 38),
                Color::Rgb(245, 125, 38),
                Color::Rgb(220, 84, 79),
                Color::Rgb(128, 96, 70),
                Color::Rgb(255, 230, 230),
                Color::Rgb(232, 244, 217),
            ],
            [
                Color::Rgb(92, 106, 114),
                Color::Rgb(112, 125, 132),
                Color::Rgb(132, 144, 150),
                Color::Rgb(152, 164, 168),
                Color::Rgb(180, 188, 188),
                Color::Rgb(132, 144, 150),
                Color::Rgb(211, 207, 180),
                Color::Rgb(230, 221, 190),
                Color::Rgb(239, 234, 206),
                Color::Rgb(246, 239, 218),
                Color::Rgb(253, 246, 227),
                Color::Rgb(211, 207, 180),
                Color::Rgb(92, 106, 114),
                Color::Rgb(45, 55, 59),
            ],
        )
    }

    pub const fn zenburn() -> Self {
        Self::from_colors(
            ThemeNames::Zenburn,
            [
                Color::Rgb(204, 147, 147),
                Color::Rgb(220, 140, 195),
                Color::Rgb(220, 140, 195),
                Color::Rgb(190, 120, 175),
                Color::Rgb(140, 208, 211),
                Color::Rgb(140, 208, 211),
                Color::Rgb(147, 224, 227),
                Color::Rgb(147, 224, 227),
                Color::Rgb(127, 159, 127),
                Color::Rgb(127, 159, 127),
                Color::Rgb(180, 200, 120),
                Color::Rgb(240, 223, 175),
                Color::Rgb(223, 175, 143),
                Color::Rgb(223, 175, 143),
                Color::Rgb(220, 150, 150),
                Color::Rgb(150, 120, 90),
                Color::Rgb(69, 45, 45),
                Color::Rgb(48, 62, 48),
            ],
            [
                Color::Rgb(220, 220, 204),
                Color::Rgb(200, 200, 184),
                Color::Rgb(180, 180, 164),
                Color::Rgb(160, 160, 144),
                Color::Rgb(128, 128, 112),
                Color::Rgb(112, 112, 96),
                Color::Rgb(95, 95, 79),
                Color::Rgb(79, 79, 63),
                Color::Rgb(74, 74, 58),
                Color::Rgb(68, 68, 52),
                Color::Rgb(63, 63, 63),
                Color::Rgb(95, 95, 79),
                Color::Rgb(220, 220, 204),
                Color::Rgb(245, 245, 225),
            ],
        )
    }

    pub const fn horizon() -> Self {
        Self::from_colors(
            ThemeNames::Horizon,
            [
                Color::Rgb(233, 86, 120),
                Color::Rgb(238, 100, 172),
                Color::Rgb(181, 126, 220),
                Color::Rgb(155, 105, 195),
                Color::Rgb(38, 187, 217),
                Color::Rgb(38, 187, 217),
                Color::Rgb(89, 225, 227),
                Color::Rgb(89, 225, 227),
                Color::Rgb(41, 211, 152),
                Color::Rgb(41, 211, 152),
                Color::Rgb(120, 225, 130),
                Color::Rgb(250, 194, 154),
                Color::Rgb(250, 183, 149),
                Color::Rgb(250, 183, 149),
                Color::Rgb(233, 100, 122),
                Color::Rgb(151, 114, 88),
                Color::Rgb(68, 36, 48),
                Color::Rgb(34, 65, 48),
            ],
            [
                Color::Rgb(245, 247, 250),
                Color::Rgb(213, 216, 218),
                Color::Rgb(181, 185, 190),
                Color::Rgb(145, 151, 160),
                Color::Rgb(110, 116, 128),
                Color::Rgb(88, 94, 106),
                Color::Rgb(59, 64, 78),
                Color::Rgb(45, 50, 64),
                Color::Rgb(35, 39, 52),
                Color::Rgb(31, 34, 44),
                Color::Rgb(28, 30, 38),
                Color::Rgb(59, 64, 78),
                Color::Rgb(213, 216, 218),
                Color::Rgb(245, 247, 250),
            ],
        )
    }

    pub const fn synthwave_84() -> Self {
        Self::from_colors(
            ThemeNames::Synthwave84,
            [
                Color::Rgb(254, 68, 108),
                Color::Rgb(255, 126, 219),
                Color::Rgb(184, 147, 206),
                Color::Rgb(145, 110, 190),
                Color::Rgb(54, 249, 246),
                Color::Rgb(54, 249, 246),
                Color::Rgb(54, 249, 246),
                Color::Rgb(114, 241, 184),
                Color::Rgb(114, 241, 184),
                Color::Rgb(114, 241, 184),
                Color::Rgb(180, 255, 145),
                Color::Rgb(254, 222, 93),
                Color::Rgb(249, 126, 114),
                Color::Rgb(249, 126, 114),
                Color::Rgb(255, 126, 180),
                Color::Rgb(166, 114, 92),
                Color::Rgb(74, 34, 59),
                Color::Rgb(39, 70, 59),
            ],
            [
                Color::Rgb(255, 255, 255),
                Color::Rgb(240, 235, 255),
                Color::Rgb(210, 198, 235),
                Color::Rgb(170, 150, 205),
                Color::Rgb(132, 112, 165),
                Color::Rgb(102, 84, 135),
                Color::Rgb(73, 58, 101),
                Color::Rgb(58, 47, 82),
                Color::Rgb(49, 42, 71),
                Color::Rgb(43, 37, 62),
                Color::Rgb(38, 35, 53),
                Color::Rgb(73, 58, 101),
                Color::Rgb(240, 235, 255),
                Color::Rgb(255, 255, 255),
            ],
        )
    }

    pub const fn base16_tomorrow() -> Self {
        Self::from_colors(
            ThemeNames::Base16Tomorrow,
            [
                Color::Rgb(204, 102, 102),
                Color::Rgb(180, 120, 180),
                Color::Rgb(178, 148, 187),
                Color::Rgb(150, 120, 170),
                Color::Rgb(129, 162, 190),
                Color::Rgb(129, 162, 190),
                Color::Rgb(138, 190, 183),
                Color::Rgb(138, 190, 183),
                Color::Rgb(181, 189, 104),
                Color::Rgb(181, 189, 104),
                Color::Rgb(200, 200, 110),
                Color::Rgb(240, 198, 116),
                Color::Rgb(222, 147, 95),
                Color::Rgb(222, 147, 95),
                Color::Rgb(204, 112, 102),
                Color::Rgb(163, 104, 90),
                Color::Rgb(64, 38, 38),
                Color::Rgb(48, 61, 38),
            ],
            [
                Color::Rgb(255, 255, 255),
                Color::Rgb(197, 200, 198),
                Color::Rgb(180, 183, 181),
                Color::Rgb(150, 152, 151),
                Color::Rgb(129, 131, 131),
                Color::Rgb(112, 114, 114),
                Color::Rgb(55, 59, 65),
                Color::Rgb(40, 42, 46),
                Color::Rgb(40, 42, 46),
                Color::Rgb(34, 36, 40),
                Color::Rgb(29, 31, 33),
                Color::Rgb(55, 59, 65),
                Color::Rgb(197, 200, 198),
                Color::Rgb(255, 255, 255),
            ],
        )
    }

    pub const fn base16_ocean() -> Self {
        Self::from_colors(
            ThemeNames::Base16Ocean,
            [
                Color::Rgb(191, 97, 106),
                Color::Rgb(180, 142, 173),
                Color::Rgb(180, 142, 173),
                Color::Rgb(150, 120, 164),
                Color::Rgb(143, 161, 179),
                Color::Rgb(143, 161, 179),
                Color::Rgb(150, 181, 180),
                Color::Rgb(150, 181, 180),
                Color::Rgb(163, 190, 140),
                Color::Rgb(163, 190, 140),
                Color::Rgb(180, 200, 130),
                Color::Rgb(235, 203, 139),
                Color::Rgb(208, 135, 112),
                Color::Rgb(208, 135, 112),
                Color::Rgb(204, 112, 118),
                Color::Rgb(160, 126, 96),
                Color::Rgb(67, 43, 50),
                Color::Rgb(48, 66, 52),
            ],
            [
                Color::Rgb(239, 241, 245),
                Color::Rgb(192, 197, 206),
                Color::Rgb(172, 180, 194),
                Color::Rgb(151, 160, 176),
                Color::Rgb(124, 135, 153),
                Color::Rgb(101, 115, 138),
                Color::Rgb(79, 91, 102),
                Color::Rgb(67, 76, 88),
                Color::Rgb(52, 61, 70),
                Color::Rgb(47, 55, 64),
                Color::Rgb(43, 48, 59),
                Color::Rgb(79, 91, 102),
                Color::Rgb(192, 197, 206),
                Color::Rgb(239, 241, 245),
            ],
        )
    }

    pub const fn base16_eighties() -> Self {
        Self::from_colors(
            ThemeNames::Base16Eighties,
            [
                Color::Rgb(242, 119, 122),
                Color::Rgb(204, 153, 204),
                Color::Rgb(204, 153, 204),
                Color::Rgb(175, 128, 185),
                Color::Rgb(102, 153, 204),
                Color::Rgb(102, 153, 204),
                Color::Rgb(102, 204, 204),
                Color::Rgb(102, 204, 204),
                Color::Rgb(153, 204, 153),
                Color::Rgb(153, 204, 153),
                Color::Rgb(181, 220, 140),
                Color::Rgb(255, 204, 102),
                Color::Rgb(249, 145, 87),
                Color::Rgb(249, 145, 87),
                Color::Rgb(242, 130, 130),
                Color::Rgb(160, 116, 88),
                Color::Rgb(70, 42, 42),
                Color::Rgb(48, 65, 48),
            ],
            [
                Color::Rgb(242, 240, 236),
                Color::Rgb(211, 208, 200),
                Color::Rgb(190, 186, 178),
                Color::Rgb(168, 164, 156),
                Color::Rgb(142, 138, 132),
                Color::Rgb(116, 112, 108),
                Color::Rgb(81, 81, 81),
                Color::Rgb(57, 57, 57),
                Color::Rgb(45, 45, 45),
                Color::Rgb(39, 39, 39),
                Color::Rgb(45, 45, 45),
                Color::Rgb(81, 81, 81),
                Color::Rgb(211, 208, 200),
                Color::Rgb(242, 240, 236),
            ],
        )
    }

    pub const fn matrix() -> Self {
        Self::from_colors(
            ThemeNames::Matrix,
            [
                Color::Rgb(0, 255, 65),
                Color::Rgb(76, 255, 112),
                Color::Rgb(42, 220, 84),
                Color::Rgb(30, 190, 72),
                Color::Rgb(20, 165, 64),
                Color::Rgb(40, 210, 88),
                Color::Rgb(94, 255, 154),
                Color::Rgb(57, 230, 124),
                Color::Rgb(0, 255, 65),
                Color::Rgb(0, 220, 58),
                Color::Rgb(150, 255, 172),
                Color::Rgb(190, 255, 196),
                Color::Rgb(112, 255, 140),
                Color::Rgb(80, 235, 112),
                Color::Rgb(36, 200, 82),
                Color::Rgb(38, 130, 54),
                Color::Rgb(0, 62, 20),
                Color::Rgb(0, 75, 24),
            ],
            [
                Color::Rgb(220, 255, 224),
                Color::Rgb(185, 244, 195),
                Color::Rgb(145, 226, 160),
                Color::Rgb(108, 200, 126),
                Color::Rgb(72, 166, 92),
                Color::Rgb(48, 130, 67),
                Color::Rgb(26, 86, 42),
                Color::Rgb(15, 58, 29),
                Color::Rgb(9, 38, 20),
                Color::Rgb(5, 26, 14),
                Color::Rgb(0, 12, 6),
                Color::Rgb(26, 86, 42),
                Color::Rgb(0, 255, 65),
                Color::Rgb(220, 255, 224),
            ],
        )
    }

    pub fn presets() -> &'static [ThemePreset] {
        THEME_PRESETS
    }
}

impl ThemeNames {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Ansi => "ansi",
            Self::Monochrome => "monochrome",
            Self::DraculaDark => "dracula dark",
            Self::DraculaLight => "dracula light",
            Self::MonokaiDark => "monokai dark",
            Self::MonokaiLight => "monokai light",
            Self::CatppuccinDark => "catppuccin dark",
            Self::CatppuccinLight => "catppuccin light",
            Self::AtomDark => "atom dark",
            Self::AtomLight => "atom light",
            Self::OneDarkWarmer => "one dark warmer",
            Self::VsCodeDark => "vscode dark",
            Self::VsCodeLight => "vscode light",
            Self::SolarizedDark => "solarized dark",
            Self::SolarizedLight => "solarized light",
            Self::GruvboxDark => "gruvbox dark",
            Self::GruvboxLight => "gruvbox light",
            Self::Nord => "nord",
            Self::TokyoNight => "tokyo night",
            Self::TokyoNightStorm => "tokyo night storm",
            Self::TokyoNightLight => "tokyo night light",
            Self::GithubDark => "github dark",
            Self::GithubLight => "github light",
            Self::GithubDarkDimmed => "github dark dimmed",
            Self::NightOwl => "night owl",
            Self::LightOwl => "light owl",
            Self::AyuDark => "ayu dark",
            Self::AyuMirage => "ayu mirage",
            Self::AyuLight => "ayu light",
            Self::Material => "material",
            Self::Palenight => "palenight",
            Self::RosePine => "rose pine",
            Self::RosePineMoon => "rose pine moon",
            Self::RosePineDawn => "rose pine dawn",
            Self::KanagawaWave => "kanagawa wave",
            Self::KanagawaDragon => "kanagawa dragon",
            Self::KanagawaLotus => "kanagawa lotus",
            Self::EverforestDark => "everforest dark",
            Self::EverforestLight => "everforest light",
            Self::Zenburn => "zenburn",
            Self::Horizon => "horizon",
            Self::Synthwave84 => "synthwave 84",
            Self::Base16Tomorrow => "base16 tomorrow",
            Self::Base16Ocean => "base16 ocean",
            Self::Base16Eighties => "base16 eighties",
            Self::Matrix => "matrix",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy)]
pub struct ThemePreset {
    pub label: &'static str,
    pub theme: Theme,
}

pub const THEME_PRESETS: &[ThemePreset] = &[
    ThemePreset { label: "classic", theme: Theme::classic() },
    ThemePreset { label: "ansi", theme: Theme::ansi() },
    ThemePreset { label: "monochrome", theme: Theme::monochrome() },
    ThemePreset { label: "dracula dark", theme: Theme::dracula_dark() },
    ThemePreset { label: "dracula light", theme: Theme::dracula_light() },
    ThemePreset { label: "monokai dark", theme: Theme::monokai_dark() },
    ThemePreset { label: "monokai light", theme: Theme::monokai_light() },
    ThemePreset { label: "catppuccin dark", theme: Theme::catppuccin_dark() },
    ThemePreset { label: "catppuccin light", theme: Theme::catppuccin_light() },
    ThemePreset { label: "atom dark", theme: Theme::atom_dark() },
    ThemePreset { label: "atom light", theme: Theme::atom_light() },
    ThemePreset { label: "one dark warmer", theme: Theme::one_dark_warmer() },
    ThemePreset { label: "vscode dark", theme: Theme::vscode_dark() },
    ThemePreset { label: "vscode light", theme: Theme::vscode_light() },
    ThemePreset { label: "solarized dark", theme: Theme::solarized_dark() },
    ThemePreset { label: "solarized light", theme: Theme::solarized_light() },
    ThemePreset { label: "gruvbox dark", theme: Theme::gruvbox_dark() },
    ThemePreset { label: "gruvbox light", theme: Theme::gruvbox_light() },
    ThemePreset { label: "nord", theme: Theme::nord() },
    ThemePreset { label: "tokyo night", theme: Theme::tokyo_night() },
    ThemePreset { label: "tokyo night storm", theme: Theme::tokyo_night_storm() },
    ThemePreset { label: "tokyo night light", theme: Theme::tokyo_night_light() },
    ThemePreset { label: "github dark", theme: Theme::github_dark() },
    ThemePreset { label: "github light", theme: Theme::github_light() },
    ThemePreset { label: "github dark dimmed", theme: Theme::github_dark_dimmed() },
    ThemePreset { label: "night owl", theme: Theme::night_owl() },
    ThemePreset { label: "light owl", theme: Theme::light_owl() },
    ThemePreset { label: "ayu dark", theme: Theme::ayu_dark() },
    ThemePreset { label: "ayu mirage", theme: Theme::ayu_mirage() },
    ThemePreset { label: "ayu light", theme: Theme::ayu_light() },
    ThemePreset { label: "material", theme: Theme::material() },
    ThemePreset { label: "palenight", theme: Theme::palenight() },
    ThemePreset { label: "rose pine", theme: Theme::rose_pine() },
    ThemePreset { label: "rose pine moon", theme: Theme::rose_pine_moon() },
    ThemePreset { label: "rose pine dawn", theme: Theme::rose_pine_dawn() },
    ThemePreset { label: "kanagawa wave", theme: Theme::kanagawa_wave() },
    ThemePreset { label: "kanagawa dragon", theme: Theme::kanagawa_dragon() },
    ThemePreset { label: "kanagawa lotus", theme: Theme::kanagawa_lotus() },
    ThemePreset { label: "everforest dark", theme: Theme::everforest_dark() },
    ThemePreset { label: "everforest light", theme: Theme::everforest_light() },
    ThemePreset { label: "zenburn", theme: Theme::zenburn() },
    ThemePreset { label: "horizon", theme: Theme::horizon() },
    ThemePreset { label: "synthwave 84", theme: Theme::synthwave_84() },
    ThemePreset { label: "base16 tomorrow", theme: Theme::base16_tomorrow() },
    ThemePreset { label: "base16 ocean", theme: Theme::base16_ocean() },
    ThemePreset { label: "base16 eighties", theme: Theme::base16_eighties() },
    ThemePreset { label: "matrix", theme: Theme::matrix() },
];

fn theme_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("theme.json");
    path
}

#[derive(Facet)]
struct ThemeConfig {
    label: String,
    #[facet(default)]
    colors: ThemeColorConfig,
}

#[derive(Clone, Default, Facet)]
struct ThemeColorConfig {
    #[facet(default)]
    red: Option<String>,
    #[facet(default)]
    pink: Option<String>,
    #[facet(default)]
    purple: Option<String>,
    #[facet(default)]
    durple: Option<String>,
    #[facet(default)]
    indigo: Option<String>,
    #[facet(default)]
    blue: Option<String>,
    #[facet(default)]
    cyan: Option<String>,
    #[facet(default)]
    teal: Option<String>,
    #[facet(default)]
    green: Option<String>,
    #[facet(default)]
    grass: Option<String>,
    #[facet(default)]
    lime: Option<String>,
    #[facet(default)]
    yellow: Option<String>,
    #[facet(default)]
    amber: Option<String>,
    #[facet(default)]
    orange: Option<String>,
    #[facet(default)]
    grapefruit: Option<String>,
    #[facet(default)]
    brown: Option<String>,
    #[facet(default)]
    dark_red: Option<String>,
    #[facet(default)]
    light_green_900: Option<String>,
    #[facet(default)]
    grey_50: Option<String>,
    #[facet(default)]
    grey_100: Option<String>,
    #[facet(default)]
    grey_200: Option<String>,
    #[facet(default)]
    grey_300: Option<String>,
    #[facet(default)]
    grey_400: Option<String>,
    #[facet(default)]
    grey_500: Option<String>,
    #[facet(default)]
    grey_600: Option<String>,
    #[facet(default)]
    grey_700: Option<String>,
    #[facet(default)]
    grey_800: Option<String>,
    #[facet(default)]
    grey_900: Option<String>,
    #[facet(default)]
    grey_950: Option<String>,
    #[facet(default)]
    border: Option<String>,
    #[facet(default)]
    text: Option<String>,
    #[facet(default)]
    highlighted: Option<String>,
}

fn normalize_color_name(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(red, green, blue))
}

fn parse_color(value: &str) -> Option<Color> {
    if let Some(color) = parse_hex_color(value) {
        return Some(color);
    }

    let normalized = normalize_color_name(value);
    match normalized.as_str() {
        "reset" => Some(Color::Reset),
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "light_red" => Some(Color::LightRed),
        "light_green" => Some(Color::LightGreen),
        "light_yellow" => Some(Color::LightYellow),
        "light_blue" => Some(Color::LightBlue),
        "light_magenta" => Some(Color::LightMagenta),
        "light_cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        value if value.starts_with("indexed_") => value.strip_prefix("indexed_").and_then(|index| index.parse::<u8>().ok()).map(Color::Indexed),
        value if value.starts_with("indexed:") => value.strip_prefix("indexed:").and_then(|index| index.parse::<u8>().ok()).map(Color::Indexed),
        _ => None,
    }
}

fn color_to_string(color: Color) -> String {
    match color {
        Color::Reset => "reset".to_string(),
        Color::Black => "black".to_string(),
        Color::Red => "red".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Blue => "blue".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Gray => "gray".to_string(),
        Color::DarkGray => "dark_gray".to_string(),
        Color::LightRed => "light_red".to_string(),
        Color::LightGreen => "light_green".to_string(),
        Color::LightYellow => "light_yellow".to_string(),
        Color::LightBlue => "light_blue".to_string(),
        Color::LightMagenta => "light_magenta".to_string(),
        Color::LightCyan => "light_cyan".to_string(),
        Color::White => "white".to_string(),
        Color::Rgb(red, green, blue) => format!("#{red:02x}{green:02x}{blue:02x}"),
        Color::Indexed(index) => format!("indexed:{index}"),
    }
}

fn apply_color(slot: &mut Color, value: &Option<String>) {
    if let Some(color) = value.as_deref().and_then(parse_color) {
        *slot = color;
    }
}

fn apply_colors(theme: &mut Theme, colors: &ThemeColorConfig) {
    apply_color(&mut theme.COLOR_RED, &colors.red);
    apply_color(&mut theme.COLOR_PINK, &colors.pink);
    apply_color(&mut theme.COLOR_PURPLE, &colors.purple);
    apply_color(&mut theme.COLOR_DURPLE, &colors.durple);
    apply_color(&mut theme.COLOR_INDIGO, &colors.indigo);
    apply_color(&mut theme.COLOR_BLUE, &colors.blue);
    apply_color(&mut theme.COLOR_CYAN, &colors.cyan);
    apply_color(&mut theme.COLOR_TEAL, &colors.teal);
    apply_color(&mut theme.COLOR_GREEN, &colors.green);
    apply_color(&mut theme.COLOR_GRASS, &colors.grass);
    apply_color(&mut theme.COLOR_LIME, &colors.lime);
    apply_color(&mut theme.COLOR_YELLOW, &colors.yellow);
    apply_color(&mut theme.COLOR_AMBER, &colors.amber);
    apply_color(&mut theme.COLOR_ORANGE, &colors.orange);
    apply_color(&mut theme.COLOR_GRAPEFRUIT, &colors.grapefruit);
    apply_color(&mut theme.COLOR_BROWN, &colors.brown);
    apply_color(&mut theme.COLOR_DARK_RED, &colors.dark_red);
    apply_color(&mut theme.COLOR_LIGHT_GREEN_900, &colors.light_green_900);
    apply_color(&mut theme.COLOR_GREY_50, &colors.grey_50);
    apply_color(&mut theme.COLOR_GREY_100, &colors.grey_100);
    apply_color(&mut theme.COLOR_GREY_200, &colors.grey_200);
    apply_color(&mut theme.COLOR_GREY_300, &colors.grey_300);
    apply_color(&mut theme.COLOR_GREY_400, &colors.grey_400);
    apply_color(&mut theme.COLOR_GREY_500, &colors.grey_500);
    apply_color(&mut theme.COLOR_GREY_600, &colors.grey_600);
    apply_color(&mut theme.COLOR_GREY_700, &colors.grey_700);
    apply_color(&mut theme.COLOR_GREY_800, &colors.grey_800);
    apply_color(&mut theme.COLOR_GREY_900, &colors.grey_900);
    apply_color(&mut theme.COLOR_GREY_950, &colors.grey_950);
    apply_color(&mut theme.COLOR_BORDER, &colors.border);
    apply_color(&mut theme.COLOR_TEXT, &colors.text);
    apply_color(&mut theme.COLOR_HIGHLIGHTED, &colors.highlighted);
}

fn theme_color_config(theme: &Theme) -> ThemeColorConfig {
    ThemeColorConfig {
        red: Some(color_to_string(theme.COLOR_RED)),
        pink: Some(color_to_string(theme.COLOR_PINK)),
        purple: Some(color_to_string(theme.COLOR_PURPLE)),
        durple: Some(color_to_string(theme.COLOR_DURPLE)),
        indigo: Some(color_to_string(theme.COLOR_INDIGO)),
        blue: Some(color_to_string(theme.COLOR_BLUE)),
        cyan: Some(color_to_string(theme.COLOR_CYAN)),
        teal: Some(color_to_string(theme.COLOR_TEAL)),
        green: Some(color_to_string(theme.COLOR_GREEN)),
        grass: Some(color_to_string(theme.COLOR_GRASS)),
        lime: Some(color_to_string(theme.COLOR_LIME)),
        yellow: Some(color_to_string(theme.COLOR_YELLOW)),
        amber: Some(color_to_string(theme.COLOR_AMBER)),
        orange: Some(color_to_string(theme.COLOR_ORANGE)),
        grapefruit: Some(color_to_string(theme.COLOR_GRAPEFRUIT)),
        brown: Some(color_to_string(theme.COLOR_BROWN)),
        dark_red: Some(color_to_string(theme.COLOR_DARK_RED)),
        light_green_900: Some(color_to_string(theme.COLOR_LIGHT_GREEN_900)),
        grey_50: Some(color_to_string(theme.COLOR_GREY_50)),
        grey_100: Some(color_to_string(theme.COLOR_GREY_100)),
        grey_200: Some(color_to_string(theme.COLOR_GREY_200)),
        grey_300: Some(color_to_string(theme.COLOR_GREY_300)),
        grey_400: Some(color_to_string(theme.COLOR_GREY_400)),
        grey_500: Some(color_to_string(theme.COLOR_GREY_500)),
        grey_600: Some(color_to_string(theme.COLOR_GREY_600)),
        grey_700: Some(color_to_string(theme.COLOR_GREY_700)),
        grey_800: Some(color_to_string(theme.COLOR_GREY_800)),
        grey_900: Some(color_to_string(theme.COLOR_GREY_900)),
        grey_950: Some(color_to_string(theme.COLOR_GREY_950)),
        border: Some(color_to_string(theme.COLOR_BORDER)),
        text: Some(color_to_string(theme.COLOR_TEXT)),
        highlighted: Some(color_to_string(theme.COLOR_HIGHLIGHTED)),
    }
}

fn theme_config(theme: &Theme) -> ThemeConfig {
    ThemeConfig { label: theme.label().to_string(), colors: theme_color_config(theme) }
}

fn theme_from_config(config: ThemeConfig) -> Option<Theme> {
    let label = config.label.trim();
    if label.is_empty() {
        return None;
    }

    let preset = Theme::from_label(label);
    let original = preset.unwrap_or_else(Theme::classic);
    let mut theme = original;
    apply_colors(&mut theme, &config.colors);

    if preset.is_none() || !theme.colors_equal(&original) {
        theme = Theme::custom(label, theme);
    }

    Some(theme)
}

fn load_theme_from_path(path: &Path) -> Theme {
    if path.exists() {
        let contents = fs::read_to_string(path).unwrap_or_default();
        if let Ok(config) = facet_json::from_str::<ThemeConfig>(&contents)
            && let Some(theme) = theme_from_config(config)
        {
            return theme;
        }
    }

    let theme = Theme::default();
    save_theme_to_path(path, &theme);
    theme
}

fn save_theme_to_path(path: &Path, theme: &Theme) {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    let config = theme_config(theme);
    let theme_string = facet_json::to_string_pretty(&config).unwrap();
    fs::write(path, &theme_string).unwrap();
}

pub fn load_theme() -> Theme {
    load_theme_from_path(&theme_path())
}

pub fn save_theme(theme: &Theme) {
    save_theme_to_path(&theme_path(), theme);
}

#[cfg(test)]
#[path = "../tests/helpers/palette.rs"]
mod tests;
