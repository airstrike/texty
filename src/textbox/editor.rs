// pub use iced::advanced::graphics::text::Editor;
// use iced::advanced::text::{self, Span};
// use iced::widget::markdown;

// use iced::Element;

// pub trait EditorTrait: text::Editor {
// fn parsed<Message, Theme, Renderer>(&self) -> Element<'_, Message, Theme, Renderer>;
// fn spans<Message>(&self) -> Vec<Span<'static, Message, iced::Font>>;
// fn lines(&self) -> impl Iterator<Item = impl std::ops::Deref<Target = str> + '_>;
// }

// impl EditorTrait for Editor {
// fn parsed<Message, Theme, Renderer>(&self) -> Element<'_, Message, Theme, Renderer> {
//     markdown::view(self.buffer())
// }

// fn spans<Message>(&self) -> Vec<Span<'static, Message, iced::Font>> {
//     let buffer = self.buffer();
//     Span::from(buffer.lines.into_iter().map(|line| line.to_string())).collect()
// }

// fn lines(&self) -> impl Iterator<Item = impl std::ops::Deref<Target = str> + '_> {
//     struct Lines<'a, Renderer: text::Renderer> {
//         internal: std::cell::Ref<'a, Internal<Renderer>>,
//         current: usize,
//     }

//     impl<'a, Renderer: text::Renderer> Iterator for Lines<'a, Renderer> {
//         type Item = std::cell::Ref<'a, str>;

//         fn next(&mut self) -> Option<Self::Item> {
//             let line =
//                 std::cell::Ref::filter_map(std::cell::Ref::clone(&self.internal), |internal| {
//                     internal.editor.line(self.current)
//                 })
//                 .ok()?;

//             self.current += 1;

//             Some(line)
//         }
//     }

//     Lines {
//         internal: self.borrow(),
//         current: 0,
//     }
// }
// }
