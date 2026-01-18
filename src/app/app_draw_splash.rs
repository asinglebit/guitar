use crate::app::app::{App, Focus};
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

        // How many rows the actual content will take
        let content_rows =
            if self.spinner.is_running() {
                1
            } else if self.recent.is_empty() && self.repo.is_none() {
                5
            } else if self.recent.is_empty() {
                3
            } else {
                2 + self.recent.len()
            };

        // Logo height by terminal width
        let logo_rows = if self.layout.app.width < 80 {
            1
        } else if self.layout.app.width < 120 {
            9
        } else {
            11
        };

        // Total visible height
        let visible = visible_height;

        // Total splash height
        let splash_rows = logo_rows + content_rows;

        // Center vertically, clamp to 0
        let dummies = visible
            .saturating_sub(splash_rows)
            .saturating_div(2);

        for _ in 0..dummies {
            lines.push(Line::default());
        }

        if self.layout.app.width < 80 {
            lines.push(Line::from(Span::styled("guita╭".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
        } else if self.layout.app.width < 120 {
            lines.push(Line::from(Span::styled("                    :#   :#                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("                         L#                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("  .##5#^.  .#   .#  :C  #C6#   #?##:        ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("  #B   #G  C#   #B  #7   B?        G#       ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered());
            lines.push(Line::from(Span::styled("  #4   B5  B5   B5  B5   B5    1B5B#G  .a###".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("  #b   5?  ?B   B5  B5   B5   ##   ##  B?   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("  .#B~6G!  .#6#~G.  #5   ~##  .##Y~#.  !#   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("      .##                              !B   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
            lines.push(Line::from(Span::styled("     ~G#                               ~?   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered());
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
        if self.spinner.is_running() {
            let icon_spinner = format!("{} ", self.spinner.get_char());
            lines.push(Line::from(vec![Span::styled(format!("{} loading...", icon_spinner), Style::default().fg(self.theme.COLOR_TEXT))]).centered());
        } else if self.recent.is_empty() {
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
                        // Repository lines
        self.recent.iter().enumerate().for_each(|(i, path)| {
            let style = if Some(path) == self.path.as_ref() {
                self.theme.COLOR_GRASS
            } else {
                self.theme.COLOR_TEXT
            };

            let mut line = Line::from(Span::styled(path.clone(), Style::default().fg(style))).centered();

            // Add selection highlighting
            if i == self.splash_selected && self.focus == Focus::Viewport && !self.spinner.is_running() {
                let mut spans = Vec::new();
                spans.push(Span::styled("⏵ ", Style::default().fg(self.theme.COLOR_GRASS)));
                spans.extend(line.spans.clone());
                spans.push(Span::styled(" ⏴", Style::default().fg(self.theme.COLOR_GRASS)));
                line = Line::from(spans).centered();
            }

            lines.push(line);
        });
                    }

// Convert to ListItems for rendering
let list_items: Vec<ListItem> = lines.into_iter().map(ListItem::from).collect();
        // Setup the list
        let list = List::new(list_items).block(Block::default().padding(padding));

        // Render the list
        frame.render_widget(list, self.layout.app);
    }
}
