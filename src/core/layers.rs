use crate::helpers::colors::ColorPicker;
use ratatui::{
    style::{Color, Style},
    text::Span,
};

#[derive(Clone, Copy)]
struct LayerToken {
    symbol: &'static str,
    color: Color,
}

// Small facade used by the graph renderer to collect symbols per visual layer.
#[derive(Clone)]
pub struct LayersContext {
    commits: Vec<LayerToken>,
    merges: Vec<LayerToken>,
    pipes: Vec<LayerToken>,
    color: ColorPicker,
}

impl LayersContext {
    pub fn new(color: ColorPicker) -> Self {
        Self { commits: Vec::new(), merges: Vec::new(), pipes: Vec::new(), color }
    }

    pub fn clear(&mut self) {
        self.commits.clear();
        self.merges.clear();
        self.pipes.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.commits.reserve(additional);
        self.merges.reserve(additional);
        self.pipes.reserve(additional);
    }

    pub fn commit(&mut self, sym: &'static str, lane: usize) {
        let color = self.color.get_lane(lane);
        self.commits.push(LayerToken { symbol: sym, color });
    }

    pub fn pipe(&mut self, sym: &'static str, lane: usize) {
        let color = self.color.get_lane(lane);
        self.pipes.push(LayerToken { symbol: sym, color });
    }

    pub fn merge(&mut self, sym: &'static str, lane: usize) {
        let color = self.color.get_lane(lane);
        self.merges.push(LayerToken { symbol: sym, color });
    }

    pub fn pipe_custom(&mut self, sym: &'static str, _lane: usize, color: Color) {
        self.pipes.push(LayerToken { symbol: sym, color });
    }

    pub fn bake(&mut self, spans: &mut Vec<Span<'static>>) {
        trim_empty(&mut self.commits);
        trim_empty(&mut self.merges);
        trim_empty(&mut self.pipes);

        // Composite up to the widest layer so sparse merge lines still render.
        let max_len = self.commits.len().max(self.merges.len()).max(self.pipes.len());

        for token_index in 0..max_len {
            let token = self
                .commits
                .get(token_index)
                .filter(|token| !is_empty(token.symbol))
                .or_else(|| self.merges.get(token_index).filter(|token| !is_empty(token.symbol)))
                .or_else(|| self.pipes.get(token_index).filter(|token| !is_empty(token.symbol)));

            let (symbol, color) = token.map(|token| (token.symbol, token.color)).unwrap_or((" ", Color::Black));
            spans.push(Span::styled(symbol, Style::default().fg(color)));
        }
    }
}

fn trim_empty(tokens: &mut Vec<LayerToken>) {
    while tokens.last().is_some_and(|token| is_empty(token.symbol)) {
        tokens.pop();
    }
}

fn is_empty(symbol: &str) -> bool {
    symbol.trim().is_empty()
}
