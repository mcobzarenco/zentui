use anyhow::{Context, Result};
use im::Vector;
use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderValue, ACCEPT, USER_AGENT},
    Client as HttpClient, IntoUrl, Url,
};
use serde::{self, de::Deserializer, Deserialize};
use serde_derive::Deserialize;
use std::sync::Arc;

use zi::Colour;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct RepoId(pub u64);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Repo {
    pub id: RepoId,
    pub full_name: RepoFullName,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct IssueNumber(pub usize);

#[serde(rename_all = "lowercase")]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub number: IssueNumber,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub state: IssueState,
    pub labels: Vector<Label>,
    pub pull_request: Option<PullRequestRefs>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    #[serde(deserialize_with = "from_hex_colour")]
    pub color: Colour,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PullRequestRefs {}

fn from_hex_colour<'de, DeserializerT>(
    deserializer: DeserializerT,
) -> std::result::Result<Colour, DeserializerT::Error>
where
    DeserializerT: Deserializer<'de>,
{
    let hex_str: &str = Deserialize::deserialize(deserializer)?;
    let colour = u64::from_str_radix(hex_str, 16).map_err(serde::de::Error::custom)?;
    Ok(Colour {
        red: ((colour >> 16) & 0xff) as u8,
        green: ((colour >> 8) & 0xff) as u8,
        blue: (colour & 0xff) as u8,
    })
}

#[derive(Debug)]
pub struct Client {
    endpoints: Endpoints,
    http_client: HttpClient,
    authorization_token: HeaderValue,
}

impl Client {
    /// Create a new Github client.
    pub fn new(token: Token) -> Result<Client> {
        Ok(Client {
            endpoints: Endpoints::new(DEFAULT_ENDPOINT.clone())?,
            http_client: HttpClient::builder().gzip(true).build()?,
            authorization_token: HeaderValue::from_str(&format!("token {}", token.0))?,
        })
    }

    /// Create a new API client.
    pub async fn get_repo(&self, repo: &RepoFullName) -> Result<Repo> {
        self.get::<_, Repo>(self.endpoints.repo(repo)?).await
    }

    /// Get an issue.
    pub async fn get_issue(
        self: Arc<Self>,
        repo: Arc<RepoFullName>,
        issue_number: IssueNumber,
    ) -> Result<Issue> {
        self.get::<_, Issue>(self.endpoints.issue(&repo, &issue_number)?)
            .await
    }

    async fn get<LocationT, SuccessT>(&self, url: LocationT) -> Result<SuccessT>
    where
        LocationT: IntoUrl + std::fmt::Display,
        for<'de> SuccessT: Deserialize<'de>,
    {
        log::debug!("Attempting GET `{}`", url);
        self.http_client
            .get(url)
            .header(ACCEPT, ACCEPT_API_V3)
            .header(USER_AGENT, USER_AGENT_VALUE)
            .header("authorization", &self.authorization_token)
            .send()
            .await
            .with_context(|| "GET operation failed.")?
            .error_for_status()
            .with_context(|| "GET returned non-success status code.")?
            .json::<SuccessT>()
            .await
            .with_context(|| "Could not parse JSON response")
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RepoFullName(pub String);

impl std::str::FromStr for RepoFullName {
    type Err = anyhow::Error;

    fn from_str(full_name: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(full_name.into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token(pub String);

impl std::str::FromStr for Token {
    type Err = anyhow::Error;

    fn from_str(token: &str) -> std::result::Result<Self, Self::Err> {
        Ok(token.into())
    }
}

impl<T> From<T> for Token
where
    T: Into<String>,
{
    fn from(token: T) -> Self {
        Self(token.into())
    }
}

#[derive(Debug)]
struct Endpoints {
    base: Url,
}

impl Endpoints {
    pub fn new(base: Url) -> Result<Self> {
        Ok(Endpoints { base })
    }

    fn repo(&self, full_name: &RepoFullName) -> Result<Url> {
        self.base
            .join(&format!("/repos/{}", full_name.0,))
            .with_context(|| format!("Could not build URL for Github repo `{}`.", full_name.0))
    }

    fn issue(&self, repo: &RepoFullName, issue_number: &IssueNumber) -> Result<Url> {
        self.base
            .join(&format!(
                "/repos/{repo}/issues/{issue_number}",
                repo = repo.0,
                issue_number = issue_number.0,
            ))
            .with_context(|| {
                format!(
                    "Could not build URL for Github issue `{}` for repo `{}`.",
                    issue_number.0, repo.0,
                )
            })
    }
}

static DEFAULT_ENDPOINT: Lazy<Url> =
    Lazy::new(|| Url::parse("https://api.github.com").expect("Default URL is well-formed"));

const ACCEPT_API_V3: &str = "application/vnd.github.v3+json";
const USER_AGENT_VALUE: &str = "zentui/0.0.1";
