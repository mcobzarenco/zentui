use std::iter;
use zi::{
    component::{
        border::{Border, BorderProperties},
        text::{Text, TextAlign, TextProperties},
    },
    frontend, layout, BindingMatch, BindingTransition, Canvas, Colour, Component, ComponentLink,
    Key, Layout, Rect, Result, ShouldRender, Size, Style,
};

use crate::zenhub::{self, Board};

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
    pub pipeline: zenhub::Pipeline,
}

pub struct Pipeline {
    properties: Properties,
    link: ComponentLink<Self>,
}

impl Component for Pipeline {
    type Message = usize;
    type Properties = Properties;

    fn create(properties: Self::Properties, _frame: Rect, link: ComponentLink<Self>) -> Self {
        Self { properties, link }
    }

    fn view(&self) -> Layout {
        layout::row_iter(
            iter::once(layout::auto(layout::component_with_key::<Text>(
                0,
                TextProperties {
                    style: self.properties.theme.pipeline_title,
                    content: self.properties.pipeline.name.clone(),
                    align: TextAlign::Centre,
                },
            )))
            .chain(
                self.properties
                    .pipeline
                    .issues
                    .iter()
                    .enumerate()
                    .take(10)
                    .map(|(issue_index, issue)| {
                        layout::auto(layout::component_with_key::<Border>(
                            issue_index + 1,
                            BorderProperties {
                                style: self.properties.theme.issue_number,
                                component: layout::component::<Text>(TextProperties {
                                    style: self.properties.theme.issue_number,
                                    content: format!("#{}", issue.number.0),
                                    align: TextAlign::Left,
                                }),
                            },
                        ))
                    }),
            ),
        )

        // layout::row_iter(self.properties.board.pipelines.iter().enumerate().map(
        //     |(pipeline_index, pipeline)| {
        //         layout::auto(layout::component_with_key::<Border>(
        //             1000 * pipeline_index,
        //             BorderProperties {
        //                 style: self.properties.theme.issue_number,
        //                 component: layout::column_iter(
        //                     iter::once(layout::auto(layout::component_with_key::<Text>(
        //                         1000 * pipeline_index + 1,
        //                         TextProperties {
        //                             style: self.properties.theme.pipeline_title,
        //                             content: pipeline.name.clone(),
        //                             align: TextAlign::Centre,
        //                         },
        //                     )))
        //                     .chain(issues(pipeline_index, pipeline)),
        //                 ),
        //             },
        //         ))
        //     },
        // ))
    }
}
