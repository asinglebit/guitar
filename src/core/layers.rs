use crate::{core::chunk::LaneRef, helpers::colors::ColorPicker};
use ratatui::{
    style::{Color, Style},
    text::Span,
};

#[derive(Clone)]
struct LayerToken<'a> {
    symbol: &'a str,
    color: Color,
}

// Small facade used by the graph renderer to collect symbols per visual layer.
#[derive(Clone)]
pub struct LayersContext<'a> {
    commits: Vec<LayerToken<'a>>,
    merges: Vec<LayerToken<'a>>,
    pipes: Vec<LayerToken<'a>>,
    color: ColorPicker,
    flattened_lanes: Vec<u8>,
}

impl<'a> LayersContext<'a> {
    pub fn new(color: ColorPicker) -> Self {
        Self { commits: Vec::new(), merges: Vec::new(), pipes: Vec::new(), color, flattened_lanes: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.commits.clear();
        self.merges.clear();
        self.pipes.clear();
        self.flattened_lanes.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.commits.reserve(additional);
        self.merges.reserve(additional);
        self.pipes.reserve(additional);
    }

    pub fn set_flattened_lanes(&mut self, flattened_lanes: &[u8]) {
        self.flattened_lanes.clear();
        self.flattened_lanes.extend_from_slice(flattened_lanes);
    }

    pub fn commit(&mut self, sym: &'a str, lane: usize) {
        self.commit_ref(sym, self.lane_ref_for_index(lane));
    }

    pub fn commit_ref(&mut self, sym: &'a str, lane: LaneRef) {
        let color = self.color.get_lane_ref(lane);
        self.commits.push(LayerToken { symbol: sym, color });
    }

    pub fn commit_at(&mut self, token_index: usize, sym: &'a str, lane: usize) {
        while self.commits.len() <= token_index {
            self.commits.push(LayerToken { symbol: " ", color: Color::Black });
        }

        let color = self.color.get_lane_ref(self.lane_ref_for_index(lane));
        self.commits[token_index] = LayerToken { symbol: sym, color };
    }

    pub fn pipe(&mut self, sym: &'a str, lane: usize) {
        self.pipe_ref(sym, self.lane_ref_for_index(lane));
    }

    pub fn pipe_ref(&mut self, sym: &'a str, lane: LaneRef) {
        let color = self.color.get_lane_ref(lane);
        self.pipes.push(LayerToken { symbol: sym, color });
    }

    pub fn merge(&mut self, sym: &'a str, lane: usize) {
        self.merge_ref(sym, self.lane_ref_for_index(lane));
    }

    pub fn merge_ref(&mut self, sym: &'a str, lane: LaneRef) {
        let color = self.color.get_lane_ref(lane);
        self.merges.push(LayerToken { symbol: sym, color });
    }

    pub fn merge_at(&mut self, token_index: usize, sym: &'a str, lane: usize) {
        self.merge_at_ref(token_index, sym, self.lane_ref_for_index(lane));
    }

    pub fn merge_at_ref(&mut self, token_index: usize, sym: &'a str, lane: LaneRef) {
        while self.merges.len() <= token_index {
            self.merges.push(LayerToken { symbol: " ", color: Color::Black });
        }

        if is_empty(&self.merges[token_index].symbol) {
            let color = self.color.get_lane_ref(lane);
            self.merges[token_index] = LayerToken { symbol: sym, color };
        }
    }

    pub fn pipe_custom(&mut self, sym: &'a str, _lane: usize, color: Color) {
        self.pipes.push(LayerToken { symbol: sym, color });
    }

    fn lane_ref_for_index(&self, lane: usize) -> LaneRef {
        LaneRef::new(lane, self.flattened_lanes.get(lane).copied().unwrap_or(0) != 0)
    }

    pub fn bake(&mut self, spans: &mut Vec<Span<'a>>) {
        trim_empty(&mut self.commits);
        trim_empty(&mut self.merges);
        trim_empty(&mut self.pipes);

        // Composite up to the widest layer so sparse merge lines still render.
        let max_len = self.commits.len().max(self.merges.len()).max(self.pipes.len());

        for token_index in 0..max_len {
            let token = self
                .commits
                .get(token_index)
                .filter(|token| !is_empty(&token.symbol))
                .or_else(|| self.merges.get(token_index).filter(|token| !is_empty(&token.symbol)))
                .or_else(|| self.pipes.get(token_index).filter(|token| !is_empty(&token.symbol)));

            let (symbol, color) = token.map(|token| (token.symbol, token.color)).unwrap_or((" ", Color::Black));
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }
    }
}

fn trim_empty(tokens: &mut Vec<LayerToken<'_>>) {
    while tokens.last().is_some_and(|token| is_empty(&token.symbol)) {
        tokens.pop();
    }
}

fn is_empty(symbol: &str) -> bool {
    symbol.as_bytes().iter().all(|byte| *byte == b' ')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::palette::Theme;

    fn line_text(spans: &[Span<'_>]) -> String {
        spans.iter().map(|span| span.content.as_ref()).collect()
    }

    #[test]
    fn bake_preserves_sparse_layer_priority_and_lane_colors() {
        let theme = Theme::classic();
        let mut layers = LayersContext::new(ColorPicker::from_theme(&theme));
        layers.reserve(4);

        layers.pipe("|", 0);
        layers.pipe("|", 1);
        layers.merge_at(1, "-", 1);
        layers.commit_at(2, "o", 2);

        let mut spans = Vec::new();
        layers.bake(&mut spans);

        assert_eq!(line_text(&spans), "|-o");
        assert_eq!(spans[0].style.fg, Some(ColorPicker::from_theme(&theme).get_lane(0)));
        assert_eq!(spans[1].style.fg, Some(ColorPicker::from_theme(&theme).get_lane(1)));
        assert_eq!(spans[2].style.fg, Some(ColorPicker::from_theme(&theme).get_lane(2)));
    }

    #[test]
    fn empty_layer_tokens_are_ascii_spaces_only() {
        assert!(is_empty(""));
        assert!(is_empty(" "));
        assert!(is_empty("  "));
        assert!(!is_empty("\t"));
        assert!(!is_empty("·"));
    }
}
