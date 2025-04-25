//! A shape that can be drawn on a [`Frame`].
use iced::advanced::graphics::geometry;
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::Rectangle;

/// A shape that can be drawn on some position of a [`Frame`].
pub trait Drawable<R: geometry::Renderer>: std::fmt::Debug {
    fn draw(&self, frame: &mut Frame<R>, bounds: Rectangle, alpha: f32) {
        frame.fill(&self.path(bounds), self.fill().scale_alpha(alpha));
        frame.stroke(&self.path(bounds), self.stroke());
    }
    fn fill(&self) -> iced::Color;
    fn stroke(&self) -> Stroke;
    fn path(&self, bounds: Rectangle) -> Path;
}
