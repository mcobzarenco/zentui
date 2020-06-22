use palette::{Gradient, Hsv, LinSrgb};
use std::rc::Rc;
use zi::{
    components::text::{Text, TextAlign, TextProperties},
    layout,
    terminal::Style,
    Colour, Component, ComponentLink, Layout, Rect, ShouldRender,
};

use super::Base16Theme;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub pending: Style,
    pub ready: Style,
    pub text: Style,
}

impl From<&Base16Theme> for Theme {
    fn from(theme: &Base16Theme) -> Self {
        Self {
            pending: Style::bold(theme.base0e, theme.base00),
            ready: Style::bold(theme.base0e, theme.base00),
            text: Style::bold(theme.base00, theme.base04),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct PromptProperties {
    pub theme: Rc<Theme>,
    pub pending: bool,
}

pub struct Prompt {
    properties: PromptProperties,
}

impl Component for Prompt {
    type Message = ();
    type Properties = PromptProperties;

    fn create(properties: Self::Properties, _frame: Rect, _link: ComponentLink<Self>) -> Self {
        Self { properties }
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        if self.properties != properties {
            self.properties = properties;
            ShouldRender::Yes
        } else {
            ShouldRender::No
        }
    }

    fn update(&mut self, _message: Self::Message) -> ShouldRender {
        ShouldRender::Yes
    }

    fn view(&self) -> Layout {
        layout::row([
            layout::fixed(
                1,
                layout::component_with_key::<Status>(
                    0,
                    StatusProperties {
                        style: if self.properties.pending {
                            self.properties.theme.pending
                        } else {
                            self.properties.theme.ready
                        },
                        pending: self.properties.pending,
                    },
                ),
            ),
            layout::auto(layout::component_with_key::<Text>(
                1,
                TextProperties::new()
                    .content("")
                    .style(self.properties.theme.text)
                    .align(TextAlign::Left),
            )),
        ])
    }
}

#[derive(Clone, PartialEq)]
pub struct StatusProperties {
    pub pending: bool,
    pub style: Style,
}

pub struct Status {
    properties: StatusProperties,
    animation_offset: f32,
    progress_index: usize,
    gradient: Gradient<Hsv>,
}

impl Component for Status {
    type Message = ();
    type Properties = StatusProperties;

    fn create(properties: Self::Properties, _frame: Rect, _link: ComponentLink<Self>) -> Self {
        Self {
            gradient: gradient_from_style(properties.style),
            properties,
            animation_offset: 1.0,
            progress_index: 0,
        }
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        if self.properties != properties {
            self.gradient = gradient_from_style(properties.style);
            if self.properties.pending != properties.pending {
                self.animation_offset = 1.0;
            }
            self.properties = properties;
            ShouldRender::Yes
        } else {
            ShouldRender::No
        }
    }

    fn update(&mut self, _message: Self::Message) -> ShouldRender {
        // `animation_offset` ticks in the interval [0, 2]:
        self.animation_offset = (self.animation_offset + 2.0 / PROGRESS_PATTERN.len() as f32) % 2.0;
        self.progress_index = (self.progress_index + 1) % PROGRESS_PATTERN.len();
        ShouldRender::Yes
    }

    fn view(&self) -> Layout {
        let Self {
            properties: StatusProperties { style, pending },
            ..
        } = *self;

        let content = if pending {
            PROGRESS_PATTERN[self.progress_index]
        } else {
            '✓'
        }
        .to_string();
        layout::component::<Text>(TextProperties::new().content(content).style(if pending {
            self.animated_style()
        } else {
            style
        }))
    }

    fn tick(&self) -> Option<Self::Message> {
        if self.properties.pending {
            Some(())
        } else {
            None
        }
    }
}

fn gradient_from_style(style: Style) -> Gradient<Hsv> {
    Gradient::new(vec![
        Hsv::from(
            LinSrgb::new(
                style.background.red,
                style.background.green,
                style.background.blue,
            )
            .into_format::<f32>(),
        ),
        Hsv::from(
            LinSrgb::new(
                style.foreground.red,
                style.foreground.green,
                style.foreground.blue,
            )
            .into_format::<f32>(),
        ),
    ])
}

impl Status {
    fn animated_style(&self) -> Style {
        let background = LinSrgb::from(self.gradient.get((self.animation_offset - 1.0).abs()))
            .into_format::<u8>();
        let foreground =
            LinSrgb::from(self.gradient.get(1.0 - (self.animation_offset - 1.0).abs()))
                .into_format::<u8>();

        Style::normal(
            Colour {
                red: background.red,
                green: background.green,
                blue: background.blue,
            },
            Colour {
                red: foreground.red,
                green: foreground.green,
                blue: foreground.blue,
            },
        )
    }
}

// const PROGRESS_PATTERN: [char; 16] = [
//     '⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷', '⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈',
// ];
const PROGRESS_PATTERN: [char; 13] = [
    '▉', '▊', '▋', '▌', '▍', '▎', '▏', '▎', '▍', '▌', '▋', '▊', '▉',
];
// const PROGRESS_PATTERN: [char; 8] = ['▙', '▛', '▜', '▟', '▘', '▝', '▖', '▗'];
// const PROGRESS_PATTERN: [char; 6] = ['◜', '◠', '◝', '◞', '◡', '◟'];
// const PROGRESS_PATTERN: [char; 4] = ['■', '□', '▪', '▫'];
// const PROGRESS_PATTERN: [char; 8] = ['▘', '▀', '▝', '▐', '▗', '▄', '▖', '▌'];
// const PROGRESS_PATTERN: [char; 29] = [
//     '⠁', '⠁', '⠉', '⠙', '⠚', '⠒', '⠂', '⠂', '⠒', '⠲', '⠴', '⠤', '⠄', '⠄', '⠤', '⠠', '⠠', '⠤', '⠦',
//     '⠖', '⠒', '⠐', '⠐', '⠒', '⠓', '⠋', '⠉', '⠈', '⠈',
// ];
