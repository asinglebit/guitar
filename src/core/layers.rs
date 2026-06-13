use crate::helpers::colors::ColorPicker;
use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Eq, Hash, PartialEq, Clone)]
pub enum LayerTypes {
    // Earlier variants win when layers are baked into visible spans.
    Commits = 0,
    Merges = 1,
    Pipes = 2,
}

#[derive(Clone)]
pub struct LayerBuilder {
    layers: HashMap<LayerTypes, Vec<(String, Color)>>,
    color: Rc<RefCell<ColorPicker>>,
}

impl LayerBuilder {
    pub fn new(color: Rc<RefCell<ColorPicker>>) -> Self {
        Self { layers: HashMap::new(), color }
    }

    pub fn add(&mut self, layer: LayerTypes, symbol: String, lane_idx: usize, custom: Option<Color>) {
        // Lane color is the default, but callers can override for special markers.
        self.layers.entry(layer).or_default().push((symbol, custom.unwrap_or(self.color.borrow().get_lane(lane_idx))));
    }
}

// Small facade used by the graph renderer to collect symbols per visual layer.
#[derive(Clone)]
pub struct LayersContext {
    pub builder: LayerBuilder,
}

impl LayersContext {
    pub fn clear(&mut self) {
        self.builder.layers.clear();
    }
    pub fn commit(&mut self, sym: &str, lane: usize) {
        self.builder.add(LayerTypes::Commits, sym.to_string(), lane, None);
    }
    pub fn pipe(&mut self, sym: &str, lane: usize) {
        self.builder.add(LayerTypes::Pipes, sym.to_string(), lane, None);
    }
    pub fn merge(&mut self, sym: &str, lane: usize) {
        self.builder.add(LayerTypes::Merges, sym.to_string(), lane, None);
    }
    pub fn pipe_custom(&mut self, sym: &str, lane: usize, color: Color) {
        self.builder.add(LayerTypes::Pipes, sym.to_string(), lane, Some(color));
    }
    pub fn bake(&mut self, spans: &mut Vec<Span>) {
        // Trim unused right-side columns before compositing layers.
        for layer in [LayerTypes::Commits, LayerTypes::Merges, LayerTypes::Pipes] {
            if let Some(tokens) = self.builder.layers.get_mut(&layer) {
                while tokens.last().is_some_and(|(sym, _)| sym.trim().is_empty()) {
                    tokens.pop();
                }
            }
        }

        // Composite up to the widest layer so sparse merge lines still render.
        let max_len = [LayerTypes::Commits, LayerTypes::Merges, LayerTypes::Pipes].iter().filter_map(|layer| self.builder.layers.get(layer)).map(|tokens| tokens.len()).max().unwrap_or(0);

        for token_index in 0..max_len {
            let mut symbol = " ";
            let mut color: Color = Color::Black;

            // First non-empty layer at this column wins.
            for layer in [LayerTypes::Commits, LayerTypes::Merges, LayerTypes::Pipes] {
                if let Some(tokens) = self.builder.layers.get(&layer)
                    && token_index < tokens.len()
                    && let Some((_symbol, _color)) = tokens.get(token_index)
                    && _symbol.trim() != ""
                {
                    symbol = _symbol;
                    color = *_color;
                    break;
                }
            }
            spans.push(Span::styled(symbol.to_string(), Style::default().fg(color)));
        }
    }
}

// Convenience macro keeps render_graph_range readable while creating a fresh layer context.
#[macro_export]
macro_rules! layers {
    ($color:expr) => {{
        let builder = $crate::core::layers::LayerBuilder::new($color);
        let ctx = $crate::core::layers::LayersContext { builder };
        ctx
    }};
}
