use im::hashmap::HashMap;
use std::rc::Rc;
use zi::{
    components::{
        select::{Select, SelectProperties},
        text::{Text, TextAlign, TextProperties},
    },
    layout, Callback, Component, ComponentLink, Layout, Rect, ShouldRender, Style,
};

use super::{
    issue_card::{self, IssueCard},
    Base16Theme, FutureValue, PipelineView,
};
use crate::github::{Issue, IssueNumber};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub title: Style,
    pub subtitle: Style,
    pub issue: Rc<issue_card::Theme>,
}

impl From<&Base16Theme> for Theme {
    fn from(theme: &Base16Theme) -> Self {
        Self {
            title: Style::bold(theme.base0f, theme.base0d),
            subtitle: Style::normal(theme.base0f, theme.base04),
            issue: Rc::new(theme.into()),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Properties {
    pub theme: Rc<Theme>,
    pub pipeline_view: PipelineView,
    pub issues: HashMap<IssueNumber, FutureValue<Issue>>,
    pub focused: bool,
    pub on_selected_change: Callback<usize>,
}

pub struct Pipeline {
    properties: Properties,
    link: ComponentLink<Self>,
}

pub enum Message {}

impl Component for Pipeline {
    type Message = Message;
    type Properties = Properties;

    fn create(properties: Self::Properties, _frame: Rect, link: ComponentLink<Self>) -> Self {
        Self { properties, link }
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
        let Self {
            properties:
                Properties {
                    ref pipeline_view,
                    ref theme,
                    ref issues,
                    ref on_selected_change,
                    focused,
                    ..
                },
            ..
        } = *self;

        let pipeline_issues = pipeline_view.pipeline.issues.clone();
        let issues = issues.clone();
        let theme = theme.clone();
        let selected_issue = pipeline_view.selected_issue;
        let subtitle = if pipeline_issues.is_empty() {
            "(empty)".into()
        } else {
            format!("({} issues)", pipeline_issues.len())
        };
        layout::column([
            layout::fixed(
                1,
                layout::component_with_key::<Text>(
                    0,
                    TextProperties::new()
                        .content(pipeline_view.pipeline.name.clone())
                        .style(theme.title)
                        .align(TextAlign::Centre),
                ),
            ),
            layout::fixed(
                2,
                layout::component_with_key::<Text>(
                    1,
                    TextProperties::new()
                        .content(subtitle)
                        .style(theme.subtitle)
                        .align(TextAlign::Centre),
                ),
            ),
            layout::auto(layout::component_with_key::<Select>(
                2,
                SelectProperties {
                    background: theme.title,
                    direction: layout::FlexDirection::Column,
                    focused,
                    num_items: pipeline_issues.len(),
                    item_at: (move |index: usize| {
                        let issue_number = pipeline_issues[index].number;
                        let issue = issues.get(&issue_number).cloned();
                        layout::fixed(
                            10,
                            layout::component_with_key::<IssueCard>(
                                10000 + pipeline_issues[index].number.0,
                                issue_card::Properties {
                                    theme: theme.issue.clone(),
                                    issue_number,
                                    issue: issue.unwrap_or(FutureValue::Pending),
                                    focused: focused && index == selected_issue,
                                },
                            ),
                        )
                    })
                    .into(),
                    item_size: 10,
                    selected: selected_issue,
                    on_change: Some(on_selected_change.clone()),
                },
            )),
        ])
    }
}
