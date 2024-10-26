//! Build and show dropdown menus.
use iced::advanced::text::{self, Text};
use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout};
use iced::advanced::{Shell, Widget};
use iced::alignment;
use iced::border::{self, Border};
use iced::event::{self, Event};
use iced::touch;
use iced::widget::scrollable::{self, Scrollable};
use iced::{
    Background, Color, Element, Length, Padding, Pixels, Point, Rectangle,
    Size, Theme, Vector,
};

/// A list of selectable options.
#[allow(missing_debug_implementations)]
pub struct Menu<
    'a,
    'b,
    T,
    Message,
    Theme = iced::Theme,
    Renderer = iced::Renderer,
> where
    Theme: Catalog,
    Renderer: text::Renderer,
    'b: 'a,
{
    state: &'a mut State,
    options: &'a [T],
    disabled: Option<Vec<bool>>,
    hovered_option: &'a mut Option<usize>,
    on_selected: Box<dyn FnMut(T) -> Message + 'a>,
    on_option_hovered: Option<&'a dyn Fn(T) -> Message>,
    width: f32,
    padding: Padding,
    text_size: Option<Pixels>,
    text_line_height: text::LineHeight,
    text_shaping: text::Shaping,
    font: Option<Renderer::Font>,
    class: &'a <Theme as Catalog>::Class<'b>,
}

impl<'a, 'b, T, Message, Theme, Renderer>
    Menu<'a, 'b, T, Message, Theme, Renderer>
where
    T: ToString + Clone,
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
    'b: 'a,
{
    /// Creates a new [`Menu`] with the given [`State`], a list of options,
    /// the message to produced when an option is selected, and its [`Style`].
    pub fn new(
        state: &'a mut State,
        options: &'a [T],
        hovered_option: &'a mut Option<usize>,
        on_selected: impl FnMut(T) -> Message + 'a,
        disabled: Option<Vec<bool>>,
        on_option_hovered: Option<&'a dyn Fn(T) -> Message>,
        class: &'a <Theme as Catalog>::Class<'b>,
    ) -> Self {
        Menu {
            state,
            options,
            disabled,
            hovered_option,
            on_selected: Box::new(on_selected),
            on_option_hovered,
            width: 0.0,
            padding: Padding::ZERO,
            text_size: None,
            text_line_height: text::LineHeight::default(),
            text_shaping: text::Shaping::Basic,
            font: None,
            class,
        }
    }

    /// Sets the width of the [`Menu`].
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the [`Padding`] of the [`Menu`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the text size of the [`Menu`].
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Menu`].
    pub fn text_line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.text_line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Menu`].
    pub fn text_shaping(mut self, shaping: text::Shaping) -> Self {
        self.text_shaping = shaping;
        self
    }

    /// Sets the font of the [`Menu`].
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Turns the [`Menu`] into an overlay [`Element`] at the given target
    /// position.
    ///
    /// The `target_height` will be used to display the menu either on top
    /// of the target or under it, depending on the screen position and the
    /// dimensions of the [`Menu`].
    pub fn overlay(
        self,
        position: Point,
        target_height: f32,
    ) -> overlay::Element<'a, Message, Theme, Renderer> {
        overlay::Element::new(Box::new(Overlay::new(
            position,
            self,
            target_height,
        )))
    }
}

/// The local state of a [`Menu`].
#[derive(Debug)]
pub struct State {
    tree: Tree,
}

impl State {
    /// Creates a new [`State`] for a [`Menu`].
    pub fn new() -> Self {
        Self {
            tree: Tree::empty(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, 'b, T, Message, Theme, Renderer>
    List<'a, 'b, T, Message, Theme, Renderer>
where
    T: Clone + ToString,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// Calculate the index of an option based on a cursor position within the list bounds
    fn option_index_at(
        &self,
        cursor_position: Point,
        renderer: &Renderer,
    ) -> Option<usize> {
        let text_size =
            self.text_size.unwrap_or_else(|| renderer.default_size());
        let option_height =
            f32::from(self.text_line_height.to_absolute(text_size))
                + self.padding.vertical();

        let index = (cursor_position.y / option_height) as usize;

        if index < self.options.len() {
            Some(index)
        } else {
            None
        }
    }

    /// Check if an option at the given index is disabled
    fn is_disabled(&self, index: usize) -> bool {
        self.disabled
            .as_ref()
            .and_then(|d| d.get(index))
            .copied()
            .unwrap_or(false)
    }
}

struct Overlay<'a, 'b, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: renderer::Renderer,
{
    position: Point,
    state: &'a mut Tree,
    list: Scrollable<'a, Message, Theme, Renderer>,
    width: f32,
    target_height: f32,
    class: &'a <Theme as Catalog>::Class<'b>,
}

impl<'a, 'b, Message, Theme, Renderer> Overlay<'a, 'b, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + scrollable::Catalog + 'a,
    Renderer: text::Renderer + 'a,
    'b: 'a,
{
    pub fn new<T>(
        position: Point,
        menu: Menu<'a, 'b, T, Message, Theme, Renderer>,
        target_height: f32,
    ) -> Self
    where
        T: Clone + ToString,
    {
        let Menu {
            state,
            options,
            disabled,
            hovered_option,
            on_selected,
            on_option_hovered,
            width,
            padding,
            font,
            text_size,
            text_line_height,
            text_shaping,
            class,
        } = menu;

        let list = Scrollable::new(List {
            options,
            disabled,
            hovered_option,
            on_selected,
            on_option_hovered,
            font,
            text_size,
            text_line_height,
            text_shaping,
            padding,
            class,
        });

        state.tree.diff(&list as &dyn Widget<_, _, _>);

        Self {
            position,
            state: &mut state.tree,
            list,
            width,
            target_height,
            class,
        }
    }
}

impl<'a, 'b, Message, Theme, Renderer>
    iced::advanced::Overlay<Message, Theme, Renderer>
    for Overlay<'a, 'b, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let space_below =
            bounds.height - (self.position.y + self.target_height);
        let space_above = self.position.y;

        let limits = layout::Limits::new(
            Size::ZERO,
            Size::new(
                bounds.width - self.position.x,
                if space_below > space_above {
                    space_below
                } else {
                    space_above
                },
            ),
        )
        .width(self.width);

        let node = self.list.layout(self.state, renderer, &limits);
        let size = node.size();

        node.move_to(if space_below > space_above {
            self.position + Vector::new(0.0, self.target_height)
        } else {
            self.position - Vector::new(0.0, size.height)
        })
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        let bounds = layout.bounds();

        self.list.on_event(
            self.state, event, layout, cursor, renderer, clipboard, shell,
            &bounds,
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.list
            .mouse_interaction(self.state, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        let style = Catalog::style(theme, self.class);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                ..renderer::Quad::default()
            },
            style.background,
        );

        self.list.draw(
            self.state, renderer, theme, defaults, layout, cursor, &bounds,
        );
    }
}

struct List<'a, 'b, T, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    options: &'a [T],
    disabled: Option<Vec<bool>>,
    hovered_option: &'a mut Option<usize>,
    on_selected: Box<dyn FnMut(T) -> Message + 'a>,
    on_option_hovered: Option<&'a dyn Fn(T) -> Message>,
    padding: Padding,
    text_size: Option<Pixels>,
    text_line_height: text::LineHeight,
    text_shaping: text::Shaping,
    font: Option<Renderer::Font>,
    class: &'a <Theme as Catalog>::Class<'b>,
}

impl<'a, 'b, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for List<'a, 'b, T, Message, Theme, Renderer>
where
    T: Clone + ToString,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        use std::f32;

        let text_size =
            self.text_size.unwrap_or_else(|| renderer.default_size());

        let text_line_height = self.text_line_height.to_absolute(text_size);

        let size = {
            let intrinsic = Size::new(
                0.0,
                (f32::from(text_line_height) + self.padding.vertical())
                    * self.options.len() as f32,
            );

            limits.resolve(Length::Fill, Length::Shrink, intrinsic)
        };

        layout::Node::new(size)
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(cursor_position) =
                    cursor.position_in(layout.bounds())
                {
                    if let Some(clicked_index) =
                        self.option_index_at(cursor_position, renderer)
                    {
                        if !self.is_disabled(clicked_index) {
                            if let Some(option) =
                                self.options.get(clicked_index)
                            {
                                shell.publish((self.on_selected)(
                                    option.clone(),
                                ));
                            }
                        }
                        return event::Status::Captured;
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(cursor_position) =
                    cursor.position_in(layout.bounds())
                {
                    if let Some(new_hovered_option) =
                        self.option_index_at(cursor_position, renderer)
                    {
                        if !self.is_disabled(new_hovered_option) {
                            if let Some(on_option_hovered) =
                                self.on_option_hovered
                            {
                                if *self.hovered_option
                                    != Some(new_hovered_option)
                                {
                                    if let Some(option) =
                                        self.options.get(new_hovered_option)
                                    {
                                        shell.publish(on_option_hovered(
                                            option.clone(),
                                        ));
                                    }
                                }
                            }
                            *self.hovered_option = Some(new_hovered_option);
                        }
                        return event::Status::Captured;
                    }
                }
            }
            Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor_position) =
                    cursor.position_in(layout.bounds())
                {
                    if let Some(new_hovered_option) =
                        self.option_index_at(cursor_position, renderer)
                    {
                        if !self.is_disabled(new_hovered_option) {
                            *self.hovered_option = Some(new_hovered_option);
                            if let Some(option) =
                                self.options.get(new_hovered_option)
                            {
                                shell.publish((self.on_selected)(
                                    option.clone(),
                                ));
                            }
                        }
                        return event::Status::Captured;
                    }
                }
            }
            _ => {}
        }

        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if let Some(cursor_position) = cursor.position_in(layout.bounds()) {
            if let Some(hovered_index) =
                self.option_index_at(cursor_position, renderer)
            {
                if !self.is_disabled(hovered_index) {
                    return mouse::Interaction::Pointer;
                }
            }
        }

        mouse::Interaction::default()
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let style = Catalog::style(theme, self.class);
        let bounds = layout.bounds();

        let text_size =
            self.text_size.unwrap_or_else(|| renderer.default_size());
        let option_height =
            f32::from(self.text_line_height.to_absolute(text_size))
                + self.padding.vertical();

        let offset = viewport.y - bounds.y;
        let start = (offset / option_height) as usize;
        let end = ((offset + viewport.height) / option_height).ceil() as usize;

        let visible_options = &self.options[start..end.min(self.options.len())];

        for (i, option) in visible_options.iter().enumerate() {
            let i = start + i;
            let is_selected = *self.hovered_option == Some(i);
            let is_disabled = self
                .disabled
                .as_ref()
                .and_then(|d| d.get(i))
                .copied()
                .unwrap_or(false);

            let bounds = Rectangle {
                x: bounds.x,
                y: bounds.y + (option_height * i as f32),
                width: bounds.width,
                height: option_height,
            };

            if is_selected && !is_disabled {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + style.border.width,
                            width: bounds.width - style.border.width * 2.0,
                            ..bounds
                        },
                        border: border::rounded(style.border.radius),
                        ..renderer::Quad::default()
                    },
                    style.selected_background,
                );
            } else if is_disabled {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + style.border.width,
                            width: bounds.width - style.border.width * 2.0,
                            ..bounds
                        },
                        border: border::rounded(style.border.radius),
                        ..renderer::Quad::default()
                    },
                    style.disabled_background,
                );
            }

            renderer.fill_text(
                Text {
                    content: option.to_string(),
                    bounds: Size::new(f32::INFINITY, bounds.height),
                    size: text_size,
                    line_height: self.text_line_height,
                    font: self.font.unwrap_or_else(|| renderer.default_font()),
                    horizontal_alignment: alignment::Horizontal::Left,
                    vertical_alignment: alignment::Vertical::Center,
                    shaping: self.text_shaping,
                    wrapping: text::Wrapping::default(),
                },
                Point::new(bounds.x + self.padding.left, bounds.center_y()),
                if is_disabled {
                    style.disabled_text_color
                } else if is_selected {
                    style.selected_text_color
                } else {
                    style.text_color
                },
                *viewport,
            );
        }
    }
}

impl<'a, 'b, T, Message, Theme, Renderer>
    From<List<'a, 'b, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: ToString + Clone,
    Message: 'a,
    Theme: 'a + Catalog,
    Renderer: 'a + text::Renderer,
    'b: 'a,
{
    fn from(list: List<'a, 'b, T, Message, Theme, Renderer>) -> Self {
        Element::new(list)
    }
}

/// The appearance of a [`Menu`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the menu.
    pub background: Background,
    /// The [`Border`] of the menu.
    pub border: Border,
    /// The text [`Color`] of the menu.
    pub text_color: Color,
    /// The text [`Color`] of a selected option in the menu.
    pub selected_text_color: Color,
    /// The background [`Color`] of a selected option in the menu.
    pub selected_background: Background,
    /// The text [`Color`] of a disabled option in the menu.
    pub disabled_text_color: Color,
    /// The background [`Color`] of a disabled option in the menu.
    pub disabled_background: Background,
}

/// The theme catalog of a [`Menu`].
pub trait Catalog: scrollable::Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> <Self as Catalog>::Class<'a>;

    /// The default class for the scrollable of the [`Menu`].
    fn default_scrollable<'a>() -> <Self as scrollable::Catalog>::Class<'a> {
        <Self as scrollable::Catalog>::default()
    }

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &<Self as Catalog>::Class<'_>) -> Style;
}

/// A styling function for a [`Menu`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default)
    }

    fn style(&self, class: &StyleFn<'_, Self>) -> Style {
        class(self)
    }
}

/// The default style of the list of a [`Menu`].
pub fn default(theme: &Theme) -> Style {
    let palette = theme.extended_palette();

    Style {
        background: palette.background.weak.color.into(),
        border: Border {
            width: 1.0,
            radius: 0.0.into(),
            color: palette.background.strong.color,
        },
        text_color: palette.background.weak.text,
        selected_text_color: palette.primary.strong.text,
        selected_background: palette.primary.strong.color.into(),
        disabled_text_color: palette.background.weak.text.scale_alpha(0.5),
        disabled_background: palette
            .background
            .weak
            .color
            .scale_alpha(0.5)
            .into(),
    }
}
