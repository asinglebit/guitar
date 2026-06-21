use crate::git::queries::remotes::effective_default_remote_from_remotes;
use crate::helpers::heatmap::heat_cell;
use crate::helpers::keymap::{Command, InputMode, KeymapSelection, action_keymap_visible_entries, keybinding_to_visual_string};
use crate::helpers::layout::scrollbar_content_length;
use crate::helpers::localisation::{Language, common, empty, settings as settings_text};
use crate::helpers::palette::*;
use crate::helpers::symbols::{SymbolTheme, SymbolThemeName};
use crate::helpers::version::VERSION;
use crate::{
    app::app::{App, Direction, Focus, SettingsSelection, SettingsSelectionKind, SettingsTab, SettingsTabHitbox},
    core::renderers::render_keybindings,
    git::queries::commits::get_git_user_info,
    helpers::text::fill_width,
};
use ratatui::Frame;
use ratatui::widgets::Borders;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

const SETTINGS_PANE_COMMANDS: &[(&str, Command, fn() -> &'static str)] = &[
    ("1", Command::ToggleBranches, settings_text::BRANCHES),
    ("2", Command::ToggleTags, settings_text::TAGS),
    ("3", Command::ToggleStashes, settings_text::STASHES),
    ("4", Command::ToggleReflogs, settings_text::REFLOG),
    ("5", Command::ToggleWorktrees, settings_text::WORKTREES),
    ("6", Command::ToggleSubmodules, settings_text::SUBMODULES),
    ("7", Command::ToggleSearch, settings_text::SEARCH),
    ("8", Command::ToggleInspector, settings_text::INSPECTOR),
    ("9", Command::ToggleStatus, settings_text::STATUS),
    ("0", Command::ResetLayout, settings_text::RESET_LAYOUT),
];

const SETTINGS_GRAPH_COMMANDS: &[(&str, Command, fn() -> &'static str)] = &[
    (")", Command::ToggleGraphReflogs, settings_text::GRAPH_REFLOG_COMMITS),
    ("!", Command::ToggleShas, settings_text::SHAS),
    ("@", Command::ToggleGraphDates, settings_text::COMMITTER_DATE_TIME),
    ("#", Command::ToggleGraphCommitters, settings_text::COMMITTERS),
    ("$", Command::ToggleGraphRefs, settings_text::REFS),
];

impl App {
    fn settings_section_line(&self, label: &str, width: usize) -> Line<'static> {
        Line::from(Span::styled(fill_width(label, "", width), Style::default().fg(self.theme.COLOR_HIGHLIGHTED))).centered()
    }

    fn settings_layout_command_key(&self, command: &Command, fallback: &str) -> String {
        self.keymaps
            .get(&InputMode::Normal)
            .and_then(|mode_keymap| mode_keymap.iter().find(|(_, current)| *current == command).map(|(key, _)| keybinding_to_visual_string(key)))
            .unwrap_or_else(|| fallback.to_string())
    }

    fn settings_layout_command_state(&self, command: &Command) -> String {
        match command {
            Command::ToggleBranches => {
                if self.layout_config.is_branches {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleTags => {
                if self.layout_config.is_tags {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleStashes => {
                if self.layout_config.is_stashes {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleStatus => {
                if self.layout_config.is_status {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleInspector => {
                if self.layout_config.is_inspector {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleWorktrees => {
                if self.layout_config.is_worktrees {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleSubmodules => {
                if self.layout_config.is_submodules {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleReflogs => {
                if self.layout_config.is_reflogs {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleSearch => {
                if self.layout_config.is_search {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleShas => {
                if self.layout_config.is_shas {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleGraphReflogs => {
                if self.layout_config.is_graph_reflogs {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleGraphDates => {
                if self.layout_config.is_graph_dates {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleGraphCommitters => {
                if self.layout_config.is_graph_committers {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ToggleGraphRefs => {
                if self.layout_config.is_graph_refs {
                    self.symbols.form.checkbox_on.clone()
                } else {
                    self.symbols.form.checkbox_off.clone()
                }
            },
            Command::ResetLayout => settings_text::ENTER_ACTION().to_string(),
            _ => String::new(),
        }
    }

    fn settings_filled_line(&self, left: &str, right: &str, width: usize, style: Style) -> Line<'static> {
        Line::from(Span::styled(fill_width(left, right, width), style)).centered()
    }

    fn settings_text_area(&self) -> (u16, u16) {
        let border = u16::from(self.layout_config.is_zen);
        let x = self.layout.graph.x.saturating_add(border).saturating_add(1);
        let width = self.layout.graph.width.saturating_sub(border.saturating_mul(2)).saturating_sub(2);
        (x, width)
    }

    fn settings_centered_text_start(&self, text_width: usize) -> u16 {
        let (x, width) = self.settings_text_area();
        x.saturating_add(width.saturating_sub(text_width as u16) / 2)
    }

    fn settings_tab_bar_from_labels(&mut self, width: usize, line: usize, tab_gap: &'static str, labels: &[(SettingsTab, String)]) -> Line<'static> {
        let base_width = labels.iter().map(|(_, label)| label.chars().count()).sum::<usize>().saturating_add(tab_gap.chars().count().saturating_mul(labels.len().saturating_sub(1)));
        let pad = width.saturating_sub(base_width);
        let left_pad = pad / 2;
        let right_pad = pad.saturating_sub(left_pad);
        let text_width = base_width.saturating_add(pad);
        let row_start = self.settings_centered_text_start(text_width);
        let mut offset = left_pad;
        let mut spans = Vec::new();

        if left_pad > 0 {
            spans.push(Span::raw(" ".repeat(left_pad)));
        }

        for (idx, (tab, label)) in labels.iter().enumerate() {
            if idx > 0 {
                spans.push(Span::raw(tab_gap));
                offset = offset.saturating_add(tab_gap.chars().count());
            }

            let label_width = label.chars().count();
            let start = row_start.saturating_add(offset as u16);
            let end = start.saturating_add(label_width as u16);
            self.settings_tab_hitboxes.push(SettingsTabHitbox { tab: *tab, line, start, end });

            let style = if *tab == self.settings_tab {
                Style::default().fg(self.theme.COLOR_HIGHLIGHTED).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900))
            } else {
                Style::default().fg(self.theme.COLOR_TEXT)
            };
            spans.push(Span::styled(label.clone(), style));
            offset = offset.saturating_add(label_width);
        }

        if right_pad > 0 {
            spans.push(Span::raw(" ".repeat(right_pad)));
        }

        Line::from(spans).centered()
    }

    fn settings_tab_bar_line(&mut self, width: usize, line: usize) -> Line<'static> {
        let full_tab_gap = "  ";
        let full_labels: Vec<(SettingsTab, String)> = SettingsTab::ALL.iter().map(|&tab| (tab, format!(" {} ", tab.label()))).collect();
        let full_width = full_labels.iter().map(|(_, label)| label.chars().count()).sum::<usize>().saturating_add(full_tab_gap.chars().count().saturating_mul(full_labels.len().saturating_sub(1)));

        if full_width <= width {
            return self.settings_tab_bar_from_labels(width, line, full_tab_gap, &full_labels);
        }

        let compact_tab_gap = if width >= SettingsTab::ALL.len().saturating_mul(2).saturating_sub(1) { " " } else { "" };
        let visible_tabs = if compact_tab_gap.is_empty() { width.min(SettingsTab::ALL.len()) } else { SettingsTab::ALL.len() };
        let compact_labels: Vec<(SettingsTab, String)> = SettingsTab::ALL.iter().take(visible_tabs).map(|&tab| (tab, self.symbols.settings.compact_tab.clone())).collect();

        self.settings_tab_bar_from_labels(width, line, compact_tab_gap, &compact_labels)
    }

    fn add_settings_selection(&mut self, lines: &[Line<'static>], kind: SettingsSelectionKind) {
        self.settings_selections.push(SettingsSelection { line: lines.len().saturating_sub(1), kind });
    }

    fn append_settings_performance(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::PERFORMANCE(), width));
        lines.push(Line::default());

        lines.push(self.settings_filled_line(
            settings_text::GRAPH_LANE_LIMIT(),
            format!(" {} {} ", self.layout_config.graph_lane_limit, settings_text::ENTER_ACTION()).as_str(),
            width,
            Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
        ));
        self.add_settings_selection(lines, SettingsSelectionKind::GraphLaneLimit);
    }

    fn append_settings_general(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        // Config paths are informational, but still selectable for consistent navigation.
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::PATHS_SECTION(), width));
        lines.push(Line::default());
        let mut pathbuf = dirs::config_dir().unwrap();
        pathbuf.push("guitar");
        let path = pathbuf.as_path().to_str().unwrap();

        let shaded = Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
        let plain = Style::default().fg(self.theme.COLOR_TEXT);

        lines.push(self.settings_filled_line(settings_text::KEYMAP(), format!(" {}/keymap.json ", path).as_str(), width, shaded));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::LANGUAGE(), format!(" {}/language.json ", path).as_str(), width, plain));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::LAYOUT(), format!(" {}/layout.json ", path).as_str(), width, shaded));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::THEME(), format!(" {}/theme.json ", path).as_str(), width, plain));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::SYMBOLS(), format!(" {}/symbols.json ", path).as_str(), width, shaded));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::RECENT_FILE(), format!(" {}/recent.json ", path).as_str(), width, plain));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);

        self.append_settings_performance(lines, width);

        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::RECENT_REPOSITORIES(), width));
        lines.push(Line::default());
        lines.push(self.settings_filled_line(settings_text::ACTIONS(), format!("{} ", self.recent_repository_actions_detail_text()).as_str(), width, plain));
        lines.push(Line::default());

        if self.recent.is_empty() {
            lines.push(self.settings_filled_line(&format!(" {}", empty::NO_RECENT_REPOSITORIES()), "", width, plain));
        } else {
            let recent = self.recent.clone();
            for (idx, path) in recent.iter().enumerate() {
                let mut style = Style::default().fg(if Some(path) == self.path.as_ref() { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT });
                if idx.is_multiple_of(2) {
                    style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
                }
                lines.push(self.settings_filled_line(format!(" {}", path).as_str(), "", width, style));
                self.add_settings_selection(lines, SettingsSelectionKind::RecentRepository(idx));
            }
        }
    }

    fn append_settings_repo(&mut self, lines: &mut Vec<Line<'static>>, repo: &git2::Repository, width: usize) {
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::REMOTES(), width));
        lines.push(Line::default());
        lines.push(self.settings_filled_line(settings_text::ACTIONS(), settings_text::REMOTES_ACTIONS_DETAIL(), width, Style::default().fg(self.theme.COLOR_TEXT)));
        lines.push(Line::default());

        let remotes = self.remotes.clone();
        match remotes.as_slice() {
            [] => {
                lines.push(self.settings_filled_line(settings_text::DEFAULT_REMOTE(), format!(" {} ", common::NONE()).as_str(), width, Style::default().fg(self.theme.COLOR_TEXT)));
                lines.push(Line::default());
                lines.push(self.settings_filled_line(
                    settings_text::ADD_REMOTE(),
                    format!("{} ", settings_text::ENTER_ACTION()).as_str(),
                    width,
                    Style::default().fg(self.theme.COLOR_GRASS).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
                ));
                self.add_settings_selection(lines, SettingsSelectionKind::RemoteAdd);
                lines.push(self.settings_filled_line(&format!(" {}", empty::NO_REMOTES()), "", width, Style::default().fg(self.theme.COLOR_TEXT)));
            },
            remotes => {
                let default_remote = effective_default_remote_from_remotes(repo, remotes);
                let default_label = default_remote.as_deref().unwrap_or(common::NONE());
                lines.push(self.settings_filled_line(settings_text::DEFAULT_REMOTE(), format!(" {default_label} ").as_str(), width, Style::default().fg(self.theme.COLOR_GRASS)));
                lines.push(Line::default());
                lines.push(self.settings_filled_line(
                    settings_text::ADD_REMOTE(),
                    format!("{} ", settings_text::ENTER_ACTION()).as_str(),
                    width,
                    Style::default().fg(self.theme.COLOR_GRASS).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
                ));
                self.add_settings_selection(lines, SettingsSelectionKind::RemoteAdd);

                for (idx, remote) in remotes.iter().enumerate() {
                    let effective_push_url = remote.push_url.as_deref().filter(|url| !url.is_empty()).unwrap_or(remote.url.as_str());
                    let is_default = default_remote.as_deref() == Some(remote.name.as_str());
                    let marker = if is_default { format!(" {}", common::DEFAULT_REMOTE()) } else { String::new() };

                    let mut style = Style::default().fg(if is_default { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT });
                    if (idx + 1).is_multiple_of(2) {
                        style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
                    }

                    lines.push(self.settings_filled_line(format!(" {} {}", remote.name, settings_text::FETCH_SUFFIX()).as_str(), format!(" {}{marker} ", remote.url).as_str(), width, style));
                    self.add_settings_selection(lines, SettingsSelectionKind::Remote(remote.name.clone()));

                    if !effective_push_url.is_empty() {
                        lines.push(self.settings_filled_line(format!(" {} {}", remote.name, settings_text::PUSH_SUFFIX()).as_str(), format!(" {effective_push_url}{marker} ").as_str(), width, style));
                        self.add_settings_selection(lines, SettingsSelectionKind::Remote(remote.name.clone()));
                    }
                }
            },
        }
    }

    fn append_settings_auth(&mut self, lines: &mut Vec<Line<'static>>, repo: &git2::Repository, width: usize) {
        // Credentials are read live so the settings view reflects git config changes.
        let (name, email) = get_git_user_info(repo).unwrap();
        let shaded = Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
        let plain = Style::default().fg(self.theme.COLOR_TEXT);

        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::CREDENTIALS(), width));
        lines.push(Line::default());

        lines.push(self.settings_filled_line(settings_text::NAME(), format!("{} ", name.unwrap()).as_str(), width, shaded));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::EMAIL(), format!("{} ", email.unwrap()).as_str(), width, plain));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::AUTHORIZATION(), settings_text::SSH_AGENT_DETAIL(), width, shaded));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::SSH_FALLBACK(), settings_text::SSH_FALLBACK_DETAIL(), width, plain));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::HTTPS(), settings_text::HTTPS_DETAIL(), width, shaded));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
        lines.push(self.settings_filled_line(settings_text::SECRETS(), settings_text::SECRETS_DETAIL(), width, plain));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);
    }

    fn append_settings_themes(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::THEMES(), width));
        lines.push(Line::default());

        if self.theme.name == ThemeNames::Custom {
            lines.push(self.settings_filled_line(
                settings_text::ACTIVE_CUSTOM(),
                format!(" {} ", self.theme.label()).as_str(),
                width,
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ));
            lines.push(Line::default());
        }

        for (idx, preset) in Theme::presets().iter().enumerate() {
            let label = format!(" {}", preset.label);
            let marker = format!("{} ", if self.theme.name == preset.theme.name { &self.symbols.form.radio_on } else { &self.symbols.form.radio_off });
            let mut style = Style::default().fg(self.theme.COLOR_TEXT);
            if idx.is_multiple_of(2) {
                style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
            }
            lines.push(self.settings_filled_line(&label, &marker, width, style));
            self.add_settings_selection(lines, SettingsSelectionKind::Theme(idx));
        }
    }

    fn append_settings_languages(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::LANGUAGE(), width));
        lines.push(Line::default());

        for (idx, language) in Language::ALL.iter().enumerate() {
            let label = format!(" {}", language.native_label());
            let marker = format!("{} ", if self.language == *language { &self.symbols.form.radio_on } else { &self.symbols.form.radio_off });
            let mut style = Style::default().fg(self.theme.COLOR_TEXT);
            if idx.is_multiple_of(2) {
                style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
            }
            lines.push(self.settings_filled_line(&label, &marker, width, style));
            self.add_settings_selection(lines, SettingsSelectionKind::Language(idx));
        }
    }

    fn append_settings_symbol_themes(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::SYMBOL_THEMES(), width));
        lines.push(Line::default());

        if self.symbols.name == SymbolThemeName::Custom {
            lines.push(self.settings_filled_line(
                settings_text::ACTIVE_CUSTOM_SYMBOLS(),
                format!(" {} ", self.symbols.label()).as_str(),
                width,
                Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
            ));
            lines.push(Line::default());
        }

        for (idx, preset) in SymbolTheme::presets().iter().enumerate() {
            let preset_theme = preset.theme();
            let label = format!(" {}", preset.label);
            let marker = format!("{} ", if self.symbols.name == preset_theme.name { &self.symbols.form.radio_on } else { &self.symbols.form.radio_off });
            let mut style = Style::default().fg(self.theme.COLOR_TEXT);
            if idx.is_multiple_of(2) {
                style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
            }
            lines.push(self.settings_filled_line(&label, &marker, width, style));
            self.add_settings_selection(lines, SettingsSelectionKind::SymbolTheme(idx));
        }
    }

    fn append_settings_layout(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::PANE_VISIBILITY(), width));
        lines.push(Line::default());
        for (idx, (fallback, command, label)) in SETTINGS_PANE_COMMANDS.iter().enumerate() {
            let key = self.settings_layout_command_key(command, fallback);
            let label = format!(" {} {}:", key, label());
            let state = format!(" {} ", self.settings_layout_command_state(command));
            let mut style = Style::default().fg(self.theme.COLOR_TEXT);
            if idx.is_multiple_of(2) {
                style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
            }
            lines.push(self.settings_filled_line(&label, &state, width, style));
            self.add_settings_selection(lines, SettingsSelectionKind::LayoutCommand(command.clone()));
        }

        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::GRAPH_METADATA(), width));
        lines.push(Line::default());
        for (idx, (fallback, command, label)) in SETTINGS_GRAPH_COMMANDS.iter().enumerate() {
            let key = self.settings_layout_command_key(command, fallback);
            let label = format!(" {} {}:", key, label());
            let state = format!(" {} ", self.settings_layout_command_state(command));
            let mut style = Style::default().fg(self.theme.COLOR_TEXT);
            if idx.is_multiple_of(2) {
                style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
            }
            lines.push(self.settings_filled_line(&label, &state, width, style));
            self.add_settings_selection(lines, SettingsSelectionKind::LayoutCommand(command.clone()));
        }
    }

    fn append_settings_shortcuts(&mut self, lines: &mut Vec<Line<'static>>, width: usize) {
        // Keymap sections are generated from the active keymap data, not duplicated text.
        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::SHORTCUTS_NORMAL_MODE(), width));
        lines.push(Line::default());
        if let Some(mode_keymap) = self.keymaps.get(&InputMode::Normal).cloned() {
            let rendered = render_keybindings(&self.theme, &mode_keymap, width);
            for (idx, ((kb, cmd), kb_line)) in mode_keymap.iter().zip(rendered).enumerate() {
                let spans: Vec<Span> = kb_line
                    .spans
                    .iter()
                    .map(|span| {
                        let mut style = span.style;
                        if idx % 2 == 0 {
                            style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
                        }
                        Span::styled(span.content.clone(), style)
                    })
                    .collect();
                lines.push(Line::from(spans).centered());
                self.add_settings_selection(lines, SettingsSelectionKind::KeyBinding(KeymapSelection::new(InputMode::Normal, kb.clone(), cmd.clone())));
            }
        }

        lines.push(Line::default());
        lines.push(self.settings_section_line(settings_text::SHORTCUTS_ACTION_MODE(), width));
        lines.push(Line::default());
        if let Some(action_keymap) = self.keymaps.get(&InputMode::Action).cloned() {
            let normal_keymap = self.keymaps.get(&InputMode::Normal).cloned();
            let unique_action = action_keymap_visible_entries(normal_keymap.as_ref(), &action_keymap);
            let rendered = render_keybindings(&self.theme, &unique_action, width);
            for (idx, ((kb, cmd), kb_line)) in unique_action.iter().zip(rendered).enumerate() {
                let spans: Vec<Span> = kb_line
                    .spans
                    .iter()
                    .map(|span| {
                        let mut style = span.style;
                        if idx % 2 == 0 {
                            style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_900));
                        }
                        Span::styled(span.content.clone(), style)
                    })
                    .collect();

                lines.push(Line::from(spans).centered());
                self.add_settings_selection(lines, SettingsSelectionKind::KeyBinding(KeymapSelection::new(InputMode::Action, kb.clone(), cmd.clone())));
            }
        }
    }

    fn append_settings_header(&mut self, lines: &mut Vec<Line<'static>>, width: usize, week_start: usize) {
        lines.push(Line::default());
        lines.push(self.settings_filled_line(
            settings_text::VERSION(),
            format!("{} ", VERSION).as_str(),
            width,
            Style::default().fg(self.theme.COLOR_TEXT).bg(self.theme.background_or_default(self.theme.COLOR_GREY_900)),
        ));
        self.add_settings_selection(lines, SettingsSelectionKind::Info);

        // Heatmap rows use weekday labels followed by the cropped commit grid.
        lines.push(Line::default());
        for (day_idx, label) in self.symbols.weekday.labels().iter().enumerate() {
            let mut spans = Vec::new();
            spans.push(Span::styled(format!(" {}  ", label), Style::default().fg(self.theme.COLOR_TEXT)));
            spans.extend(self.heatmap[day_idx][week_start..].iter().map(|&count| {
                let span = heat_cell(count, &self.theme, &self.symbols);
                Span::styled(span.content.to_string(), span.style)
            }));
            lines.push(Line::from(spans).centered());
        }

        lines.push(Line::default());
        let tab_line = lines.len();
        lines.push(self.settings_tab_bar_line(width, tab_line));
        lines.push(Line::default());
    }

    pub(crate) fn settings_lines(&mut self, repo: &git2::Repository) -> Vec<Line<'static>> {
        let available_width = self.layout.graph.width.saturating_sub(1) as usize;

        // settings_selections maps selectable line indices to their settings action.
        let mut lines: Vec<Line<'static>> = Vec::new();
        self.settings_selections = Vec::new();
        self.settings_tab_hitboxes = Vec::new();

        // Each heat cell renders as two terminal columns.
        let cell_width = 2;

        // Outer margins are reserved so the centered content breathes in narrow terminals.
        let border_width = 8;

        let weekday_label_width = 2;
        let usable_width = available_width.saturating_sub(border_width).saturating_sub(weekday_label_width);

        // Only the most recent weeks that fit are rendered.
        let max_weeks_fit = (usable_width / cell_width).max(1);
        let total_weeks = self.heatmap[0].len();
        let visible_weeks = max_weeks_fit.min(total_weeks);

        let week_start = (total_weeks.saturating_sub(visible_weeks).saturating_add(2)).min(total_weeks);

        // All settings rows align to the heatmap body width.
        let heatmap_width = visible_weeks * cell_width;

        self.append_settings_header(&mut lines, heatmap_width, week_start);

        match self.settings_tab {
            SettingsTab::General => self.append_settings_general(&mut lines, heatmap_width),
            SettingsTab::Display => {
                self.append_settings_languages(&mut lines, heatmap_width);
                self.append_settings_layout(&mut lines, heatmap_width);
                self.append_settings_symbol_themes(&mut lines, heatmap_width);
                self.append_settings_themes(&mut lines, heatmap_width);
            },
            SettingsTab::Auth => self.append_settings_auth(&mut lines, repo, heatmap_width),
            SettingsTab::Repo => self.append_settings_repo(&mut lines, repo, heatmap_width),
            SettingsTab::Shortcuts => self.append_settings_shortcuts(&mut lines, heatmap_width),
        }

        lines
    }

    pub(crate) fn switch_settings_tab(&mut self, tab: SettingsTab) {
        if self.settings_tab == tab {
            return;
        }

        self.settings_tab = tab;
        self.settings_scroll.set(0);
        self.last_input_direction = None;
        self.settings_selected = 0;

        if let Some(repo) = self.repo.clone() {
            let _ = self.settings_lines(&repo);
            let content_start = self.settings_tab_hitboxes.first().map(|hitbox| hitbox.line.saturating_add(1)).unwrap_or(0);
            let selection = self.settings_selections.iter().find(|selection| selection.line >= content_start).or_else(|| self.settings_selections.first());
            if let Some(first) = selection {
                self.settings_selected = first.line;
            }
        }
    }

    pub fn draw_settings(&mut self, frame: &mut Frame, repo: &git2::Repository) {
        // Settings owns the center viewport and uses centered rows throughout.
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        let lines = self.settings_lines(repo);

        // Settings follows the same bounded scrolling behavior as graph-like lists.
        let total_lines = lines.len();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(2) as usize } else { self.layout.graph.height as usize };

        // Navigation lands on selectable rows even when the rendered list contains headings.
        if !self.settings_selections.iter().any(|selection| selection.line == self.settings_selected) {
            let mut nearest = None;

            if self.last_input_direction == Some(Direction::Down) {
                nearest = self.settings_selections.iter().map(|selection| selection.line).find(|&i| i > self.settings_selected);
            }

            if nearest.is_none() && self.last_input_direction == Some(Direction::Up) {
                nearest = self.settings_selections.iter().rev().map(|selection| selection.line).find(|&i| i < self.settings_selected);
            }

            // Without direction, choose the closest selectable row by distance.
            if nearest.is_none() {
                nearest = self.settings_selections.iter().map(|selection| selection.line).min_by_key(|&i| i.abs_diff(self.settings_selected));
            }

            if let Some(target) = nearest {
                self.settings_selected = target;
            }
        }

        self.trap_selection(self.settings_selected, &self.settings_scroll, total_lines, visible_height);

        let start = self.settings_scroll.get().min(total_lines.saturating_sub(visible_height));
        let end = (start + visible_height).min(total_lines);

        // Ensure blank lines still occupy space after conversion to ListItem.
        let list_items: Vec<ListItem> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let absolute_idx = start + i;
                let mut item = line.clone();

                if item.spans.is_empty() {
                    item.spans.push(Span::raw(" "));
                }

                // Highlight only while settings has viewport focus.
                if absolute_idx == self.settings_selected && self.focus == Focus::Viewport {
                    let spans: Vec<Span> = item
                        .spans
                        .iter()
                        .map(|span| {
                            let mut style = span.style;
                            style = style.bg(self.theme.background_or_default(self.theme.COLOR_GREY_800));
                            Span::styled(span.content.clone(), style)
                        })
                        .collect();
                    item = Line::from(spans).centered();
                }

                ListItem::from(item)
            })
            .collect();

        if self.layout_config.is_zen {
            // Zen mode frames settings as a standalone list.
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).border_set(self.symbols.border.block_set()).padding(padding));

            frame.render_widget(list, self.layout.graph);

            let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(start);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
                .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
                .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
                .thumb_symbol(self.symbols.scrollbar.thumb.as_str())
                .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

            frame.render_stateful_widget(scrollbar, self.layout.app, &mut scrollbar_state);

            return;
        }

        // Normal mode settings reuses the graph viewport without graph borders.
        let list = List::new(list_items).block(Block::default().padding(padding));

        frame.render_widget(list, self.layout.graph);

        let mut scrollbar_state = ScrollbarState::new(scrollbar_content_length(total_lines, visible_height)).position(start);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some(self.symbols.scrollbar.begin.as_str()))
            .end_symbol(Some(self.symbols.scrollbar.end.as_str()))
            .track_symbol(Some(self.symbols.scrollbar.track.as_str()))
            .thumb_symbol(self.symbols.scrollbar.thumb.as_str())
            .thumb_style(Style::default().fg(if self.focus == Focus::Viewport { self.theme.COLOR_GREY_600 } else { self.theme.COLOR_BORDER }));

        frame.render_stateful_widget(scrollbar, self.layout.app, &mut scrollbar_state);
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/settings.rs"]
mod tests;
