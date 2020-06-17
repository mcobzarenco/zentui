mod pipeline;

use std::iter;
use zi::{
    component::{
        border::{Border, BorderProperties},
        text::{Text, TextAlign, TextProperties},
    },
    frontend, layout, BindingMatch, BindingTransition, Canvas, Colour, Component, ComponentLink,
    Key, Layout, Rect, Result, ShouldRender, Size, Style,
};

use crate::zenhub::Board;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pipeline_title: Style,
    issue_number: Style,
    issue_text: Style,
}

impl Default for Theme {
    fn default() -> Self {
        const DARK0_SOFT: Colour = Colour::rgb(50, 48, 47);
        const LIGHT2: Colour = Colour::rgb(213, 196, 161);
        const GRAY_245: Colour = Colour::rgb(146, 131, 116);
        const BRIGHT_BLUE: Colour = Colour::rgb(131, 165, 152);

        Self {
            pipeline_title: Style::bold(DARK0_SOFT, BRIGHT_BLUE),
            issue_number: Style::normal(DARK0_SOFT, GRAY_245),
            issue_text: Style::normal(DARK0_SOFT, LIGHT2),
        }
    }
}

#[derive(Clone)]
pub struct Properties {
    pub theme: Theme,
    pub board: Board,
}

pub struct App {
    properties: Properties,
    link: ComponentLink<Self>,
}

impl Component for App {
    type Message = usize;
    type Properties = Properties;

    fn create(properties: Self::Properties, _frame: Rect, link: ComponentLink<Self>) -> Self {
        Self { properties, link }
    }

    fn view(&self) -> Layout {
        layout::row_iter(self.properties.board.pipelines.iter().enumerate().map(
            |(pipeline_index, pipeline)| {
                layout::auto(layout::component_with_key::<pipeline::Pipeline>(
                    1000 * pipeline_index,
                    pipeline::Properties {
                        theme: pipeline::Theme::default(),
                        pipeline: pipeline.clone(),
                    },
                ))
            },
        ))
    }

    fn has_focus(&self) -> bool {
        true
    }

    fn input_binding(&self, pressed: &[Key]) -> BindingMatch<Self::Message> {
        let mut transition = BindingTransition::Clear;
        let message = match pressed {
            &[Key::Ctrl('x'), Key::Ctrl('c')] => {
                self.link.exit();
                None
            }
            &[Key::Ctrl('x')] => {
                transition = BindingTransition::Continue;
                None
            }
            _ => None,
        };
        BindingMatch {
            transition,
            message,
        }
    }
}
