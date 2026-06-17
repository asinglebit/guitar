use crate::app::app::{App, ContextMenuAction, ContextMenuState, Direction, Focus, SettingsTab, Viewport};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
};

impl App {
    pub(crate) fn open_context_menu(&mut self, column: u16, row: u16) {
        if self.is_modal_focus() {
            return;
        }

        self.mouse_drag = None;
        self.last_mouse_click = None;
        self.context_menu = Some(ContextMenuState { column, row, selected: self.first_enabled_context_menu_index() });
    }

    pub(crate) fn close_context_menu(&mut self) {
        self.context_menu = None;
    }

    pub(crate) fn handle_context_menu_key_event(&mut self, key_event: KeyEvent) -> bool {
        if self.context_menu.is_none() || self.is_modal_focus() {
            return false;
        }

        if key_event.modifiers != KeyModifiers::NONE {
            return true;
        }

        match key_event.code {
            KeyCode::Esc => self.close_context_menu(),
            KeyCode::Enter => self.activate_context_menu_selected(),
            KeyCode::Up | KeyCode::Char('k') => self.move_context_menu_selection(Direction::Up),
            KeyCode::Down | KeyCode::Char('j') => self.move_context_menu_selection(Direction::Down),
            _ => {},
        }

        true
    }

    pub(crate) fn handle_context_menu_left_click(&mut self, column: u16, row: u16) -> bool {
        if self.context_menu.is_none() {
            return false;
        }

        self.mouse_drag = None;
        self.last_mouse_click = None;

        if let Some(index) = self.context_menu_item_at(column, row) {
            let action = ContextMenuAction::ALL[index];
            if self.context_menu_action_enabled(action) {
                if let Some(menu) = &mut self.context_menu {
                    menu.selected = index;
                }
                self.activate_context_menu_action(action);
            }
            return true;
        }

        if self.context_menu_area_for_input().is_some_and(|area| rect_contains(area, column, row)) {
            return true;
        }

        self.close_context_menu();
        true
    }

    pub(crate) fn context_menu_action_enabled(&self, action: ContextMenuAction) -> bool {
        match action {
            ContextMenuAction::Settings => self.repo.is_some(),
            ContextMenuAction::Splash | ContextMenuAction::Exit => true,
        }
    }

    pub(crate) fn context_menu_area_for_bounds(&self, bounds: Rect) -> Option<Rect> {
        self.context_menu.map(|menu| menu.area(bounds)).filter(|area| area.width > 0 && area.height > 0)
    }

    fn context_menu_item_at(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.context_menu_area_for_input()?;
        if !rect_contains(area, column, row) {
            return None;
        }

        if column <= area.x || column >= area.x.saturating_add(area.width).saturating_sub(1) {
            return None;
        }

        if row <= area.y || row >= area.y.saturating_add(area.height).saturating_sub(1) {
            return None;
        }

        let index = row.saturating_sub(area.y).saturating_sub(1) as usize;
        (index < ContextMenuAction::ALL.len()).then_some(index)
    }

    fn context_menu_area_for_input(&self) -> Option<Rect> {
        self.context_menu_area_for_bounds(self.context_menu_bounds())
    }

    fn context_menu_bounds(&self) -> Rect {
        let mut bounds = self.layout.app;
        for rect in [self.layout.title_left, self.layout.title_right, self.layout.statusbar_left, self.layout.statusbar_right] {
            bounds = union_rect(bounds, rect);
        }
        bounds
    }

    fn first_enabled_context_menu_index(&self) -> usize {
        ContextMenuAction::ALL.iter().position(|&action| self.context_menu_action_enabled(action)).unwrap_or(0)
    }

    fn move_context_menu_selection(&mut self, direction: Direction) {
        let Some(menu) = self.context_menu else {
            return;
        };

        let count = ContextMenuAction::ALL.len();
        for offset in 1..=count {
            let index = match direction {
                Direction::Down => (menu.selected + offset) % count,
                Direction::Up => (menu.selected + count - (offset % count)) % count,
            };
            if self.context_menu_action_enabled(ContextMenuAction::ALL[index]) {
                if let Some(menu) = &mut self.context_menu {
                    menu.selected = index;
                }
                return;
            }
        }
    }

    fn activate_context_menu_selected(&mut self) {
        let Some(menu) = self.context_menu else {
            return;
        };
        let Some(&action) = ContextMenuAction::ALL.get(menu.selected) else {
            self.close_context_menu();
            return;
        };
        if self.context_menu_action_enabled(action) {
            self.activate_context_menu_action(action);
        }
    }

    fn activate_context_menu_action(&mut self, action: ContextMenuAction) {
        if !self.context_menu_action_enabled(action) {
            return;
        }

        self.close_context_menu();
        match action {
            ContextMenuAction::Settings => self.open_settings_from_context_menu(),
            ContextMenuAction::Splash => self.open_splash_from_context_menu(),
            ContextMenuAction::Exit => self.exit(),
        }
    }

    fn open_settings_from_context_menu(&mut self) {
        if self.viewport != Viewport::Settings {
            self.settings_tab = SettingsTab::Paths;
            self.settings_selected = 0;
            self.settings_scroll.set(0);
        }
        self.viewport = Viewport::Settings;
        self.focus = Focus::Viewport;
        self.last_input_direction = None;
    }

    fn open_splash_from_context_menu(&mut self) {
        self.viewer_selected = 0;
        self.file_name = None;
        self.viewport = Viewport::Splash;
        self.focus = Focus::Viewport;
        self.last_input_direction = None;

        let selected = self.path.as_ref().and_then(|path| self.recent.iter().position(|recent| recent == path)).unwrap_or(0);
        self.splash_selected = selected.min(self.recent.len().saturating_sub(1));
    }
}

fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    rect.width > 0 && rect.height > 0 && column >= rect.x && column < rect.x.saturating_add(rect.width) && row >= rect.y && row < rect.y.saturating_add(rect.height)
}

fn union_rect(first: Rect, second: Rect) -> Rect {
    if first.width == 0 || first.height == 0 {
        return second;
    }
    if second.width == 0 || second.height == 0 {
        return first;
    }

    let left = first.x.min(second.x);
    let top = first.y.min(second.y);
    let right = first.x.saturating_add(first.width).max(second.x.saturating_add(second.width));
    let bottom = first.y.saturating_add(first.height).max(second.y.saturating_add(second.height));

    Rect::new(left, top, right.saturating_sub(left), bottom.saturating_sub(top))
}
