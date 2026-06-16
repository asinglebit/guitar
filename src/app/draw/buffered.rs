use crate::app::app::App;
use ratatui::{
    Frame,
    buffer::{Buffer, Cell},
    layout::{Position, Rect},
    widgets::{StatefulWidget, Widget},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum DrawSurface {
    Branches,
    Graph,
    Inspector,
    Modal,
    Reflogs,
    Search,
    Settings,
    Splash,
    Stashes,
    Status,
    Statusbar,
    Tags,
    Title,
    Viewer,
    Worktrees,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRender {
    Ready,
    Deferred,
}

impl From<()> for SurfaceRender {
    fn from(_: ()) -> Self {
        Self::Ready
    }
}

pub trait DrawTarget {
    fn area(&self) -> Rect;
    fn buffer_mut(&mut self) -> &mut Buffer;
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P);

    fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer_mut());
    }

    fn render_stateful_widget<W>(&mut self, widget: W, area: Rect, state: &mut W::State)
    where
        W: StatefulWidget,
    {
        widget.render(area, self.buffer_mut(), state);
    }
}

impl DrawTarget for Frame<'_> {
    fn area(&self) -> Rect {
        Frame::area(self)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        Frame::buffer_mut(self)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) {
        Frame::set_cursor_position(self, position);
    }
}

pub struct DrawBuffer {
    base: Buffer,
    buffer: Buffer,
    cursor_position: Option<Position>,
}

impl DrawBuffer {
    pub fn from_target<T: DrawTarget>(target: &mut T) -> Self {
        let area = target.area();
        let mut base = Buffer::empty(area);
        let source = target.buffer_mut();

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let (Some(source_cell), Some(target_cell)) = (source.cell(Position { x, y }), base.cell_mut(Position { x, y })) {
                    *target_cell = source_cell.clone();
                }
            }
        }

        Self { buffer: base.clone(), base, cursor_position: None }
    }

    fn finish(self) -> RenderedSurface {
        let dirty = self.buffer.content.iter().zip(self.base.content.iter()).map(|(next, previous)| next != previous).collect();
        RenderedSurface { area: self.buffer.area, content: self.buffer.content, dirty, cursor_position: self.cursor_position }
    }
}

impl DrawTarget for DrawBuffer {
    fn area(&self) -> Rect {
        self.buffer.area
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) {
        self.cursor_position = Some(position.into());
    }
}

#[derive(Clone)]
pub struct RenderedSurface {
    area: Rect,
    content: Vec<Cell>,
    dirty: Vec<bool>,
    cursor_position: Option<Position>,
}

impl RenderedSurface {
    fn flush_to<T: DrawTarget>(&self, target: &mut T) {
        let target_buffer = target.buffer_mut();

        for (idx, cell) in self.content.iter().enumerate() {
            if !self.dirty.get(idx).copied().unwrap_or(false) {
                continue;
            }

            let x = idx % self.area.width as usize + self.area.x as usize;
            let y = idx / self.area.width as usize + self.area.y as usize;
            let Ok(x) = u16::try_from(x) else {
                continue;
            };
            let Ok(y) = u16::try_from(y) else {
                continue;
            };

            if let Some(target_cell) = target_buffer.cell_mut(Position { x, y }) {
                *target_cell = cell.clone();
            }
        }
    }
}

#[derive(Default)]
pub struct SurfaceBuffers {
    fronts: HashMap<DrawSurface, RenderedSurface>,
}

impl SurfaceBuffers {
    pub fn clear(&mut self) {
        self.fronts.clear();
    }

    fn store(&mut self, surface: DrawSurface, rendered: RenderedSurface) {
        self.fronts.insert(surface, rendered);
    }

    fn replay<T: DrawTarget>(&self, surface: DrawSurface, target: &mut T) -> bool {
        let Some(rendered) = self.fronts.get(&surface) else {
            return false;
        };

        if rendered.area != target.area() {
            return false;
        }

        rendered.flush_to(target);
        if let Some(cursor_position) = rendered.cursor_position {
            target.set_cursor_position(cursor_position);
        }
        true
    }
}

impl App {
    pub fn draw_surface<T, F, R>(&mut self, target: &mut T, surface: DrawSurface, draw: F)
    where
        T: DrawTarget,
        F: FnOnce(&mut Self, &mut DrawBuffer) -> R,
        R: Into<SurfaceRender>,
    {
        let mut back = DrawBuffer::from_target(target);
        let render = draw(self, &mut back).into();
        let rendered = back.finish();

        match render {
            SurfaceRender::Ready => {
                rendered.flush_to(target);
                if let Some(cursor_position) = rendered.cursor_position {
                    target.set_cursor_position(cursor_position);
                }
                self.surface_buffers.store(surface, rendered);
            },
            SurfaceRender::Deferred => {
                if !self.surface_buffers.replay(surface, target) {
                    rendered.flush_to(target);
                    if let Some(cursor_position) = rendered.cursor_position {
                        target.set_cursor_position(cursor_position);
                    }
                    self.surface_buffers.store(surface, rendered);
                }
            },
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/draw/buffered.rs"]
mod tests;
