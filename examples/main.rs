use iced::Length::Shrink;
use iced::widget::{button, center, column, pick_list, row};
use iced::{Center, Element, alignment};
use iced_widget::horizontal_space;
use texty::textbox;
use texty::textbox::span;

pub fn main() -> iced::Result {
    iced::application(Texty::default, Texty::update, Texty::view)
        .title("iced • editable textbox")
        .window_size([600.0, 400.0])
        .antialiasing(true)
        .run()
}

struct Texty {
    value: textbox::Content,
    spans: Vec<textbox::Span<'static, (), iced::Font>>,
    align_x: alignment::Horizontal,
    align_y: alignment::Vertical,
}

impl Default for Texty {
    fn default() -> Self {
        let value = textbox::Content::with_text(
            "iced really is the best GUI library in Rust...\n\
            I mean, in the whole world, actually!\n\n\
            Double-click to edit this textbox.\nHit Escape to finish editing.",
        );
        let spans = vec![span(value.text())];
        Self {
            value,
            spans,
            align_x: alignment::Horizontal::Center,
            align_y: alignment::Vertical::Center,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Action(textbox::Action),
    AlignX(alignment::Horizontal),
    AlignY(alignment::Vertical),
    Clear,
}

impl Texty {
    fn update(&mut self, message: Message) {
        match message {
            Message::Action(action) => {
                let is_edit = action.is_edit();
                self.value.perform(action);
                if is_edit {
                    self.spans = vec![span(self.value.text())];
                }
            }
            Message::Clear => {
                self.value = textbox::Content::default();
                self.spans = vec![span(self.value.text())];
            }
            Message::AlignX(x) => self.align_x = x,
            Message::AlignY(y) => self.align_y = y,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let align_x = pick_list(
            ["Left", "Center", "Right"],
            Some(match self.align_x {
                alignment::Horizontal::Left => "Left",
                alignment::Horizontal::Center => "Center",
                alignment::Horizontal::Right => "Right",
            }),
            |s| match s {
                "Left" => Message::AlignX(alignment::Horizontal::Left),
                "Center" => Message::AlignX(alignment::Horizontal::Center),
                "Right" => Message::AlignX(alignment::Horizontal::Right),
                _ => unreachable!(),
            },
        );

        let align_y = pick_list(
            ["Top", "Middle", "Bottom"],
            Some(match self.align_y {
                alignment::Vertical::Top => "Top",
                alignment::Vertical::Center => "Middle",
                alignment::Vertical::Bottom => "Bottom",
            }),
            |s| match s {
                "Top" => Message::AlignY(alignment::Vertical::Top),
                "Middle" => Message::AlignY(alignment::Vertical::Center),
                "Bottom" => Message::AlignY(alignment::Vertical::Bottom),
                _ => unreachable!(),
            },
        );

        center(
            column![
                row![
                    "Align X:",
                    align_x,
                    horizontal_space(),
                    "Align Y:",
                    align_y,
                    horizontal_space(),
                    button("Clear")
                        .on_press(Message::Clear)
                        .style(button::danger)
                ]
                .align_y(Center)
                .spacing(5),
                textbox(&self.spans, &self.value)
                    .on_action(Message::Action)
                    .style(|theme: &iced::Theme, status| textbox::Style {
                        background: theme.extended_palette().background.weak.color.into(),
                        border: iced::Border {
                            width: 1.0,
                            color: theme.extended_palette().background.strong.color.into(),
                            radius: 0.0.into(),
                        },
                        value: theme.extended_palette().background.weak.text.into(),
                        ..textbox::default(theme, status)
                    })
                    .align_x(self.align_x)
                    .align_y(self.align_y)
                    .width(400)
                    .height(200),
            ]
            .width(Shrink)
            .spacing(5),
        )
        .padding(20)
        .into()
    }
}
