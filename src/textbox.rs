use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::core::text::{self, Alignment, Difference, Editor, Paragraph, Text, highlighter};
use crate::core::widget::operation::Focusable;
use crate::core::widget::tree::{self, Tree};
use crate::core::widget::{Id, Operation, operation};
use crate::core::{
    Background, Border, Color, Element, Event, Length, Padding, Pixels, Point, Radians, Rectangle,
    Size, Vector, alignment, window,
};
use crate::core::{Clipboard, Layout, Shell, Widget, clipboard, layout, mouse, renderer};
use crate::widget::text::{LineHeight, Shaping, Wrapping};
use iced_graphics::geometry;

pub const EDITOR_INSET: [f32; 2] = [5.0, 5.0];
pub const INSET_VECTOR: Vector = Vector {
    x: EDITOR_INSET[0],
    y: EDITOR_INSET[1],
};

pub mod editor;
pub mod update;

pub use iced::advanced::text::Span;
pub use iced::widget::span;
pub use text::editor::{Action, Cursor, Direction, Edit, Line, LineEnding, Motion};
use update::Update;
pub use update::{Binding, KeyPress};

/// Creates a new [`TextBox`] with the given text.
///
/// # Example
/// ```no_run
/// use iced::font;
/// use iced::{color, Font};
/// use iced::widget::svg as svg_widget;
///
/// use crate::widget::{textbox, span};
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     textbox([
///         span("I am gray!").color(color!(0x313131)),
///         span(" "),
///         span("And I am bold!").font(Font { weight: font::Weight::Bold, ..Font::default() }),
///     ])
///     .background(svg_widget::from_handle(handle))
///     .text_size(20)
///     .into()
/// }
/// ```
pub fn textbox<'a, Link, Message, Theme, Renderer>(
    spans: impl AsRef<[text::Span<'a, Link, Renderer::Font>]> + 'a,
    content: &'a Content<Renderer>,
) -> TextBox<'a, Link, highlighter::PlainText, Message, Theme, Renderer>
where
    Link: Clone + 'static,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + geometry::Renderer + 'a,
    Renderer::Font: 'a,
{
    TextBox::new(spans, content)
}

/// A bunch of editable rich text on top of some background element
#[allow(missing_debug_implementations)]
pub struct TextBox<'a, Link, H, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Link: Clone + 'static,
    H: highlighter::Highlighter,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog,
    Renderer: text::Renderer + 'a,
{
    id: Option<Id>,
    spans: Box<dyn AsRef<[Span<'a, Link, Renderer::Font>]> + 'a>,
    content: &'a Content<Renderer>,
    rotation: Radians,
    text_size: Option<Pixels>,
    padding: Padding,
    line_height: LineHeight,
    width: Length,
    height: Length,
    color: Option<Color>,
    font: Option<Renderer::Font>,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    wrapping: Wrapping,
    class: Theme::Class<'a>,
    key_binding: Option<Box<dyn Fn(KeyPress) -> Option<Binding<Message>> + 'a>>,
    on_edit: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    highlighter_settings: H::Settings,
    highlighter_format: fn(&H::Highlight, &Theme) -> highlighter::Format<Renderer::Font>,
    on_submit: Option<Message>,
    on_blur: Option<Message>,
}

impl<'a, Link, Message, Theme, Renderer>
    TextBox<'a, Link, highlighter::PlainText, Message, Theme, Renderer>
where
    Link: Clone + 'a,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog,
    Renderer: text::Renderer + geometry::Renderer + 'a,
    Renderer::Font: 'a,
{
    /// Creates a new empty [`TextBox`].
    pub fn new(
        spans: impl AsRef<[Span<'a, Link, Renderer::Font>]> + 'a,
        content: &'a Content<Renderer>,
    ) -> Self {
        Self {
            id: None,
            spans: Box::new(spans),
            content,
            rotation: Radians::from(0.0),
            text_size: None,
            padding: Padding::from([5, 5]),
            line_height: LineHeight::default(),
            width: Length::Fill,
            height: Length::Shrink,
            font: None,
            color: None,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            wrapping: Wrapping::WordOrGlyph,
            class: Theme::default(),
            key_binding: None,
            on_edit: None,
            highlighter_settings: (),
            highlighter_format: |_highlight, _theme| highlighter::Format::default(),
            on_submit: None,
            on_blur: None,
        }
    }

    /// Sets the [`Id`] of the [`TextBox`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the rotation of the [`TextBox`] text.
    /// FIXME: This is currently unimplemented and doesn't do anything
    /// because `iced` doesn't really support rotating the renderer with
    /// transformations, but in the future we'd like to let the user
    /// rotate the textbox and have both the background and the text
    /// rotate together.
    pub fn rotated(mut self, rotation: Radians) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets the default size of the [`TextBox`] text.
    pub fn text_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(size.into());
        self
    }

    /// Sets the default [`LineHeight`] of the [`TextBox`] text.
    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the color of the [`TextBox`] text.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the default font of the [`TextBox`] text.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the width of the [`TextBox`] text boundaries.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`TextBox`] text boundaries.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`TextInput`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Centers the [`TextBox`] text, both horizontally and vertically.
    pub fn center(self) -> Self {
        self.align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
    }

    /// Sets the [`alignment::Horizontal`] of the [`TextBox`] text.
    pub fn align_x(mut self, alignment: impl Into<text::Alignment>) -> Self {
        self.align_x = alignment.into();
        self
    }

    /// Sets the [`alignment::Vertical`] of the [`TextBox`] text.
    pub fn align_y(mut self, alignment: impl Into<alignment::Vertical>) -> Self {
        self.align_y = alignment.into();
        self
    }

    /// Sets the [`Wrapping`] strategy of the [`TextBox`] text.
    pub fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Sets the message that should be produced when some action is performed in
    /// the [`TextBox`].
    pub fn on_action(mut self, on_edit: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_edit = Some(Box::new(on_edit));
        self
    }

    /// Sets the message that should be produced when this [`TextBox`] is submitted.
    pub fn on_submit(mut self, on_submit: Message) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    /// Sets the message that should be produced when this [`TextBox`] focus is blurred.
    pub fn on_blur(mut self, on_blur: Message) -> Self {
        self.on_blur = Some(on_blur);
        self
    }

    /// Sets the closure to produce key bindings on key presses.
    ///
    /// See [`Binding`] for the list of available bindings.
    pub fn key_binding(
        mut self,
        key_binding: impl Fn(KeyPress) -> Option<Binding<Message>> + 'a,
    ) -> Self {
        self.key_binding = Some(Box::new(key_binding));
        self
    }

    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`TextBox`].
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

pub struct Content<R = iced::Renderer>(RefCell<Internal<R>>)
where
    R: text::Renderer;

struct Internal<R>
where
    R: text::Renderer,
{
    editor: R::Editor,
    is_dirty: bool,
}

impl<R> Content<R>
where
    R: text::Renderer,
{
    /// Creates an empty [`Content`].
    pub fn new() -> Self {
        Self::with_text("")
    }

    /// Creates a [`Content`] with the given text.
    pub fn with_text(text: &str) -> Self {
        Self(RefCell::new(Internal {
            editor: R::Editor::with_text(text),
            is_dirty: true,
        }))
    }

    /// Performs an [`Action`] on the [`Content`].
    pub fn perform(&mut self, action: Action) {
        let internal = self.0.get_mut();

        internal.editor.perform(action);
        internal.is_dirty = true;
    }

    /// Returns the amount of lines of the [`Content`].
    pub fn line_count(&self) -> usize {
        self.0.borrow().editor.line_count()
    }

    /// Returns the text of the line at the given index, if it exists.
    pub fn line(&self, index: usize) -> Option<Line<'_>> {
        let internal = self.0.borrow();
        let line = internal.editor.line(index)?;

        Some(Line {
            text: Cow::Owned(line.text.into_owned()),
            ending: line.ending,
        })
    }

    /// Returns an iterator of the text of the lines in the [`Content`].
    pub fn lines(&self) -> impl Iterator<Item = Line<'_>> {
        (0..)
            .map(|i| self.line(i))
            .take_while(Option::is_some)
            .flatten()
    }

    /// Returns the text of the [`Content`].
    pub fn text(&self) -> String {
        let mut contents = String::new();
        let mut lines = self.lines().peekable();

        while let Some(line) = lines.next() {
            contents.push_str(&line.text);

            if lines.peek().is_some() {
                contents.push_str(if line.ending == LineEnding::None {
                    LineEnding::default().as_str()
                } else {
                    line.ending.as_str()
                });
            }
        }

        contents
    }

    /// Returns the kind of [`LineEnding`] used for separating lines in the [`Content`].
    pub fn line_ending(&self) -> Option<LineEnding> {
        Some(self.line(0)?.ending)
    }

    /// Returns the selected text of the [`Content`].
    pub fn selection(&self) -> Option<String> {
        self.0.borrow().editor.selection()
    }

    /// Returns the current cursor position of the [`Content`].
    pub fn cursor_position(&self) -> (usize, usize) {
        self.0.borrow().editor.cursor_position()
    }
}

impl<Renderer> Default for Content<Renderer>
where
    Renderer: text::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Renderer> std::fmt::Debug for Content<Renderer>
where
    Renderer: text::Renderer,
    Renderer::Editor: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let internal = self.0.borrow();

        f.debug_struct("Content")
            .field("editor", &internal.editor)
            .field("is_dirty", &internal.is_dirty)
            .finish()
    }
}

struct State<Link, H: highlighter::Highlighter, P: Paragraph> {
    focus: Option<Focus>,
    last_click: Option<mouse::Click>,
    drag_click: Option<mouse::click::Kind>,
    partial_scroll: f32,
    highlighter: RefCell<H>,
    highlighter_settings: H::Settings,
    highlighter_format_address: usize,
    paragraph: P,
    spans: Vec<Span<'static, Link, P::Font>>,
}

#[derive(Debug, Clone, Copy)]
struct Focus {
    updated_at: Instant,
    now: Instant,
    is_window_focused: bool,
}

impl Focus {
    const CURSOR_BLINK_INTERVAL_MILLIS: u128 = 500;

    fn now() -> Self {
        let now = Instant::now();

        Self {
            updated_at: now,
            now,
            is_window_focused: true,
        }
    }

    fn is_cursor_visible(&self) -> bool {
        self.is_window_focused
            && ((self.now - self.updated_at).as_millis() / Self::CURSOR_BLINK_INTERVAL_MILLIS) % 2
                == 0
    }
}

impl<Link, H: highlighter::Highlighter, P: Paragraph> operation::Focusable for State<Link, H, P> {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::now());
    }

    fn unfocus(&mut self) {
        self.focus = None;
    }
}

// Layout function for the editor, following text_editor's pattern
fn layout_editor<H, Renderer>(
    editor: &mut Renderer::Editor,
    limits: &layout::Limits,
    padding: Padding,
    font: Renderer::Font,
    text_size: Pixels,
    line_height: LineHeight,
    wrapping: Wrapping,
    highlighter: &mut H,
) -> layout::Node
where
    H: highlighter::Highlighter,
    Renderer: text::Renderer,
{
    // First make the editor infinitely tall so that we can measure how much the
    // text would take up with `editor.min_bounds()` We need this because the
    // LayoutRuns used by `text::measure` to calculate the size of the content
    // will be limited to the lines that fit the editor's bounds.
    editor.update(
        Size::new(editor.bounds().width, f32::INFINITY),
        font,
        text_size,
        line_height,
        wrapping,
        highlighter,
    );

    // Get the bounds needed to fit this whole text
    let min_bounds = editor.min_bounds();

    // Resize to either the layout limits or whatever we need for the editor
    let node = layout::Node::new(Size::new(
        limits.max().width + padding.horizontal(),
        limits
            .max()
            .height
            .max(min_bounds.height + padding.vertical()),
    ));

    // Update editor with the final computed size
    editor.update(
        node.size(),
        font,
        text_size,
        line_height,
        wrapping,
        highlighter,
    );

    node
}

// Layout function for background, taking maximum space after padding
fn layout_background(limits: &layout::Limits, width: Length, height: Length) -> layout::Node {
    layout::Node::new(limits.resolve(width, height, Size::ZERO))
}

impl<'a, Link, H, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TextBox<'a, Link, H, Message, Theme, Renderer>
where
    Link: Clone + 'static,
    H: highlighter::Highlighter,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog,
    Renderer: text::Renderer<Font = iced::Font> + geometry::Renderer + 'a,
    Renderer::Paragraph: 'a,
{
    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree
            .state
            .downcast_mut::<State<Link, H, Renderer::Paragraph>>();

        // Calculate content bounds with padding
        let padding = self.padding.fit(Size::ZERO, limits.max());
        let content_limits = limits.width(self.width).height(self.height).shrink(padding);

        let text_size = self.text_size.unwrap_or_else(|| renderer.default_size());
        let font = self.font.unwrap_or_else(|| renderer.default_font());

        // Layout each component
        let mut spans_node = layout_spans(
            state,
            renderer,
            &content_limits,
            self.width,
            self.height,
            self.spans.as_ref().as_ref(),
            self.line_height,
            text_size,
            font,
            self.align_x,
            self.align_y,
            self.wrapping,
        );

        let background_node = layout_background(&limits, self.width, self.height);

        let mut internal = self.content.0.borrow_mut();

        if state.highlighter_format_address != self.highlighter_format as usize {
            state.highlighter.borrow_mut().change_line(0);

            state.highlighter_format_address = self.highlighter_format as usize;
        }

        if state.highlighter_settings != self.highlighter_settings {
            state
                .highlighter
                .borrow_mut()
                .update(&self.highlighter_settings);

            state.highlighter_settings = self.highlighter_settings.clone();
        }

        let editor_node = layout_editor::<H, Renderer>(
            &mut internal.editor,
            &content_limits,
            self.padding,
            font,
            text_size,
            self.line_height,
            self.wrapping,
            state.highlighter.borrow_mut().deref_mut(),
        );

        // Position the spans and editor nodes within the padded space
        spans_node = spans_node
            .align(
                alignment::Alignment::Start,
                alignment::Alignment::from(self.align_y),
                content_limits.max(),
            )
            .move_to(Point::new(padding.left, padding.top));

        // Use the maximum of content_bounds and editor's required size,
        // but only if we're focused
        let final_bounds = if state.is_focused() {
            Size::new(
                content_limits.max().width,
                content_limits.max().height.max(editor_node.size().height),
            )
            .expand(padding)
        } else {
            limits.resolve(
                spans_node.size().width + padding.horizontal(),
                spans_node.size().height + padding.vertical(),
                Size::ZERO,
            )
        };

        layout::Node::with_children(final_bounds, vec![spans_node, background_node, editor_node])
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Link, H, Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Link, H, _> {
            paragraph: Renderer::Paragraph::default(),
            focus: None,
            last_click: None,
            drag_click: None,
            partial_scroll: 0.0,
            spans: Vec::new(),
            highlighter: RefCell::new(H::new(&self.highlighter_settings)),
            highlighter_settings: self.highlighter_settings.clone(),
            highlighter_format_address: self.highlighter_format as usize,
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree
            .state
            .downcast_ref::<State<Link, H, Renderer::Paragraph>>();

        let bounds = layout.bounds();

        let is_mouse_over = cursor.is_over(bounds);

        let status = if state.focus.is_some() {
            Status::Focused
        } else if is_mouse_over {
            Status::Hovered
        } else {
            Status::Active
        };

        let style = theme.style(&self.class, status);

        let mut children_layout = layout.children();
        let spans_layout = children_layout.next().unwrap();
        let background_layout = children_layout.next().unwrap();
        let editor_layout = children_layout.next().unwrap();

        let translation = layout.position() - Point::ORIGIN;

        let hovered_span = cursor
            .position_in(layout.bounds())
            .and_then(|position| state.paragraph.hit_span(position));

        let text_bounds = layout.bounds();

        if !state.is_focused() {
            // Draw a stroke around the whole object
            renderer.fill_quad(
                renderer::Quad {
                    bounds: background_layout.bounds(),
                    border: style.border,
                    ..renderer::Quad::default()
                },
                style.background,
            );

            for (index, span) in self.spans.as_ref().as_ref().into_iter().enumerate() {
                let is_hovered_link = span.link.is_some() && Some(index) == hovered_span;

                if span.highlight.is_some()
                    || span.underline
                    || span.strikethrough
                    || is_hovered_link
                {
                    let regions = state.paragraph.span_bounds(index);

                    if let Some(highlight) = span.highlight {
                        for bounds in &regions {
                            let bounds = Rectangle::new(
                                bounds.position()
                                    - Vector::new(span.padding.left, span.padding.top),
                                bounds.size()
                                    + Size::new(span.padding.horizontal(), span.padding.vertical()),
                            );

                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: bounds + translation,
                                    border: highlight.border,
                                    ..Default::default()
                                },
                                highlight.background,
                            );
                        }
                    }

                    if span.underline || span.strikethrough || is_hovered_link {
                        let size = span
                            .size
                            .or(self.text_size)
                            .unwrap_or(renderer.default_size());

                        let line_height = span
                            .line_height
                            .unwrap_or(self.line_height)
                            .to_absolute(size);

                        // let color = span.color.or(style.value).unwrap_or(defaults.text_color);
                        let color = span.color.or(style.value).unwrap_or_else(|| Color::BLACK);

                        let baseline =
                            translation + Vector::new(0.0, size.0 + (line_height.0 - size.0) / 2.0);

                        if span.underline || is_hovered_link {
                            for bounds in &regions {
                                renderer.fill_quad(
                                    renderer::Quad {
                                        bounds: Rectangle::new(
                                            bounds.position() + baseline
                                                - Vector::new(0.0, size.0 * 0.08),
                                            Size::new(bounds.width, 1.0),
                                        ),
                                        ..Default::default()
                                    },
                                    color,
                                );
                            }
                        }

                        if span.strikethrough {
                            for bounds in &regions {
                                renderer.fill_quad(
                                    renderer::Quad {
                                        bounds: Rectangle::new(
                                            bounds.position() + baseline
                                                - Vector::new(0.0, size.0 / 2.0),
                                            Size::new(bounds.width, 1.0),
                                        ),
                                        ..Default::default()
                                    },
                                    color,
                                );
                            }
                        }
                    }
                }
            }
            draw_text(
                self.color.or(style.value),
                renderer,
                defaults,
                spans_layout,
                &state.paragraph,
                &viewport,
            );
        } else {
            let mut internal = self.content.0.borrow_mut();
            let font = self.font.unwrap_or_else(|| renderer.default_font());
            internal.editor.highlight(
                font,
                state.highlighter.borrow_mut().deref_mut(),
                |highlight| (self.highlighter_format)(highlight, theme),
            );

            let inset = INSET_VECTOR;
            let editor_rect = editor_layout.bounds().shrink(EDITOR_INSET);

            renderer.with_layer(*viewport, |renderer| {
                // Draw a stroke around the whole object
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            editor_layout.position(),
                            Size::new(
                                editor_layout.bounds().width,
                                editor_layout
                                    .bounds()
                                    .height
                                    .max(background_layout.bounds().height),
                            ),
                        )
                        .shrink(1.0),
                        border: Border {
                            width: 1.0,
                            color: Color::BLACK.scale_alpha(0.8),
                            radius: 0.0.into(),
                        },
                        ..renderer::Quad::default()
                    },
                    style.background,
                );

                renderer.fill_editor(
                    &internal.editor,
                    editor_rect.position(),
                    style.value.unwrap_or(defaults.text_color),
                    editor_rect,
                );

                if let Some(focus) = state.focus.as_ref() {
                    match internal.editor.cursor() {
                        Cursor::Caret(position) if focus.is_cursor_visible() => {
                            let cursor = Rectangle::new(
                                position + translation,
                                Size::new(
                                    1.0,
                                    self.line_height
                                        .to_absolute(
                                            self.text_size
                                                .unwrap_or_else(|| renderer.default_size()),
                                        )
                                        .into(),
                                ),
                            );

                            if let Some(clipped_cursor) =
                                editor_layout.bounds().intersection(&cursor)
                            {
                                renderer.fill_quad(
                                    renderer::Quad {
                                        bounds: clipped_cursor + inset,
                                        ..renderer::Quad::default()
                                    },
                                    style.value.unwrap_or(defaults.text_color),
                                );
                            }
                        }
                        Cursor::Selection(ranges) => {
                            for range in ranges.into_iter().filter_map(|range| {
                                text_bounds.intersection(&(range + translation))
                            }) {
                                renderer.fill_quad(
                                    renderer::Quad {
                                        bounds: range + inset,
                                        ..renderer::Quad::default()
                                    },
                                    style.selection,
                                );
                            }
                        }
                        Cursor::Caret(_) => {}
                    }
                }
            });
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree
            .state
            .downcast_mut::<State<Link, H, Renderer::Paragraph>>();

        match *event {
            Event::Window(window::Event::Unfocused) => {
                if let Some(focus) = &mut state.focus {
                    focus.is_window_focused = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(focus) = &mut state.focus {
                    focus.is_window_focused = true;
                    focus.updated_at = Instant::now();

                    shell.request_redraw();
                    shell.invalidate_layout();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if let Some(focus) = &mut state.focus {
                    if focus.is_window_focused {
                        focus.now = now;

                        let millis_until_redraw = Focus::CURSOR_BLINK_INTERVAL_MILLIS
                            - (now - focus.updated_at).as_millis()
                                % Focus::CURSOR_BLINK_INTERVAL_MILLIS;

                        shell.request_redraw_at(window::RedrawRequest::At(
                            now + Duration::from_millis(millis_until_redraw as u64),
                        ));
                    }
                }
            }
            _ => {}
        }

        let Some(on_edit) = self.on_edit.as_ref() else {
            return;
        };

        let Some(update) = Update::from_event::<Link, H, Renderer>(
            event,
            state,
            layout.bounds(),
            self.padding,
            cursor,
            self.key_binding.as_deref(),
        ) else {
            return;
        };

        match update {
            Update::Click(click) => match click.kind() {
                mouse::click::Kind::Single => {
                    state.last_click = Some(click);
                    state.drag_click = Some(click.kind());
                    if state.is_focused() {
                        shell.capture_event();
                        shell.publish(on_edit(Action::Click(click.position())));
                        shell.request_redraw();
                    }
                }
                mouse::click::Kind::Double => {
                    state.last_click = Some(click);
                    state.drag_click = Some(click.kind());
                    if state.is_focused() {
                        shell.publish(on_edit(Action::SelectWord));
                        shell.capture_event();
                        shell.request_redraw();
                    } else {
                        state.focus();
                        shell.invalidate_layout();
                        shell.publish(on_edit(Action::Click(click.position())));
                        shell.capture_event();
                    }
                }
                mouse::click::Kind::Triple => {
                    if state.is_focused() {
                        shell.publish(on_edit(Action::SelectAll));
                        shell.capture_event();
                        shell.request_redraw();
                    } else {
                        state.focus();
                        shell.invalidate_layout();
                        shell.publish(on_edit(Action::Click(click.position())));
                        shell.capture_event();
                    }
                }
            },
            Update::Drag(position) => {
                shell.capture_event();
                shell.publish(on_edit(Action::Drag(position)));
            }
            Update::Release => {
                state.drag_click = None;
            }
            Update::Scroll(lines) => {
                let bounds = self.content.0.borrow().editor.bounds();

                if bounds.height >= i32::MAX as f32 {
                    return;
                }

                let lines = lines + state.partial_scroll;
                state.partial_scroll = lines.fract();

                shell.publish(on_edit(Action::Scroll {
                    lines: lines as i32,
                }));
            }
            Update::Binding(binding) => {
                fn apply_binding<
                    Link: Clone + 'static,
                    H: highlighter::Highlighter,
                    R: text::Renderer,
                    Message: std::fmt::Debug + Clone,
                >(
                    binding: Binding<Message>,
                    content: &Content<R>,
                    state: &mut State<Link, H, R::Paragraph>,
                    on_edit: &dyn Fn(Action) -> Message,
                    on_submit: &Option<Message>,
                    on_blur: &Option<Message>,
                    clipboard: &mut dyn Clipboard,
                    shell: &mut Shell<'_, Message>,
                ) {
                    let mut publish_if_focused =
                        |state: &mut State<Link, H, R::Paragraph>, action| {
                            if state.is_focused() {
                                shell.publish(on_edit(action));
                                state.focus();
                                shell.request_redraw();
                            }
                        };

                    match binding {
                        Binding::Unfocus => {
                            if state.is_focused() {
                                state.unfocus();
                                state.drag_click = None;
                                if let Some(on_blur) = on_blur {
                                    shell.publish(on_blur.clone());
                                }
                                shell.request_redraw();
                            }
                        }
                        Binding::Copy => {
                            if let Some(selection) = content.selection() {
                                clipboard.write(clipboard::Kind::Standard, selection);
                            }
                        }
                        Binding::Cut => {
                            if let Some(selection) = content.selection() {
                                clipboard.write(clipboard::Kind::Standard, selection);

                                publish_if_focused(state, Action::Edit(Edit::Delete));
                            }
                        }
                        Binding::Paste => {
                            if let Some(contents) = clipboard.read(clipboard::Kind::Standard) {
                                publish_if_focused(
                                    state,
                                    Action::Edit(Edit::Paste(Arc::new(contents))),
                                );
                            }
                        }
                        Binding::Move(motion) => {
                            publish_if_focused(state, Action::Move(motion));
                        }
                        Binding::Select(motion) => {
                            publish_if_focused(state, Action::Select(motion));
                        }
                        Binding::SelectWord => {
                            publish_if_focused(state, Action::SelectWord);
                        }
                        Binding::SelectLine => {
                            publish_if_focused(state, Action::SelectLine);
                        }
                        Binding::SelectAll => {
                            publish_if_focused(state, Action::SelectAll);
                        }
                        Binding::Insert(c) => {
                            publish_if_focused(state, Action::Edit(Edit::Insert(c)));
                        }
                        Binding::Enter => {
                            publish_if_focused(state, Action::Edit(Edit::Enter));
                        }
                        Binding::Submit => {
                            if state.is_focused() {
                                if let Some(on_submit) = on_submit {
                                    shell.publish(on_submit.clone());
                                }
                                state.unfocus();
                                shell.invalidate_layout();
                            }
                        }
                        Binding::Backspace => {
                            publish_if_focused(state, Action::Edit(Edit::Backspace));
                            shell.request_redraw();
                        }
                        Binding::Delete => {
                            publish_if_focused(state, Action::Edit(Edit::Delete));
                            shell.request_redraw();
                        }
                        Binding::Sequence(sequence) => {
                            for binding in sequence {
                                apply_binding(
                                    binding, content, state, on_edit, on_submit, on_blur,
                                    clipboard, shell,
                                );
                            }
                        }
                        Binding::Custom(message) => {
                            shell.publish(message);
                            shell.request_redraw();
                        }
                    }
                }

                apply_binding(
                    binding,
                    self.content,
                    state,
                    on_edit,
                    &self.on_submit,
                    &self.on_blur,
                    clipboard,
                    shell,
                );

                if let Some(focus) = &mut state.focus {
                    focus.updated_at = Instant::now();
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = state
            .state
            .downcast_ref::<State<Link, H, Renderer::Paragraph>>();

        if cursor.is_over(layout.bounds()) && state.is_focused() {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree
            .state
            .downcast_mut::<State<Link, H, Renderer::Paragraph>>();

        operation.focusable(self.id.as_ref(), layout.bounds(), state);
    }
}

/// Draws text using (roughly) the same logic as the [`Text`] widget.
///
/// Specifically:
///
/// * If no `size` is provided, the default text size of the `Renderer` will be
///   used.
/// * If no `color` is provided, the [`renderer::Style::text_color`] will be
///   used.
/// * The alignment attributes do not affect the position of the bounds of the
///   [`Layout`].
pub fn draw_text<Renderer>(
    color: Option<Color>,
    renderer: &mut Renderer,
    defaults: &renderer::Style,
    layout: Layout<'_>,
    paragraph: &Renderer::Paragraph,
    viewport: &Rectangle,
) where
    Renderer: text::Renderer,
{
    let bounds = layout.bounds();
    let x = match paragraph.align_x() {
        Alignment::Default | Alignment::Left | Alignment::Justified => bounds.x,
        Alignment::Center => bounds.center_x(),
        Alignment::Right => bounds.x + bounds.width,
    };

    let y = match paragraph.align_y() {
        alignment::Vertical::Top => bounds.y,
        alignment::Vertical::Center => bounds.center_y(),
        alignment::Vertical::Bottom => bounds.y + bounds.height,
    };

    // TODO: try style color first before default?
    renderer.fill_paragraph(
        paragraph,
        Point::new(x, y),
        color.unwrap_or(defaults.text_color),
        *viewport,
    );
}

fn layout_spans<Link, H, Renderer>(
    state: &mut State<Link, H, Renderer::Paragraph>,
    _renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    height: Length,
    spans: &[Span<'_, Link, Renderer::Font>],
    line_height: LineHeight,
    size: Pixels,
    font: Renderer::Font,
    align_x: text::Alignment,
    align_y: alignment::Vertical,
    wrapping: Wrapping,
) -> layout::Node
where
    Link: Clone,
    H: highlighter::Highlighter,
    Renderer: text::Renderer,
{
    layout::sized(limits, width, height, |limits| {
        let bounds = limits.max();

        let text_with_spans = || Text {
            content: spans,
            bounds,
            size,
            line_height,
            font,
            align_x,
            align_y,
            shaping: Shaping::Advanced,
            wrapping,
        };

        if state.spans != spans {
            state.paragraph = Renderer::Paragraph::with_spans(text_with_spans());
            state.spans = spans.iter().cloned().map(Span::to_static).collect();
        } else {
            match state.paragraph.compare(Text {
                content: (),
                bounds,
                size,
                line_height,
                font,
                align_x,
                align_y,
                shaping: Shaping::Advanced,
                wrapping,
            }) {
                Difference::None => {}
                Difference::Bounds => {
                    state.paragraph.resize(bounds);
                }
                Difference::Shape => {
                    state.paragraph = Renderer::Paragraph::with_spans(text_with_spans());
                }
            }
        }

        state.paragraph.min_bounds()
    })
}

impl<'a, Link, H, Message, Theme, Renderer> From<TextBox<'a, Link, H, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Link: Clone + 'a,
    H: highlighter::Highlighter,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer<Font = iced::Font> + geometry::Renderer + 'a,
{
    fn from(
        text: TextBox<'a, Link, H, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}

/// The possible status of a [`TextBox`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`TextBox`] can be interacted with.
    Active,
    /// The [`TextBox`] is being hovered.
    Hovered,
    /// The [`TextBox`] is focused.
    Focused,
    /// The [`TextBox`] cannot be interacted with.
    Disabled,
}

/// The appearance of a textbox.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the textbox.
    pub background: Background,
    /// The [`Border`] of the textbox.
    pub border: Border,
    /// The [`Color`] of the icon of the textbox.
    pub icon: Color,
    /// The [`Color`] of the placeholder of the textbox.
    pub placeholder: Color,
    /// The [`Color`] of the value of the textbox.
    pub value: Option<Color>,
    /// The [`Color`] of the selection of the textbox.
    pub selection: Color,
}

/// The theme catalog of a [`TextBox`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`TextBox`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for iced::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a [`TextBox`].
pub fn default(theme: &iced::Theme, status: Status) -> Style {
    let palette = theme.extended_palette();

    let active = Style {
        background: palette.background.base.color.into(),
        border: Border {
            radius: 2.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
        icon: palette.background.weak.text,
        placeholder: palette.background.strong.color,
        value: Some(palette.background.base.text),
        selection: palette.primary.weak.color,
    };

    match status {
        Status::Active => active,
        Status::Hovered => Style {
            border: Border {
                color: palette.background.base.text,
                ..active.border
            },
            ..active
        },
        Status::Focused => Style {
            border: Border {
                color: palette.primary.strong.color,
                ..active.border
            },
            ..active
        },
        Status::Disabled => Style {
            background: palette.background.weak.color.into(),
            value: Some(active.placeholder),
            ..active
        },
    }
}

impl<'a, Link, H, Message, Theme, Renderer> std::fmt::Display
    for TextBox<'a, Link, H, Message, Theme, Renderer>
where
    Link: Clone + 'a,
    H: highlighter::Highlighter,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog,
    Renderer: text::Renderer<Font = iced::Font> + 'a,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextBox({})", self.content.text())
    }
}

impl<'a, Link, H, Message, Theme, Renderer> std::fmt::Debug
    for TextBox<'a, Link, H, Message, Theme, Renderer>
where
    Link: Clone + 'a,
    H: highlighter::Highlighter,
    Message: std::fmt::Debug + Clone + 'a,
    Theme: Catalog,
    Renderer: text::Renderer<Font = iced::Font> + 'a,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextBox")
            .field("content", &self.content.text())
            .finish()
    }
}
