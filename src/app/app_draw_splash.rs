#[rustfmt::skip]
use ratatui::{
    Frame,
    style::Style,
    text::{
        Span,
        Line
    },
    widgets::{
        Block,
        List,
        ListItem
    }
};
#[rustfmt::skip]
use crate::{
    app::app::{
        App
    }
};

impl App {

    pub fn draw_splash(&mut self, frame: &mut Frame) {
        
        // Padding
        let padding = ratatui::widgets::Padding { left: 1, right: 1, top: 0, bottom: 0 };
        
        // Calculate maximum available width for text
        let available_width = self.layout.graph.width.saturating_sub(1) as usize;
        let _max_text_width = available_width.saturating_sub(2);

        // Get vertical dimensions
        let total_lines = self.viewer_lines.len();
        let visible_height = self.layout.graph.height as usize - 2;

        // Clamp selection
        if total_lines == 0 {
            self.viewer_selected = 0;
        } else if self.viewer_selected >= total_lines {
            self.viewer_selected = total_lines - 1;
        }
        
        // Trap selection
        self.trap_selection(self.viewer_selected, &self.viewer_scroll, total_lines, visible_height);

        // Calculate scroll
        let start = self.viewer_scroll.get().min(total_lines.saturating_sub(visible_height));
        let _end = (start + visible_height).min(total_lines);

        // Setup list items
        let mut list_items: Vec<ListItem> = Vec::new();

        let dummies = if self.layout.app.width < 80 {
                (visible_height / 2).saturating_sub((3 + 1 - 3) / 2)
            } else if self.layout.app.width < 120 {
                (visible_height / 2).saturating_sub((3 + 9 - 3) / 2)
            } else {
                (visible_height / 2).saturating_sub((3 + 11 - 3) / 2)
            };

        for _ in 0..dummies { list_items.push(ListItem::from(Line::default())); }

        if self.layout.app.width < 80 {
            list_items.push(ListItem::from(Line::from(Span::styled("guita╭".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));
        } else if self.layout.app.width < 120 {
            list_items.push(ListItem::from(Line::from(Span::styled("                    :#   :#                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("                         L#                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("  .##5#^.  .#   .#  :C  #C6#   #?##:        ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("  #B   #G  C#   #B  #7   B?        G#       ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("  #4   B5  B5   B5  B5   B5    1B5B#G  .a###".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("  #b   5?  ?B   B5  B5   B5   ##   ##  B?   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("  .#B~6G!  .#6#~G.  #5   ~##  .##Y~#.  !#   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("      .##                              !B   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));
            list_items.push(ListItem::from(Line::from(Span::styled("     ~G#                               ~?   ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));
        } else {
            list_items.push(ListItem::from(Line::from(Span::styled("                                 :GG~        .?Y.                                ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("       ....        ..      ..   .....      . ^BG: ..       .....                 ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("    .7555YY7JP^   ~PJ     ~PJ  ?YY5PP~    7YY5BGYYYYJ.   J555YY557.              ".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("   .5B?.  :JBB~   !#5     !#5  ...PB~     ...^BG:....    ~:.   .7#5           :^^".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("   7#5     .GB~   !B5     !B5     PB~        :BG.        .~7??J?JBG:      .~JPPPY".to_string(), Style::default().fg(self.theme.COLOR_GRASS))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("   ?#Y      PB~   !B5     !B5     PB~        :BG.       7GP7~^^^!BG:     ~5GY!:. ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("   ^GB~    7BB~   ^BG.   .YB5     5#7        :BB:       P#!     JBG:    ^GG7     ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("    ^5G5JJYJPB~    JBP???YYB5     ^5GYJJ?.    7GPJ???.  ~PGJ77?5J5B!    JG5      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("      .^~^..GB:     :~!!~. ^^       :~~~~      .^~~~~    .^!!!~. .^:    JG5      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("    .?!^^^!5G7                                                          YB5      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));    
            list_items.push(ListItem::from(Line::from(Span::styled("    .!?JJJ?!:                                                           75?      ".to_string(), Style::default().fg(self.theme.COLOR_GREEN))).centered()));    
        }
        list_items.push(ListItem::from(Line::default()));
        list_items.push(ListItem::from(Line::from(vec![
            Span::styled("made with ♡".to_string(), Style::default().fg(self.theme.COLOR_TEXT))
        ]).centered()));
        list_items.push(ListItem::from(Line::from(vec![
            Span::styled("https://github.com/asinglebit/guitar".to_string(), Style::default().fg(self.theme.COLOR_TEXT))
        ]).centered()));

        // Setup the list
        let list = List::new(list_items).block(Block::default().padding(padding));

        // Render the list
        frame.render_widget(list, self.layout.app);
    }
}
