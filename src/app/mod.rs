mod issue_card;
mod pipeline;
mod prompt;

use anyhow::Result;
use futures::future::FutureExt;
use im::hashmap::HashMap;
use std::{cmp, iter, rc::Rc, sync::Arc};
use tokio::runtime::Handle as RuntimeHandle;
use zi::{
    components::text::{Text, TextProperties},
    layout, BindingMatch, BindingTransition, Colour, Component, ComponentLink, Key, Layout, Rect,
    ShouldRender, Style,
};

use crate::{
    edit,
    github::{Client as GithubClient, Issue, IssueNumber, Repo},
    zenhub::{Board, Client as ZenhubClient, Pipeline},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    divider: Style,
    prompt: Rc<prompt::Theme>,
    pipeline_focused: Rc<pipeline::Theme>,
    pipeline_unfocused: Rc<pipeline::Theme>,
}

impl From<&Base16Theme> for Theme {
    fn from(theme: &Base16Theme) -> Self {
        Self {
            divider: Style::bold(theme.base0f, theme.base0f),
            prompt: Rc::new(theme.into()),
            pipeline_unfocused: Rc::new(theme.into()),
            pipeline_focused: Rc::new(pipeline::Theme {
                title: Style::bold(theme.base00, theme.base0d),
                subtitle: Style::normal(theme.base00, theme.base04),
                issue: Rc::new(issue_card::Theme {
                    number: Style::normal(theme.base00, theme.base06),
                    text: Style::normal(theme.base00, theme.base05),
                    border: Style::normal(theme.base00, theme.base02),
                }),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FutureValue<T> {
    Pending,
    Ready(T),
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipelineView {
    pub pipeline: Pipeline,
    pub hidden: bool,
    pub selected_issue: IssueIndex,
}

impl PipelineView {
    fn select_issue(&mut self, issue_index: usize) {
        self.selected_issue = cmp::min(issue_index, self.pipeline.issues.len().saturating_sub(1));
    }
}

impl From<Pipeline> for PipelineView {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            pipeline,
            hidden: false,
            selected_issue: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardView {
    pub pipelines: Vec<PipelineView>,
    pub selected_pipeline: PipelineIndex,
}

impl BoardView {
    fn select_next_pipeline(&mut self) {
        let mut selected_pipeline = self.selected_pipeline;
        let max_selected_pipline = self.pipelines.len().saturating_sub(1);
        loop {
            selected_pipeline = cmp::min(selected_pipeline + 1, max_selected_pipline);
            if !self.pipelines[selected_pipeline].hidden {
                self.selected_pipeline = selected_pipeline;
                break;
            } else if selected_pipeline == max_selected_pipline {
                break;
            }
        }
    }

    fn select_previous_pipeline(&mut self) {
        let mut selected_pipeline = self.selected_pipeline;
        loop {
            selected_pipeline = selected_pipeline.saturating_sub(1);
            if !self.pipelines[selected_pipeline].hidden {
                self.selected_pipeline = selected_pipeline;
                break;
            } else if selected_pipeline == 0 {
                break;
            }
        }
    }

    fn hide_pipeline(&mut self, pipeline_index: PipelineIndex) {
        if let Some(pipeline) = self.pipelines.get_mut(pipeline_index) {
            pipeline.hidden = true;
            if self.pipelines[self.selected_pipeline].hidden {
                self.select_previous_pipeline();
            }
            if self.pipelines[self.selected_pipeline].hidden {
                self.select_next_pipeline();
            }
            if self.pipelines[self.selected_pipeline].hidden {
                self.selected_pipeline = 0;
            }
        }
    }

    fn show_all_pipelines(&mut self) {
        self.pipelines
            .iter_mut()
            .for_each(|pipeline| pipeline.hidden = false);
    }

    fn selected_pipeline(&self) -> Option<&PipelineView> {
        self.pipelines.get(self.selected_pipeline)
    }

    fn selected_pipeline_mut(&mut self) -> Option<&mut PipelineView> {
        self.pipelines.get_mut(self.selected_pipeline)
    }
}

impl From<Board> for BoardView {
    fn from(board: Board) -> Self {
        Self {
            pipelines: board.pipelines.into_iter().map(Into::into).collect(),
            selected_pipeline: 0,
        }
    }
}

#[derive(Clone)]
pub struct Properties {
    pub async_runtime: RuntimeHandle,
    pub github_client: Arc<GithubClient>,
    pub zenhub_client: Arc<ZenhubClient>,
    pub repo: Repo,
}

type PipelineIndex = usize;
type IssueIndex = usize;

pub struct App {
    properties: Properties,
    link: ComponentLink<Self>,
    theme: Rc<Theme>,
    board: BoardView,
    issues: HashMap<IssueNumber, FutureValue<Issue>>,
    num_pending_tasks: usize,
}

#[derive(Debug)]
pub enum Message {
    NextPipeline,
    PreviousPipeline,
    SelectIssue(usize),
    LoadedIssue(IssueNumber, Result<Issue>),
    EditIssue(IssueNumber, Result<Issue>),
    LoadedBoard(Result<Board>),
    HidePipeline(usize),
    ShowAllPipelines,
}

impl Component for App {
    type Message = Message;
    type Properties = Properties;

    fn create(properties: Self::Properties, _frame: Rect, link: ComponentLink<Self>) -> Self {
        {
            let link = link.clone();
            let zenhub_client = properties.zenhub_client.clone();
            properties
                .async_runtime
                .spawn(
                    zenhub_client
                        .get_oldest_board(properties.repo.id)
                        .map(move |board| {
                            link.send(Message::LoadedBoard(board));
                        }),
                );
        }

        Self {
            properties,
            link,
            theme: Rc::new((&ICY).into()),
            board: BoardView::default(),
            issues: HashMap::new(),
            num_pending_tasks: 1,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        match message {
            Message::NextPipeline => self.board.select_next_pipeline(),
            Message::PreviousPipeline => self.board.select_previous_pipeline(),
            Message::SelectIssue(issue_index) => {
                eprintln!("msg: {:?}", message);
                if let Some(pipeline) = self.board.selected_pipeline_mut() {
                    pipeline.select_issue(issue_index);
                }
            }
            Message::LoadedBoard(new_board) => {
                let Self {
                    ref mut num_pending_tasks,
                    ref mut board,
                    ref properties,
                    ref link,
                    ..
                } = *self;
                *num_pending_tasks -= 1;
                *board = new_board.unwrap().into();
                let repo = Arc::new(properties.repo.full_name.clone());
                for pipeline in board.pipelines.iter() {
                    pipeline
                        .pipeline
                        .issues
                        .iter()
                        .take(7)
                        .cloned()
                        .for_each(|issue_ref| {
                            *num_pending_tasks += 1;
                            let link = link.clone();
                            let github_client = properties.github_client.clone();
                            let repo = repo.clone();
                            properties.async_runtime.spawn(
                                github_client
                                    .get_issue(repo, issue_ref.number)
                                    .map(move |issue| {
                                        link.send(Message::LoadedIssue(issue_ref.number, issue));
                                    }),
                            );
                        })
                }
            }
            Message::LoadedIssue(issue_number, result) => {
                let issue = match result {
                    Ok(issue) => FutureValue::Ready(issue),
                    Err(error) => {
                        log::error!("{:?}", error);
                        FutureValue::Error(format!("{:?}", error))
                    }
                };
                self.issues.insert(issue_number, issue);
                self.num_pending_tasks -= 1;
            }
            Message::EditIssue(issue_number, result) => {
                let issue = match result {
                    Ok(issue) => FutureValue::Ready(issue),
                    Err(error) => {
                        log::error!("{:?}", error);
                        FutureValue::Error(format!("{:?}", error))
                    }
                };
                self.issues.insert(issue_number, issue);
            }
            Message::HidePipeline(pipeline_index) => self.board.hide_pipeline(pipeline_index),
            Message::ShowAllPipelines => self.board.show_all_pipelines(),
        }
        ShouldRender::Yes
    }

    fn view(&self) -> Layout {
        let separator = |pipeline_index| {
            iter::once(layout::fixed(
                1,
                layout::component_with_key::<Text>(
                    1000 * pipeline_index + 1,
                    TextProperties::new().style(self.theme.divider),
                ),
            ))
        };

        layout::column([
            layout::auto(layout::row_reverse_iter(
                self.board
                    .pipelines
                    .iter()
                    .enumerate()
                    .rev()
                    .filter(|(_, pipeline)| !pipeline.hidden)
                    .flat_map(|(pipeline_index, pipeline)| {
                        let focused = pipeline_index == self.board.selected_pipeline;
                        separator(pipeline_index + 1).chain(iter::once(layout::auto(
                            layout::component_with_key::<pipeline::Pipeline>(
                                1000 * pipeline_index,
                                pipeline::Properties {
                                    theme: if focused {
                                        self.theme.pipeline_focused.clone()
                                    } else {
                                        self.theme.pipeline_unfocused.clone()
                                    },
                                    pipeline_view: pipeline.clone(),
                                    issues: self.issues.clone(),
                                    focused,
                                    on_selected_change: self.link.callback(Message::SelectIssue),
                                },
                            ),
                        )))
                    })
                    .skip(1),
            )),
            layout::fixed(
                1,
                layout::component_with_key::<prompt::Prompt>(
                    1,
                    prompt::PromptProperties {
                        theme: self.theme.prompt.clone(),
                        pending: self.num_pending_tasks > 0,
                    },
                ),
            ),
        ])
    }

    fn has_focus(&self) -> bool {
        true
    }

    fn input_binding(&self, pressed: &[Key]) -> BindingMatch<Self::Message> {
        let mut transition = BindingTransition::Clear;
        let message = match pressed {
            &[Key::Ctrl('f')] | &[Key::Right] | &[Key::Char('l')] => Some(Message::NextPipeline),
            &[Key::Ctrl('b')] | &[Key::Left] | &[Key::Char('h')] => Some(Message::PreviousPipeline),
            &[Key::Char('\n')] => {
                if let Some(FutureValue::Ready(issue)) = self
                    .board
                    .selected_pipeline()
                    .and_then(|pipeline| {
                        self.issues
                            .get(&pipeline.pipeline.issues[pipeline.selected_issue].number)
                    })
                    .cloned()
                {
                    self.link.run_exclusive(move || {
                        let edit_result = edit::edit(&format!("{}\n\n{}", issue.title, issue.body))
                            .map(|new_title| {
                                let mut issue = issue.clone();
                                issue.title = new_title;
                                issue
                            })
                            .map_err(anyhow::Error::from);
                        Some(Message::EditIssue(issue.number, edit_result))
                    });
                }
                None
            }
            &[Key::Ctrl('h')] => Some(Message::HidePipeline(self.board.selected_pipeline)),
            &[Key::Ctrl('x'), Key::Ctrl('h')] => Some(Message::ShowAllPipelines),
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

/// Represents a base16 theme.
///
/// Colours base00 to base07 are typically variations of a shade and run from
/// darkest to lightest. These colours are used for foreground and background,
/// status bars, line highlighting and such. Colours base08 to base0F are
/// typically individual colours used for types, operators, names and variables.
/// In order to create a dark theme, colours base00 to base07 should span from
/// dark to light. For a light theme, these colours should span from light to
/// dark.
pub struct Base16Theme {
    pub base00: Colour, // Default Background
    pub base01: Colour, // Lighter Background (Used for status bars)
    pub base02: Colour, // Selection Background
    pub base03: Colour, // Comments, Invisibles, Line Highlighting
    pub base04: Colour, // Dark Foreground (Used for status bars)
    pub base05: Colour, // Default Foreground, Caret, Delimiters, Operators
    pub base06: Colour, // Light Foreground (Not often used)
    pub base07: Colour, // Light Background (Not often used)
    pub base08: Colour, // Variables, XML Tags, Markup Link Text, Markup Lists, Diff Deleted
    pub base09: Colour, // Integers, Boolean, Constants, XML Attributes, Markup Link Url
    pub base0a: Colour, // Classes, Markup Bold, Search Text Background
    pub base0b: Colour, // Strings, Inherited Class, Markup Code, Diff Inserted
    pub base0c: Colour, // Support, Regular Expressions, Escape Characters, Markup Quotes
    pub base0d: Colour, // Functions, Methods, Attribute IDs, Headings
    pub base0e: Colour, // Keywords, Storage, Selector, Markup Italic, Diff Changed
    pub base0f: Colour, // Deprecated, Opening/Closing Embedded Language Tags, e.g. <?php ?>
}

pub const ICY: Base16Theme = Base16Theme {
    base00: Colour::rgb(2, 16, 18),
    base01: Colour::rgb(3, 22, 25),
    base02: Colour::rgb(4, 31, 35),
    base03: Colour::rgb(5, 46, 52),
    base04: Colour::rgb(6, 64, 72),
    base05: Colour::rgb(9, 91, 103),
    base06: Colour::rgb(12, 124, 140),
    base07: Colour::rgb(16, 156, 176),
    base08: Colour::rgb(22, 193, 217),
    base09: Colour::rgb(179, 235, 242),
    base0a: Colour::rgb(128, 222, 234),
    base0b: Colour::rgb(77, 208, 225),
    base0c: Colour::rgb(38, 198, 218),
    base0d: Colour::rgb(0, 188, 212),
    base0e: Colour::rgb(0, 172, 193),
    base0f: Colour::rgb(1, 9, 12),
};
