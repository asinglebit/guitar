use crate::app::app::{App, Direction, Focus};
use ratatui::{
    Frame,
    style::Style,
    text::{Line, Span},
    widgets::{Block, List, ListItem},
};

impl App {
    #[rustfmt::skip]
    pub fn draw_splash(&mut self, frame: &mut Frame) {
        // Padding
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };

        // Calculate maximum available width for text
        let available_width = self.layout.graph.width.saturating_sub(1) as usize;
        let _max_text_width = available_width.saturating_sub(2);

        // Get vertical dimensions
        let total_lines = self.viewer_lines.len();
        let visible_height = if self.layout_config.is_zen { self.layout.graph.height.saturating_sub(4) as usize } else { self.layout.graph.height.saturating_sub(2) as usize };

        // Clamp selection
        if total_lines == 0 {
            self.viewer_selected = 0;
        } else if self.viewer_selected >= total_lines {
            self.viewer_selected = total_lines.saturating_sub(1);
        }

        // Trap selection
        self.trap_selection(self.viewer_selected, &self.viewer_scroll, total_lines, visible_height);

        // Calculate scroll
        let start = self.viewer_scroll.get().min(total_lines.saturating_sub(visible_height));
        let _end = (start + visible_height).min(total_lines);

        // Setup list items
        let mut lines: Vec<Line> = Vec::new();
        self.splash_selections = Vec::new();

        let dummies = if self.layout.app.width < 80 {
            (visible_height / 2).saturating_sub((3 + 1 - 3) / 2)
        } else if self.layout.app.width < 120 {
            (visible_height / 2).saturating_sub((3 + 9 - 3) / 2)
        } else {
            (visible_height / 2).saturating_sub((3 + 11 - 3) / 2)
        };

        for _ in 0..dummies {
            lines.push(Line::default());
        }

        if self.layout.app.width < 80 {
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::from(Span::styled("guita╭".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::default());
            lines.push(Line::default());
        } else if self.layout.app.width < 120 {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled("                    :#   :#                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("                         L#                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("  .##5#^.  .#   .#  :C  #C6#   #?##:        ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("  #B   #G  C#   #B  #7   B?        G#       ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("  #4   B5  B5   B5  B5   B5    1B5B#G  .a###".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("  #b   5?  ?B   B5  B5   B5   ##   ##  B?   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("  .#B~6G!  .#6#~G.  #5   ~##  .##Y~#.  !#   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("      .##                              !B   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("     ~G#                               ~?   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::default());
        } else {
            lines.push(Line::from(Span::styled("                                 :GG~        .?Y.                                ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("       ....        ..      ..   .....      . ^BG: ..       .....                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("    .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("   .5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("   7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("   ?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("   ^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("    ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("      .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("    .?!^^^!5G7                                                          YB5      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("    .!?JJJ?!:                                                           75?      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
        }

        lines.push(Line::default());
        if self.recent.is_empty() {
            lines.push(Line::from(vec![Span::styled("made with ♡".to_string(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            lines.push(Line::default());
            lines.push(Line::from(vec![Span::styled("https://github.com/asinglebit/guitar".to_string(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            if self.repo.is_none() {
                lines.push(Line::default());
                lines.push(Line::from(vec![Span::styled("! not a valid git repository !".to_string(), Style::default().fg(self.theme.COLOR_ORANGE))]).centered());
            }
        } else {
            lines.push(Line::from(vec![Span::styled("recent repositories:".to_string(), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
            lines.push(Line::default());
            self.recent.iter().for_each(|path| {
                self.splash_selections.push(lines.len());
                let style = if Some(path) == self.path.as_ref() { self.theme.COLOR_GRASS } else { self.theme.COLOR_TEXT };
                lines.push(Line::from(Span::styled(path.clone(), Style::default().fg(style))).centered());
            });
        }

        // Snap to nearest selectable line if needed
        if !self.splash_selections.contains(&self.splash_selected) {
            // Find nearest selectable line above or below
            let mut nearest = None;

            // Moving down
            if self.last_input_direction == Some(Direction::Down) {
                nearest = self.splash_selections.iter().copied().find(|&i| i > self.splash_selected);
            }

            // Moving up
            if nearest.is_none() && self.last_input_direction == Some(Direction::Up) {
                nearest = self.splash_selections.iter().rev().copied().find(|&i| i < self.splash_selected);
            }

            // Fallback to nearest by distance if neither direction flag is set
            if nearest.is_none() {
                nearest = self.splash_selections.iter().min_by_key(|&&i| i.abs_diff(self.splash_selected)).copied();
            }

            if let Some(target) = nearest {
                self.splash_selected = target;
            }
        }

        // Setup list items
        let list_items: Vec<ListItem> = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let absolute_idx = start + i;
                let mut item = line.clone();

                if item.spans.is_empty() {
                    item.spans.push(Span::raw(" "));
                }

                if absolute_idx == self.splash_selected && self.focus == Focus::Viewport {
                    let mut spans = Vec::new();
                    spans.push(Span::styled("⏵ ", Style::default().fg(self.theme.COLOR_GRASS)));
                    spans.extend(item.spans.clone());
                    spans.push(Span::styled(" ⏴", Style::default().fg(self.theme.COLOR_GRASS)));
                    item = Line::from(spans).centered();
                }

                ListItem::from(item)
            })
            .collect();

        // Setup the list
        let list = List::new(list_items).block(Block::default().padding(padding));

        // Render the list
        frame.render_widget(list, self.layout.app);
    }
}
