use std::rc::Rc;
use unicode_width::UnicodeWidthStr;
use zi::{
    components::{
        border::{Border, BorderProperties},
        text::{Text, TextProperties, TextWrap},
    },
    layout, Canvas, Colour, Component, ComponentLink, Layout, Position, Rect, ShouldRender, Size,
    Style,
};

use super::{Base16Theme, FutureValue};
use crate::github::{Issue, IssueNumber};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub number: Style,
    pub text: Style,
    pub border: Style,
}

impl From<&Base16Theme> for Theme {
    fn from(theme: &Base16Theme) -> Self {
        Self {
            number: Style::normal(theme.base0f, theme.base06),
            text: Style::normal(theme.base0f, theme.base05),
            border: Style::normal(theme.base0f, theme.base02),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Properties {
    pub theme: Rc<Theme>,
    pub issue_number: IssueNumber,
    pub issue: FutureValue<Issue>,
    pub focused: bool,
}

pub struct IssueCard {
    properties: Properties,
}

pub enum Message {}

impl Component for IssueCard {
    type Message = Message;
    type Properties = Properties;

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

    fn view(&self) -> Layout {
        let Self {
            properties:
                Properties {
                    ref theme,
                    ref issue,
                    focused,
                    issue_number,
                },
            ..
        } = *self;

        let (title, content) = match issue {
            FutureValue::Pending => (
                format!(" #{} ", issue_number.0),
                layout::component_with_key_str::<Text>(
                    "issue-loading",
                    TextProperties::new()
                        .content("Loading issue...")
                        .style(theme.number),
                ),
            ),
            FutureValue::Ready(issue) => (
                if issue.pull_request.is_some() {
                    format!(" #{} ⎇  ", issue_number.0)
                } else {
                    format!(" #{} ", issue_number.0)
                },
                layout::component_with_key_str::<IssueContent>(
                    "issue-content",
                    IssueContentProperties {
                        theme: theme.clone(),
                        issue: issue.clone(),
                    },
                ),
            ),
            FutureValue::Error(message) => (
                format!(" #{} ", issue_number.0),
                layout::component_with_key_str::<Text>(
                    "issue-error",
                    TextProperties::new()
                        .content(message.as_str())
                        .style(theme.number),
                ),
            ),
        };

        layout::component::<Border>(
            BorderProperties::new(content)
                .style(if focused { theme.text } else { theme.border })
                .title(Some((title, theme.text))),
        )
    }
}

#[derive(Clone, PartialEq)]
pub struct IssueContentProperties {
    pub theme: Rc<Theme>,
    pub issue: Issue,
}

pub struct IssueContent {
    properties: IssueContentProperties,
    frame: Rect,
}

impl Component for IssueContent {
    type Message = ();
    type Properties = IssueContentProperties;

    fn create(properties: Self::Properties, frame: Rect, _link: ComponentLink<Self>) -> Self {
        Self { properties, frame }
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        if self.properties != properties {
            self.properties = properties;
            ShouldRender::Yes
        } else {
            ShouldRender::No
        }
    }

    fn resize(&mut self, frame: Rect) -> ShouldRender {
        self.frame = frame;
        ShouldRender::Yes
    }

    fn view(&self) -> Layout {
        let Self {
            properties:
                IssueContentProperties {
                    ref theme,
                    ref issue,
                },
            frame,
            ..
        } = *self;

        let issue_text = layout::auto(layout::component_with_key_str::<Text>(
            "issue-text",
            TextProperties::new()
                .content(issue.title.clone())
                .style(theme.number)
                .wrap(TextWrap::Word),
        ));

        let mut position = Position::zero();
        let mut label_canvas = Canvas::new(frame.size);
        label_canvas.clear(theme.text);

        for label in issue.labels.iter() {
            let label_name = format!(" {} ", label.name);
            let label_style = Style::bold(
                label.color,
                if is_light_colour(&label.color) {
                    Colour::black()
                } else {
                    Colour::white()
                },
            );
            // let label_width = UnicodeWidthStr::width(label.name.as_str());
            let label_width = UnicodeWidthStr::width(label_name.as_str());
            if position.x > 0 {
                if position.x >= frame.size.width
                    || label_width > frame.size.width.saturating_sub(position.x + 1)
                {
                    position.y += 1;
                    position.x = 0
                } else {
                    label_canvas.draw_str(position.x, position.y, theme.text, " ");
                    position.x += 1;
                }
            }
            label_canvas.draw_str(position.x, position.y, label_style, &label_name);
            position.x += label_width;
        }
        // log::info!("min height: {}", label_canvas.min_size().height);
        label_canvas.resize(Size::new(
            frame.size.width,
            label_canvas.min_size().height + 1,
        ));

        layout::column([
            issue_text,
            layout::fixed(label_canvas.min_size().height + 1, label_canvas.into()),
        ])
    }
}

fn is_light_colour(colour: &Colour) -> bool {
    (colour.red as f32 * 0.299 + colour.green as f32 * 0.587 + colour.blue as f32 * 0.114) > 146.0
}
