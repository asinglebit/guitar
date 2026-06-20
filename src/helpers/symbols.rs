use facet::Facet;
use ratatui::symbols::border::Set as BorderSet;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn s(value: &str) -> String {
    value.to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Facet)]
#[repr(C)]
pub enum SymbolThemeName {
    Main,
    Ascii,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolTheme {
    pub name: SymbolThemeName,
    custom_label: Option<String>,
    pub branch: BranchSymbols,
    pub border: BorderSymbols,
    pub entity: EntitySymbols,
    pub empty_state: EmptyStateSymbols,
    pub form: FormSymbols,
    pub graph: GraphSymbols,
    pub heatmap: HeatmapSymbols,
    pub modal: ModalSymbols,
    pub scrollbar: ScrollbarSymbols,
    pub settings: SettingsSymbols,
    pub splash: SplashSymbols,
    pub status: StatusSymbols,
    pub submodule: SubmoduleSymbols,
    pub worktree: WorktreeSymbols,
    pub weekday: WeekdaySymbols,
}

impl Default for SymbolTheme {
    fn default() -> Self {
        Self::main()
    }
}

impl SymbolTheme {
    pub fn label(&self) -> &str {
        self.custom_label.as_deref().unwrap_or_else(|| self.name.label())
    }

    pub fn from_label(label: &str) -> Option<Self> {
        let normalized = normalize_symbol_theme_label(label);
        Self::presets().iter().find(|preset| preset.label == normalized).map(SymbolThemePreset::theme)
    }

    pub fn presets() -> &'static [SymbolThemePreset] {
        SYMBOL_THEME_PRESETS
    }

    pub fn main() -> Self {
        let branch = BranchSymbols { local_visible: s("●"), local_hidden: s("○"), remote_visible: s("◆"), remote_hidden: s("◇") };
        let border = BorderSymbols {
            horizontal: s("─"),
            vertical: s("│"),
            t_right: s("┤"),
            t_left: s("├"),
            top_t: s("┬"),
            bottom_t: s("┴"),
            rounded_top_right: s("╮"),
            rounded_bottom_right: s("╯"),
            rounded_top_left: s("╭"),
            rounded_bottom_left: s("╰"),
        };
        let graph = GraphSymbols {
            commit_branch: branch.local_visible.clone(),
            commit: branch.local_hidden.clone(),
            commit_stash: s("◎"),
            empty: s(" "),
            horizontal: border.horizontal.clone(),
            horizontal_dotted: s("┄"),
            vertical: border.vertical.clone(),
            vertical_dotted: s("┊"),
            merge_left_from: border.t_right.clone(),
            merge_right_from: border.rounded_top_left.clone(),
            branch_up: border.rounded_bottom_right.clone(),
            branch_up_right: border.rounded_bottom_left.clone(),
            branch_down: border.rounded_top_right.clone(),
            merge: s("•"),
            uncommitted: s("◌"),
        };
        let scrollbar = ScrollbarSymbols {
            begin: border.rounded_top_right.clone(),
            end: border.rounded_bottom_right.clone(),
            track: border.vertical.clone(),
            thumb: s("▌"),
            inactive_thumb: border.vertical.clone(),
        };
        let splash = SplashSymbols {
            logo_word_prefix: s("  guita"),
            logo_compact: s("guita╭"),
            logo_narrow: vec![
                s("                    :#   :#                 "),
                s("                         L#                 "),
                s("  .##5#^.  .#   .#  :C  #C6#   #?##:        "),
                s("  #B   #G  C#   #B  #7   B?        G#       "),
                s("  #4   B5  B5   B5  B5   B5    1B5B#G  .a###"),
                s("  #b   5?  ?B   B5  B5   B5   ##   ##  B?   "),
                s("  .#B~6G!  .#6#~G.  #5   ~##  .##Y~#.  !#   "),
                s("      .##                              !B   "),
                s("     ~G#                               ~?   "),
            ],
            logo_wide: vec![
                s("                                 :GG~        .?Y.                                "),
                s("       ....        ..      ..   .....      . ^BG: ..       .....                 "),
                s("    .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              "),
                s("   .5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^"),
                s("   7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY"),
                s("   ?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. "),
                s("   ^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     "),
                s("    ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      "),
                s("      .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      "),
                s("    .?!^^^!5G7                                                          YB5      "),
                s("    .!?JJJ?!:                                                           75?      "),
            ],
            selected_left: s("⏵ "),
            selected_right: s(" ⏴"),
            logo_corner: border.rounded_top_left.clone(),
        };

        Self {
            name: SymbolThemeName::Main,
            custom_label: None,
            branch,
            border,
            entity: EntitySymbols { folder: s(""), tag: s("\u{F04F9}"), reflog: s("\u{F0E2}") },
            empty_state: EmptyStateSymbols { mark: s("⊘") },
            form: FormSymbols { checkbox_off: s("🞎"), checkbox_on: s("🞕"), radio_off: s("🞅"), radio_on: s("🞊") },
            graph,
            heatmap: HeatmapSymbols { zero: s("⠁"), one: s("⠁"), two: s("⠃"), three: s("⠇"), four: s("⠏"), five: s("⠟"), six: s("⠿"), seven: s("⡿"), many: s("⣿") },
            modal: ModalSymbols { selected: s(">"), unselected: s(" "), mask: s("*") },
            scrollbar,
            settings: SettingsSymbols { compact_tab: s("•") },
            splash,
            status: StatusSymbols {
                added: s("+"),
                added_spaced: s("+ "),
                conflict: s("!"),
                conflict_spaced: s("! "),
                deleted: s("-"),
                deleted_spaced: s("- "),
                modified: s("~"),
                modified_spaced: s("~ "),
                renamed: s(">"),
                renamed_arrow_spaced: s("→ "),
                other: s("*"),
                other_spaced: s("  "),
            },
            submodule: SubmoduleSymbols { default: s("\u{F03D6}"), empty: s("\u{F19D9}"), dirty: s("\u{F19D8}"), uninitialized: s("\u{f487}"), stack_separator: s(" › ") },
            worktree: WorktreeSymbols { current: s("\u{F0770}"), other: s("\u{F0256}"), dirty: s("\u{F0DCE}"), locked: s("\u{F1AA8}"), invalid: s("\u{F19F9}"), empty: s("\u{F179E}") },
            weekday: WeekdaySymbols { monday: s("M"), tuesday: s("T"), wednesday: s("W"), thursday: s("T"), friday: s("F"), saturday: s("S"), sunday: s("S") },
        }
    }

    pub fn ascii() -> Self {
        let branch = BranchSymbols { local_visible: s("*"), local_hidden: s("o"), remote_visible: s("#"), remote_hidden: s(".") };
        let border = BorderSymbols {
            horizontal: s("-"),
            vertical: s("|"),
            t_right: s("+"),
            t_left: s("+"),
            top_t: s("+"),
            bottom_t: s("+"),
            rounded_top_right: s("+"),
            rounded_bottom_right: s("+"),
            rounded_top_left: s("+"),
            rounded_bottom_left: s("+"),
        };
        let graph = GraphSymbols {
            commit_branch: branch.local_visible.clone(),
            commit: branch.local_hidden.clone(),
            commit_stash: s("@"),
            empty: s(" "),
            horizontal: border.horizontal.clone(),
            horizontal_dotted: s("."),
            vertical: border.vertical.clone(),
            vertical_dotted: s(":"),
            merge_left_from: border.t_right.clone(),
            merge_right_from: border.rounded_top_left.clone(),
            branch_up: border.rounded_bottom_right.clone(),
            branch_up_right: border.rounded_bottom_left.clone(),
            branch_down: border.rounded_top_right.clone(),
            merge: s("x"),
            uncommitted: s("?"),
        };
        let scrollbar = ScrollbarSymbols { begin: s("+"), end: s("+"), track: s("|"), thumb: s("#"), inactive_thumb: s("|") };
        let mut splash = Self::main().splash;
        splash.logo_compact = s("guita+");
        splash.selected_left = s("> ");
        splash.selected_right = s(" <");
        splash.logo_corner = s("+");

        Self {
            name: SymbolThemeName::Ascii,
            custom_label: None,
            branch,
            border,
            entity: EntitySymbols { folder: s("F"), tag: s("T"), reflog: s("R") },
            empty_state: EmptyStateSymbols { mark: s("x") },
            form: FormSymbols { checkbox_off: s("[ ]"), checkbox_on: s("[x]"), radio_off: s("( )"), radio_on: s("(*)") },
            graph,
            heatmap: HeatmapSymbols { zero: s("."), one: s("."), two: s(":"), three: s("*"), four: s("o"), five: s("O"), six: s("0"), seven: s("#"), many: s("@") },
            modal: ModalSymbols { selected: s(">"), unselected: s(" "), mask: s("*") },
            scrollbar,
            settings: SettingsSymbols { compact_tab: s("*") },
            splash,
            status: StatusSymbols {
                added: s("+"),
                added_spaced: s("+ "),
                conflict: s("!"),
                conflict_spaced: s("! "),
                deleted: s("-"),
                deleted_spaced: s("- "),
                modified: s("~"),
                modified_spaced: s("~ "),
                renamed: s(">"),
                renamed_arrow_spaced: s("> "),
                other: s("*"),
                other_spaced: s("  "),
            },
            submodule: SubmoduleSymbols { default: s("S"), empty: s("S?"), dirty: s("S!"), uninitialized: s("S-"), stack_separator: s(" > ") },
            worktree: WorktreeSymbols { current: s("W"), other: s("w"), dirty: s("!"), locked: s("L"), invalid: s("?"), empty: s("W") },
            weekday: WeekdaySymbols { monday: s("M"), tuesday: s("T"), wednesday: s("W"), thursday: s("T"), friday: s("F"), saturday: s("S"), sunday: s("S") },
        }
    }

    fn custom(label: &str, mut theme: Self) -> Self {
        let label = if label.trim().is_empty() { SymbolThemeName::Custom.label() } else { label.trim() };
        theme.name = SymbolThemeName::Custom;
        theme.custom_label = Some(label.to_string());
        theme
    }

    fn symbols_equal(&self, other: &Self) -> bool {
        self.branch == other.branch
            && self.border == other.border
            && self.entity == other.entity
            && self.empty_state == other.empty_state
            && self.form == other.form
            && self.graph == other.graph
            && self.heatmap == other.heatmap
            && self.modal == other.modal
            && self.scrollbar == other.scrollbar
            && self.settings == other.settings
            && self.splash == other.splash
            && self.status == other.status
            && self.submodule == other.submodule
            && self.worktree == other.worktree
            && self.weekday == other.weekday
    }

    pub fn values(&self) -> Vec<&str> {
        let mut values = Vec::new();
        self.branch.push_values(&mut values);
        self.border.push_values(&mut values);
        self.entity.push_values(&mut values);
        self.empty_state.push_values(&mut values);
        self.form.push_values(&mut values);
        self.graph.push_values(&mut values);
        self.heatmap.push_values(&mut values);
        self.modal.push_values(&mut values);
        self.scrollbar.push_values(&mut values);
        self.settings.push_values(&mut values);
        self.splash.push_values(&mut values);
        self.status.push_values(&mut values);
        self.submodule.push_values(&mut values);
        self.worktree.push_values(&mut values);
        self.weekday.push_values(&mut values);
        values
    }
}

impl SymbolThemeName {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Ascii => "ascii",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy)]
pub struct SymbolThemePreset {
    pub label: &'static str,
    theme: fn() -> SymbolTheme,
}

impl SymbolThemePreset {
    pub fn theme(&self) -> SymbolTheme {
        (self.theme)()
    }
}

pub const SYMBOL_THEME_PRESETS: &[SymbolThemePreset] = &[SymbolThemePreset { label: "main", theme: SymbolTheme::main }, SymbolThemePreset { label: "ascii", theme: SymbolTheme::ascii }];

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct BranchSymbols {
    pub local_visible: String,
    pub local_hidden: String,
    pub remote_visible: String,
    pub remote_hidden: String,
}

impl BranchSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.local_visible.as_str(), self.local_hidden.as_str(), self.remote_visible.as_str(), self.remote_hidden.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct BorderSymbols {
    pub horizontal: String,
    pub vertical: String,
    pub t_right: String,
    pub t_left: String,
    pub top_t: String,
    pub bottom_t: String,
    pub rounded_top_right: String,
    pub rounded_bottom_right: String,
    pub rounded_top_left: String,
    pub rounded_bottom_left: String,
}

impl BorderSymbols {
    pub fn block_set(&self) -> BorderSet<'_> {
        BorderSet {
            top_left: self.rounded_top_left.as_str(),
            top_right: self.rounded_top_right.as_str(),
            bottom_left: self.rounded_bottom_left.as_str(),
            bottom_right: self.rounded_bottom_right.as_str(),
            vertical_left: self.vertical.as_str(),
            vertical_right: self.vertical.as_str(),
            horizontal_top: self.horizontal.as_str(),
            horizontal_bottom: self.horizontal.as_str(),
        }
    }

    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([
            self.horizontal.as_str(),
            self.vertical.as_str(),
            self.t_right.as_str(),
            self.t_left.as_str(),
            self.top_t.as_str(),
            self.bottom_t.as_str(),
            self.rounded_top_right.as_str(),
            self.rounded_bottom_right.as_str(),
            self.rounded_top_left.as_str(),
            self.rounded_bottom_left.as_str(),
        ]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct EntitySymbols {
    pub folder: String,
    pub tag: String,
    pub reflog: String,
}

impl EntitySymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.folder.as_str(), self.tag.as_str(), self.reflog.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct EmptyStateSymbols {
    pub mark: String,
}

impl EmptyStateSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.push(self.mark.as_str());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct FormSymbols {
    pub checkbox_off: String,
    pub checkbox_on: String,
    pub radio_off: String,
    pub radio_on: String,
}

impl FormSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.checkbox_off.as_str(), self.checkbox_on.as_str(), self.radio_off.as_str(), self.radio_on.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct GraphSymbols {
    pub commit_branch: String,
    pub commit: String,
    pub commit_stash: String,
    pub empty: String,
    pub horizontal: String,
    pub horizontal_dotted: String,
    pub vertical: String,
    pub vertical_dotted: String,
    pub merge_left_from: String,
    pub merge_right_from: String,
    pub branch_up: String,
    pub branch_up_right: String,
    pub branch_down: String,
    pub merge: String,
    pub uncommitted: String,
}

impl GraphSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([
            self.commit_branch.as_str(),
            self.commit.as_str(),
            self.commit_stash.as_str(),
            self.empty.as_str(),
            self.horizontal.as_str(),
            self.horizontal_dotted.as_str(),
            self.vertical.as_str(),
            self.vertical_dotted.as_str(),
            self.merge_left_from.as_str(),
            self.merge_right_from.as_str(),
            self.branch_up.as_str(),
            self.branch_up_right.as_str(),
            self.branch_down.as_str(),
            self.merge.as_str(),
            self.uncommitted.as_str(),
        ]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct HeatmapSymbols {
    pub zero: String,
    pub one: String,
    pub two: String,
    pub three: String,
    pub four: String,
    pub five: String,
    pub six: String,
    pub seven: String,
    pub many: String,
}

impl HeatmapSymbols {
    pub fn cell(&self, count: usize) -> &str {
        match count {
            0 => self.zero.as_str(),
            1 => self.one.as_str(),
            2 => self.two.as_str(),
            3 => self.three.as_str(),
            4 => self.four.as_str(),
            5 => self.five.as_str(),
            6 => self.six.as_str(),
            7 => self.seven.as_str(),
            _ => self.many.as_str(),
        }
    }

    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([
            self.zero.as_str(),
            self.one.as_str(),
            self.two.as_str(),
            self.three.as_str(),
            self.four.as_str(),
            self.five.as_str(),
            self.six.as_str(),
            self.seven.as_str(),
            self.many.as_str(),
        ]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct ModalSymbols {
    pub selected: String,
    pub unselected: String,
    pub mask: String,
}

impl ModalSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.selected.as_str(), self.unselected.as_str(), self.mask.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct ScrollbarSymbols {
    pub begin: String,
    pub end: String,
    pub track: String,
    pub thumb: String,
    pub inactive_thumb: String,
}

impl ScrollbarSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.begin.as_str(), self.end.as_str(), self.track.as_str(), self.thumb.as_str(), self.inactive_thumb.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct SettingsSymbols {
    pub compact_tab: String,
}

impl SettingsSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.push(self.compact_tab.as_str());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct SplashSymbols {
    pub logo_word_prefix: String,
    pub logo_compact: String,
    pub logo_narrow: Vec<String>,
    pub logo_wide: Vec<String>,
    pub selected_left: String,
    pub selected_right: String,
    pub logo_corner: String,
}

impl SplashSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.logo_word_prefix.as_str(), self.logo_compact.as_str(), self.selected_left.as_str(), self.selected_right.as_str(), self.logo_corner.as_str()]);
        values.extend(self.logo_narrow.iter().map(String::as_str));
        values.extend(self.logo_wide.iter().map(String::as_str));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct StatusSymbols {
    pub added: String,
    pub added_spaced: String,
    pub conflict: String,
    pub conflict_spaced: String,
    pub deleted: String,
    pub deleted_spaced: String,
    pub modified: String,
    pub modified_spaced: String,
    pub renamed: String,
    pub renamed_arrow_spaced: String,
    pub other: String,
    pub other_spaced: String,
}

impl StatusSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([
            self.added.as_str(),
            self.added_spaced.as_str(),
            self.conflict.as_str(),
            self.conflict_spaced.as_str(),
            self.deleted.as_str(),
            self.deleted_spaced.as_str(),
            self.modified.as_str(),
            self.modified_spaced.as_str(),
            self.renamed.as_str(),
            self.renamed_arrow_spaced.as_str(),
            self.other.as_str(),
            self.other_spaced.as_str(),
        ]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct SubmoduleSymbols {
    pub default: String,
    pub empty: String,
    pub dirty: String,
    pub uninitialized: String,
    pub stack_separator: String,
}

impl SubmoduleSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.default.as_str(), self.empty.as_str(), self.dirty.as_str(), self.uninitialized.as_str(), self.stack_separator.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct WorktreeSymbols {
    pub current: String,
    pub other: String,
    pub dirty: String,
    pub locked: String,
    pub invalid: String,
    pub empty: String,
}

impl WorktreeSymbols {
    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend([self.current.as_str(), self.other.as_str(), self.dirty.as_str(), self.locked.as_str(), self.invalid.as_str(), self.empty.as_str()]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct WeekdaySymbols {
    pub monday: String,
    pub tuesday: String,
    pub wednesday: String,
    pub thursday: String,
    pub friday: String,
    pub saturday: String,
    pub sunday: String,
}

impl WeekdaySymbols {
    pub fn labels(&self) -> [&str; 7] {
        [self.monday.as_str(), self.tuesday.as_str(), self.wednesday.as_str(), self.thursday.as_str(), self.friday.as_str(), self.saturday.as_str(), self.sunday.as_str()]
    }

    fn push_values<'a>(&'a self, values: &mut Vec<&'a str>) {
        values.extend(self.labels());
    }
}

#[derive(Facet)]
struct SymbolThemeConfig {
    label: String,
    #[facet(default)]
    symbols: SymbolConfig,
}

#[derive(Clone, Default, Facet)]
struct SymbolConfig {
    #[facet(default)]
    branch: Option<BranchSymbolConfig>,
    #[facet(default)]
    border: Option<BorderSymbolConfig>,
    #[facet(default)]
    entity: Option<EntitySymbolConfig>,
    #[facet(default)]
    empty_state: Option<EmptyStateSymbolConfig>,
    #[facet(default)]
    form: Option<FormSymbolConfig>,
    #[facet(default)]
    graph: Option<GraphSymbolConfig>,
    #[facet(default)]
    heatmap: Option<HeatmapSymbolConfig>,
    #[facet(default)]
    modal: Option<ModalSymbolConfig>,
    #[facet(default)]
    scrollbar: Option<ScrollbarSymbolConfig>,
    #[facet(default)]
    settings: Option<SettingsSymbolConfig>,
    #[facet(default)]
    splash: Option<SplashSymbolConfig>,
    #[facet(default)]
    status: Option<StatusSymbolConfig>,
    #[facet(default)]
    submodule: Option<SubmoduleSymbolConfig>,
    #[facet(default)]
    worktree: Option<WorktreeSymbolConfig>,
    #[facet(default)]
    weekday: Option<WeekdaySymbolConfig>,
}

macro_rules! optional_symbol_config {
    ($name:ident { $($field:ident),+ $(,)? }) => {
        #[derive(Clone, Default, Facet)]
        struct $name {
            $(
                #[facet(default)]
                $field: Option<String>,
            )+
        }
    };
}

optional_symbol_config!(BranchSymbolConfig { local_visible, local_hidden, remote_visible, remote_hidden });
optional_symbol_config!(BorderSymbolConfig { horizontal, vertical, t_right, t_left, top_t, bottom_t, rounded_top_right, rounded_bottom_right, rounded_top_left, rounded_bottom_left });
optional_symbol_config!(EntitySymbolConfig { folder, tag, reflog });
optional_symbol_config!(EmptyStateSymbolConfig { mark });
optional_symbol_config!(FormSymbolConfig { checkbox_off, checkbox_on, radio_off, radio_on });
optional_symbol_config!(GraphSymbolConfig {
    commit_branch,
    commit,
    commit_stash,
    empty,
    horizontal,
    horizontal_dotted,
    vertical,
    vertical_dotted,
    merge_left_from,
    merge_right_from,
    branch_up,
    branch_up_right,
    branch_down,
    merge,
    uncommitted,
});
optional_symbol_config!(HeatmapSymbolConfig { zero, one, two, three, four, five, six, seven, many });
optional_symbol_config!(ModalSymbolConfig { selected, unselected, mask });
optional_symbol_config!(ScrollbarSymbolConfig { begin, end, track, thumb, inactive_thumb });
optional_symbol_config!(SettingsSymbolConfig { compact_tab });
optional_symbol_config!(StatusSymbolConfig { added, added_spaced, conflict, conflict_spaced, deleted, deleted_spaced, modified, modified_spaced, renamed, renamed_arrow_spaced, other, other_spaced });
optional_symbol_config!(SubmoduleSymbolConfig { default, empty, dirty, uninitialized, stack_separator });
optional_symbol_config!(WorktreeSymbolConfig { current, other, dirty, locked, invalid, empty });
optional_symbol_config!(WeekdaySymbolConfig { monday, tuesday, wednesday, thursday, friday, saturday, sunday });

#[derive(Clone, Default, Facet)]
struct SplashSymbolConfig {
    #[facet(default)]
    logo_word_prefix: Option<String>,
    #[facet(default)]
    logo_compact: Option<String>,
    #[facet(default)]
    logo_narrow: Option<Vec<String>>,
    #[facet(default)]
    logo_wide: Option<Vec<String>>,
    #[facet(default)]
    selected_left: Option<String>,
    #[facet(default)]
    selected_right: Option<String>,
    #[facet(default)]
    logo_corner: Option<String>,
}

fn normalize_symbol_theme_label(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['-', '_'], " ")
}

fn apply_symbol(slot: &mut String, value: &Option<String>) {
    if let Some(value) = value {
        *slot = value.clone();
    }
}

fn apply_symbol_vec(slot: &mut Vec<String>, value: &Option<Vec<String>>) {
    if let Some(value) = value {
        *slot = value.clone();
    }
}

fn apply_symbols(theme: &mut SymbolTheme, config: &SymbolConfig) {
    if let Some(branch) = &config.branch {
        apply_symbol(&mut theme.branch.local_visible, &branch.local_visible);
        apply_symbol(&mut theme.branch.local_hidden, &branch.local_hidden);
        apply_symbol(&mut theme.branch.remote_visible, &branch.remote_visible);
        apply_symbol(&mut theme.branch.remote_hidden, &branch.remote_hidden);
    }

    if let Some(border) = &config.border {
        apply_symbol(&mut theme.border.horizontal, &border.horizontal);
        apply_symbol(&mut theme.border.vertical, &border.vertical);
        apply_symbol(&mut theme.border.t_right, &border.t_right);
        apply_symbol(&mut theme.border.t_left, &border.t_left);
        apply_symbol(&mut theme.border.top_t, &border.top_t);
        apply_symbol(&mut theme.border.bottom_t, &border.bottom_t);
        apply_symbol(&mut theme.border.rounded_top_right, &border.rounded_top_right);
        apply_symbol(&mut theme.border.rounded_bottom_right, &border.rounded_bottom_right);
        apply_symbol(&mut theme.border.rounded_top_left, &border.rounded_top_left);
        apply_symbol(&mut theme.border.rounded_bottom_left, &border.rounded_bottom_left);
    }

    if let Some(entity) = &config.entity {
        apply_symbol(&mut theme.entity.folder, &entity.folder);
        apply_symbol(&mut theme.entity.tag, &entity.tag);
        apply_symbol(&mut theme.entity.reflog, &entity.reflog);
    }
    if let Some(empty_state) = &config.empty_state {
        apply_symbol(&mut theme.empty_state.mark, &empty_state.mark);
    }

    if let Some(form) = &config.form {
        apply_symbol(&mut theme.form.checkbox_off, &form.checkbox_off);
        apply_symbol(&mut theme.form.checkbox_on, &form.checkbox_on);
        apply_symbol(&mut theme.form.radio_off, &form.radio_off);
        apply_symbol(&mut theme.form.radio_on, &form.radio_on);
    }

    if let Some(graph) = &config.graph {
        apply_symbol(&mut theme.graph.commit_branch, &graph.commit_branch);
        apply_symbol(&mut theme.graph.commit, &graph.commit);
        apply_symbol(&mut theme.graph.commit_stash, &graph.commit_stash);
        apply_symbol(&mut theme.graph.empty, &graph.empty);
        apply_symbol(&mut theme.graph.horizontal, &graph.horizontal);
        apply_symbol(&mut theme.graph.horizontal_dotted, &graph.horizontal_dotted);
        apply_symbol(&mut theme.graph.vertical, &graph.vertical);
        apply_symbol(&mut theme.graph.vertical_dotted, &graph.vertical_dotted);
        apply_symbol(&mut theme.graph.merge_left_from, &graph.merge_left_from);
        apply_symbol(&mut theme.graph.merge_right_from, &graph.merge_right_from);
        apply_symbol(&mut theme.graph.branch_up, &graph.branch_up);
        apply_symbol(&mut theme.graph.branch_up_right, &graph.branch_up_right);
        apply_symbol(&mut theme.graph.branch_down, &graph.branch_down);
        apply_symbol(&mut theme.graph.merge, &graph.merge);
        apply_symbol(&mut theme.graph.uncommitted, &graph.uncommitted);
    }

    if let Some(heatmap) = &config.heatmap {
        apply_symbol(&mut theme.heatmap.zero, &heatmap.zero);
        apply_symbol(&mut theme.heatmap.one, &heatmap.one);
        apply_symbol(&mut theme.heatmap.two, &heatmap.two);
        apply_symbol(&mut theme.heatmap.three, &heatmap.three);
        apply_symbol(&mut theme.heatmap.four, &heatmap.four);
        apply_symbol(&mut theme.heatmap.five, &heatmap.five);
        apply_symbol(&mut theme.heatmap.six, &heatmap.six);
        apply_symbol(&mut theme.heatmap.seven, &heatmap.seven);
        apply_symbol(&mut theme.heatmap.many, &heatmap.many);
    }

    if let Some(modal) = &config.modal {
        apply_symbol(&mut theme.modal.selected, &modal.selected);
        apply_symbol(&mut theme.modal.unselected, &modal.unselected);
        apply_symbol(&mut theme.modal.mask, &modal.mask);
    }

    if let Some(scrollbar) = &config.scrollbar {
        apply_symbol(&mut theme.scrollbar.begin, &scrollbar.begin);
        apply_symbol(&mut theme.scrollbar.end, &scrollbar.end);
        apply_symbol(&mut theme.scrollbar.track, &scrollbar.track);
        apply_symbol(&mut theme.scrollbar.thumb, &scrollbar.thumb);
        apply_symbol(&mut theme.scrollbar.inactive_thumb, &scrollbar.inactive_thumb);
    }

    if let Some(settings) = &config.settings {
        apply_symbol(&mut theme.settings.compact_tab, &settings.compact_tab);
    }

    if let Some(splash) = &config.splash {
        apply_symbol(&mut theme.splash.logo_word_prefix, &splash.logo_word_prefix);
        apply_symbol(&mut theme.splash.logo_compact, &splash.logo_compact);
        apply_symbol_vec(&mut theme.splash.logo_narrow, &splash.logo_narrow);
        apply_symbol_vec(&mut theme.splash.logo_wide, &splash.logo_wide);
        apply_symbol(&mut theme.splash.selected_left, &splash.selected_left);
        apply_symbol(&mut theme.splash.selected_right, &splash.selected_right);
        apply_symbol(&mut theme.splash.logo_corner, &splash.logo_corner);
    }

    if let Some(status) = &config.status {
        apply_symbol(&mut theme.status.added, &status.added);
        apply_symbol(&mut theme.status.added_spaced, &status.added_spaced);
        apply_symbol(&mut theme.status.conflict, &status.conflict);
        apply_symbol(&mut theme.status.conflict_spaced, &status.conflict_spaced);
        apply_symbol(&mut theme.status.deleted, &status.deleted);
        apply_symbol(&mut theme.status.deleted_spaced, &status.deleted_spaced);
        apply_symbol(&mut theme.status.modified, &status.modified);
        apply_symbol(&mut theme.status.modified_spaced, &status.modified_spaced);
        apply_symbol(&mut theme.status.renamed, &status.renamed);
        apply_symbol(&mut theme.status.renamed_arrow_spaced, &status.renamed_arrow_spaced);
        apply_symbol(&mut theme.status.other, &status.other);
        apply_symbol(&mut theme.status.other_spaced, &status.other_spaced);
    }

    if let Some(submodule) = &config.submodule {
        apply_symbol(&mut theme.submodule.default, &submodule.default);
        apply_symbol(&mut theme.submodule.empty, &submodule.empty);
        apply_symbol(&mut theme.submodule.dirty, &submodule.dirty);
        apply_symbol(&mut theme.submodule.uninitialized, &submodule.uninitialized);
        apply_symbol(&mut theme.submodule.stack_separator, &submodule.stack_separator);
    }

    if let Some(worktree) = &config.worktree {
        apply_symbol(&mut theme.worktree.current, &worktree.current);
        apply_symbol(&mut theme.worktree.other, &worktree.other);
        apply_symbol(&mut theme.worktree.dirty, &worktree.dirty);
        apply_symbol(&mut theme.worktree.locked, &worktree.locked);
        apply_symbol(&mut theme.worktree.invalid, &worktree.invalid);
        apply_symbol(&mut theme.worktree.empty, &worktree.empty);
    }

    if let Some(weekday) = &config.weekday {
        apply_symbol(&mut theme.weekday.monday, &weekday.monday);
        apply_symbol(&mut theme.weekday.tuesday, &weekday.tuesday);
        apply_symbol(&mut theme.weekday.wednesday, &weekday.wednesday);
        apply_symbol(&mut theme.weekday.thursday, &weekday.thursday);
        apply_symbol(&mut theme.weekday.friday, &weekday.friday);
        apply_symbol(&mut theme.weekday.saturday, &weekday.saturday);
        apply_symbol(&mut theme.weekday.sunday, &weekday.sunday);
    }
}

fn symbol_config(theme: &SymbolTheme) -> SymbolConfig {
    SymbolConfig {
        branch: Some(BranchSymbolConfig {
            local_visible: Some(theme.branch.local_visible.clone()),
            local_hidden: Some(theme.branch.local_hidden.clone()),
            remote_visible: Some(theme.branch.remote_visible.clone()),
            remote_hidden: Some(theme.branch.remote_hidden.clone()),
        }),
        border: Some(BorderSymbolConfig {
            horizontal: Some(theme.border.horizontal.clone()),
            vertical: Some(theme.border.vertical.clone()),
            t_right: Some(theme.border.t_right.clone()),
            t_left: Some(theme.border.t_left.clone()),
            top_t: Some(theme.border.top_t.clone()),
            bottom_t: Some(theme.border.bottom_t.clone()),
            rounded_top_right: Some(theme.border.rounded_top_right.clone()),
            rounded_bottom_right: Some(theme.border.rounded_bottom_right.clone()),
            rounded_top_left: Some(theme.border.rounded_top_left.clone()),
            rounded_bottom_left: Some(theme.border.rounded_bottom_left.clone()),
        }),
        entity: Some(EntitySymbolConfig { folder: Some(theme.entity.folder.clone()), tag: Some(theme.entity.tag.clone()), reflog: Some(theme.entity.reflog.clone()) }),
        empty_state: Some(EmptyStateSymbolConfig { mark: Some(theme.empty_state.mark.clone()) }),
        form: Some(FormSymbolConfig {
            checkbox_off: Some(theme.form.checkbox_off.clone()),
            checkbox_on: Some(theme.form.checkbox_on.clone()),
            radio_off: Some(theme.form.radio_off.clone()),
            radio_on: Some(theme.form.radio_on.clone()),
        }),
        graph: Some(GraphSymbolConfig {
            commit_branch: Some(theme.graph.commit_branch.clone()),
            commit: Some(theme.graph.commit.clone()),
            commit_stash: Some(theme.graph.commit_stash.clone()),
            empty: Some(theme.graph.empty.clone()),
            horizontal: Some(theme.graph.horizontal.clone()),
            horizontal_dotted: Some(theme.graph.horizontal_dotted.clone()),
            vertical: Some(theme.graph.vertical.clone()),
            vertical_dotted: Some(theme.graph.vertical_dotted.clone()),
            merge_left_from: Some(theme.graph.merge_left_from.clone()),
            merge_right_from: Some(theme.graph.merge_right_from.clone()),
            branch_up: Some(theme.graph.branch_up.clone()),
            branch_up_right: Some(theme.graph.branch_up_right.clone()),
            branch_down: Some(theme.graph.branch_down.clone()),
            merge: Some(theme.graph.merge.clone()),
            uncommitted: Some(theme.graph.uncommitted.clone()),
        }),
        heatmap: Some(HeatmapSymbolConfig {
            zero: Some(theme.heatmap.zero.clone()),
            one: Some(theme.heatmap.one.clone()),
            two: Some(theme.heatmap.two.clone()),
            three: Some(theme.heatmap.three.clone()),
            four: Some(theme.heatmap.four.clone()),
            five: Some(theme.heatmap.five.clone()),
            six: Some(theme.heatmap.six.clone()),
            seven: Some(theme.heatmap.seven.clone()),
            many: Some(theme.heatmap.many.clone()),
        }),
        modal: Some(ModalSymbolConfig { selected: Some(theme.modal.selected.clone()), unselected: Some(theme.modal.unselected.clone()), mask: Some(theme.modal.mask.clone()) }),
        scrollbar: Some(ScrollbarSymbolConfig {
            begin: Some(theme.scrollbar.begin.clone()),
            end: Some(theme.scrollbar.end.clone()),
            track: Some(theme.scrollbar.track.clone()),
            thumb: Some(theme.scrollbar.thumb.clone()),
            inactive_thumb: Some(theme.scrollbar.inactive_thumb.clone()),
        }),
        settings: Some(SettingsSymbolConfig { compact_tab: Some(theme.settings.compact_tab.clone()) }),
        splash: Some(SplashSymbolConfig {
            logo_word_prefix: Some(theme.splash.logo_word_prefix.clone()),
            logo_compact: Some(theme.splash.logo_compact.clone()),
            logo_narrow: Some(theme.splash.logo_narrow.clone()),
            logo_wide: Some(theme.splash.logo_wide.clone()),
            selected_left: Some(theme.splash.selected_left.clone()),
            selected_right: Some(theme.splash.selected_right.clone()),
            logo_corner: Some(theme.splash.logo_corner.clone()),
        }),
        status: Some(StatusSymbolConfig {
            added: Some(theme.status.added.clone()),
            added_spaced: Some(theme.status.added_spaced.clone()),
            conflict: Some(theme.status.conflict.clone()),
            conflict_spaced: Some(theme.status.conflict_spaced.clone()),
            deleted: Some(theme.status.deleted.clone()),
            deleted_spaced: Some(theme.status.deleted_spaced.clone()),
            modified: Some(theme.status.modified.clone()),
            modified_spaced: Some(theme.status.modified_spaced.clone()),
            renamed: Some(theme.status.renamed.clone()),
            renamed_arrow_spaced: Some(theme.status.renamed_arrow_spaced.clone()),
            other: Some(theme.status.other.clone()),
            other_spaced: Some(theme.status.other_spaced.clone()),
        }),
        submodule: Some(SubmoduleSymbolConfig {
            default: Some(theme.submodule.default.clone()),
            empty: Some(theme.submodule.empty.clone()),
            dirty: Some(theme.submodule.dirty.clone()),
            uninitialized: Some(theme.submodule.uninitialized.clone()),
            stack_separator: Some(theme.submodule.stack_separator.clone()),
        }),
        worktree: Some(WorktreeSymbolConfig {
            current: Some(theme.worktree.current.clone()),
            other: Some(theme.worktree.other.clone()),
            dirty: Some(theme.worktree.dirty.clone()),
            locked: Some(theme.worktree.locked.clone()),
            invalid: Some(theme.worktree.invalid.clone()),
            empty: Some(theme.worktree.empty.clone()),
        }),
        weekday: Some(WeekdaySymbolConfig {
            monday: Some(theme.weekday.monday.clone()),
            tuesday: Some(theme.weekday.tuesday.clone()),
            wednesday: Some(theme.weekday.wednesday.clone()),
            thursday: Some(theme.weekday.thursday.clone()),
            friday: Some(theme.weekday.friday.clone()),
            saturday: Some(theme.weekday.saturday.clone()),
            sunday: Some(theme.weekday.sunday.clone()),
        }),
    }
}

fn symbol_theme_config(theme: &SymbolTheme) -> SymbolThemeConfig {
    SymbolThemeConfig { label: theme.label().to_string(), symbols: symbol_config(theme) }
}

fn symbol_theme_from_config(config: SymbolThemeConfig) -> Option<SymbolTheme> {
    let label = config.label.trim();
    if label.is_empty() {
        return None;
    }

    let preset = SymbolTheme::from_label(label);
    let original = preset.clone().unwrap_or_else(SymbolTheme::main);
    let mut theme = original.clone();
    apply_symbols(&mut theme, &config.symbols);

    if preset.is_none() || !theme.symbols_equal(&original) {
        theme = SymbolTheme::custom(label, theme);
    }

    Some(theme)
}

fn symbol_theme_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap();
    path.push("guitar");
    path.push("symbols.json");
    path
}

pub fn load_symbol_theme_from_path(path: &Path) -> SymbolTheme {
    if path.exists() {
        let contents = fs::read_to_string(path).unwrap_or_default();
        if let Ok(config) = facet_json::from_str::<SymbolThemeConfig>(&contents)
            && let Some(theme) = symbol_theme_from_config(config)
        {
            save_symbol_theme_to_path(path, &theme);
            return theme;
        }
        if let Ok(label) = facet_json::from_str::<String>(&contents)
            && let Some(theme) = SymbolTheme::from_label(&label)
        {
            save_symbol_theme_to_path(path, &theme);
            return theme;
        }
    }

    let theme = SymbolTheme::default();
    save_symbol_theme_to_path(path, &theme);
    theme
}

pub fn save_symbol_theme_to_path(path: &Path, theme: &SymbolTheme) {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        let _ = fs::create_dir_all(parent);
    }

    let config = symbol_theme_config(theme);
    let symbol_string = facet_json::to_string_pretty(&config).unwrap();
    fs::write(path, symbol_string).unwrap();
}

pub fn load_symbol_theme() -> SymbolTheme {
    load_symbol_theme_from_path(&symbol_theme_path())
}

pub fn save_symbol_theme(theme: &SymbolTheme) {
    save_symbol_theme_to_path(&symbol_theme_path(), theme);
}

pub mod branch {
    pub const LOCAL_VISIBLE: &str = "●";
    pub const LOCAL_HIDDEN: &str = "○";
    pub const REMOTE_VISIBLE: &str = "◆";
    pub const REMOTE_HIDDEN: &str = "◇";
}

pub mod border {
    pub const HORIZONTAL: &str = "─";
    pub const VERTICAL: &str = "│";
    pub const T_RIGHT: &str = "┤";
    pub const T_LEFT: &str = "├";
    pub const TOP_T: &str = "┬";
    pub const BOTTOM_T: &str = "┴";
    pub const ROUNDED_TOP_RIGHT: &str = "╮";
    pub const ROUNDED_BOTTOM_RIGHT: &str = "╯";
    pub const ROUNDED_TOP_LEFT: &str = "╭";
    pub const ROUNDED_BOTTOM_LEFT: &str = "╰";
}

pub mod entity {
    pub const FOLDER: &str = "";
    pub const TAG: &str = "\u{F04F9}";
    pub const REFLOG: &str = "\u{F0E2}";
}

pub mod empty_state {
    pub const MARK: &str = "⊘";
}

pub mod form {
    pub const CHECKBOX_OFF: &str = "🞎";
    pub const CHECKBOX_ON: &str = "🞕";
    pub const RADIO_OFF: &str = "🞅";
    pub const RADIO_ON: &str = "🞊";
}

pub mod graph {
    pub const COMMIT_BRANCH: &str = super::branch::LOCAL_VISIBLE;
    pub const COMMIT: &str = super::branch::LOCAL_HIDDEN;
    pub const COMMIT_STASH: &str = "◎";
    pub const EMPTY: &str = " ";
    pub const HORIZONTAL: &str = super::border::HORIZONTAL;
    pub const HORIZONTAL_DOTTED: &str = "┄";
    pub const VERTICAL: &str = super::border::VERTICAL;
    pub const VERTICAL_DOTTED: &str = "┊";
    pub const MERGE_LEFT_FROM: &str = super::border::T_RIGHT;
    pub const MERGE_RIGHT_FROM: &str = super::border::ROUNDED_TOP_LEFT;
    pub const BRANCH_UP: &str = super::border::ROUNDED_BOTTOM_RIGHT;
    pub const BRANCH_UP_RIGHT: &str = super::border::ROUNDED_BOTTOM_LEFT;
    pub const BRANCH_DOWN: &str = super::border::ROUNDED_TOP_RIGHT;
    pub const MERGE: &str = "•";
    pub const UNCOMMITTED: &str = "◌";
}

pub mod modal {
    pub const SELECTED: &str = ">";
    pub const UNSELECTED: &str = " ";
    pub const MASK: &str = "*";
}

pub mod scrollbar {
    pub const BEGIN: &str = super::border::ROUNDED_TOP_RIGHT;
    pub const END: &str = super::border::ROUNDED_BOTTOM_RIGHT;
    pub const TRACK: &str = super::border::VERTICAL;
    pub const THUMB: &str = "▌";
    pub const INACTIVE_THUMB: &str = super::border::VERTICAL;
}

pub mod settings {
    pub const COMPACT_TAB: &str = "•";
}

pub mod splash {
    pub const LOGO_WORD_PREFIX: &str = "  guita";
    pub const LOGO_COMPACT: &str = "guita╭";
    pub const LOGO_NARROW: [&str; 9] = [
        "                    :#   :#                 ",
        "                         L#                 ",
        "  .##5#^.  .#   .#  :C  #C6#   #?##:        ",
        "  #B   #G  C#   #B  #7   B?        G#       ",
        "  #4   B5  B5   B5  B5   B5    1B5B#G  .a###",
        "  #b   5?  ?B   B5  B5   B5   ##   ##  B?   ",
        "  .#B~6G!  .#6#~G.  #5   ~##  .##Y~#.  !#   ",
        "      .##                              !B   ",
        "     ~G#                               ~?   ",
    ];
    pub const LOGO_WIDE: [&str; 11] = [
        "                                 :GG~        .?Y.                                ",
        "       ....        ..      ..   .....      . ^BG: ..       .....                 ",
        "    .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              ",
        "   .5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^",
        "   7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY",
        "   ?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. ",
        "   ^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     ",
        "    ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      ",
        "      .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      ",
        "    .?!^^^!5G7                                                          YB5      ",
        "    .!?JJJ?!:                                                           75?      ",
    ];
    pub const SELECTED_LEFT: &str = "⏵ ";
    pub const SELECTED_RIGHT: &str = " ⏴";
    pub const LOGO_CORNER: &str = super::border::ROUNDED_TOP_LEFT;
}

pub mod status {
    pub const ADDED: &str = "+";
    pub const ADDED_SPACED: &str = "+ ";
    pub const CONFLICT: &str = "!";
    pub const CONFLICT_SPACED: &str = "! ";
    pub const DELETED: &str = "-";
    pub const DELETED_SPACED: &str = "- ";
    pub const MODIFIED: &str = "~";
    pub const MODIFIED_SPACED: &str = "~ ";
    pub const RENAMED: &str = ">";
    pub const RENAMED_ARROW_SPACED: &str = "→ ";
    pub const OTHER: &str = "*";
    pub const OTHER_SPACED: &str = "  ";
}

pub mod submodule {
    pub const DEFAULT: &str = "\u{F03D6}";
    pub const EMPTY: &str = "\u{F19D9}";
    pub const DIRTY: &str = "\u{F19D8}";
    pub const UNINITIALIZED: &str = "\u{f487}";
    pub const STACK_SEPARATOR: &str = " › ";
}

pub mod worktree {
    pub const CURRENT: &str = "\u{F0770}";
    pub const OTHER: &str = "\u{F0256}";
    pub const DIRTY: &str = "\u{F0DCE}";
    pub const LOCKED: &str = "\u{F1AA8}";
    pub const INVALID: &str = "\u{F19F9}";
    pub const EMPTY: &str = "\u{F179E}";
}

pub const WEEKDAY_LABELS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];

#[cfg(test)]
#[path = "../tests/helpers/symbols.rs"]
mod tests;
