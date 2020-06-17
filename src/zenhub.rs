use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use reqwest::{
    blocking::Client as HttpClient,
    header::{HeaderMap, HeaderName, HeaderValue},
    IntoUrl, Response, Url,
};
use serde::Deserialize;
use serde_derive::Deserialize;
use std::ops::Deref;

// {
//     "estimate": {
//         "value": 8
//     },
//     "plus_ones": [
//         {
//             "created_at": "2015-12-11T18:43:22.296Z"
//         }
//     ],
//     "pipeline": {
//         "name": "QA",
//         "pipeline_id": "5d0a7a9741fd098f6b7f58a7",
//         "workspace_id": "5d0a7a9741fd098f6b7f58ac"
//     },
//     "pipelines": [
//         {
//             "name": "QA",
//             "pipeline_id": "5d0a7a9741fd098f6b7f58a7",
//             "workspace_id": "5d0a7a9741fd098f6b7f58ac"
//         },
//         {
//             "name": "Done",
//             "pipeline_id": "5d0a7cea41fd098f6b7f58b7",
//             "workspace_id": "5d0a7cea41fd098f6b7f58b8"
//         }
//     ],
//     "is_epic": true
// }

// struct Estimate {
//     value: usize,
// }

#[derive(Clone, Debug, Deserialize)]
pub struct Board {
    pub pipelines: Vec<Pipeline>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub issues: Vec<IssueRef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct IssueRef {
    #[serde(rename = "issue_number")]
    pub number: IssueNumber,
    pub is_epic: bool,
}

#[derive(Debug)]
pub struct Client {
    endpoints: Endpoints,
    http_client: HttpClient,
    headers: HeaderMap,
}

impl Client {
    /// Create a new API client.
    pub fn new(token: Token) -> Result<Client> {
        Ok(Client {
            endpoints: Endpoints::new(DEFAULT_ENDPOINT.clone())?,
            http_client: build_http_client(&token)?,
            headers: build_headers(&token)?,
        })
    }

    /// Create a new API client.
    pub fn get_oldest_board(&self, repo_id: &RepoId) -> Result<Board> {
        self.get::<_, Board>(self.endpoints.oldest_board(repo_id)?)
    }

    fn get<LocationT, SuccessT>(&self, url: LocationT) -> Result<SuccessT>
    where
        LocationT: IntoUrl + std::fmt::Display,
        for<'de> SuccessT: Deserialize<'de>,
    {
        log::debug!("Attempting GET `{}`", url);
        self.http_client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .with_context(|| "GET operation failed.")?
            .error_for_status()
            .with_context(|| "GET operation failed.")?
            .json::<SuccessT>()
            .with_context(|| "Could not parse JSON response")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token(String);

impl std::str::FromStr for Token {
    type Err = anyhow::Error;

    fn from_str(token: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(token.into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoId(pub String);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IssueNumber(pub usize);

#[derive(Debug)]
struct Endpoints {
    base: Url,
}

impl Endpoints {
    pub fn new(base: Url) -> Result<Self> {
        // let datasets = base
        //     .join("")
        //     .chain_err(|| ErrorKind::Unknown {
        //         message: "Could not build URL for dataset resources.".to_owned(),
        //     })?;
        // let sources = base
        //     .join("/api/v1/sources")
        //     .chain_err(|| ErrorKind::Unknown {
        //         message: "Could not build URL for source resources.".to_owned(),
        //     })?;
        // let buckets = base
        //     .join("/api/_private/buckets")
        //     .chain_err(|| ErrorKind::Unknown {
        //         message: "Could not build URL for bucket resources.".to_owned(),
        //     })?;
        // let users = base
        //     .join("/api/_private/users")
        //     .chain_err(|| ErrorKind::Unknown {
        //         message: "Could not build URL for users resources.".to_owned(),
        //     })?;
        // let current_user = base.join("/auth/user").chain_err(|| ErrorKind::Unknown {
        //     message: "Could not build URL for users resources.".to_owned(),
        // })?;
        Ok(Endpoints { base })
    }

    fn issue(&self, repo_id: &RepoId, issue_number: &IssueNumber) -> Result<Url> {
        self.base
            .join(&format!(
                "/p1/repositories/{}/issues/{}",
                repo_id.0, issue_number.0
            ))
            .with_context(|| {
                format!(
                    "Could not build URL for issue with repo_id `{}`, issue_number `{}`.",
                    repo_id.0, issue_number.0
                )
            })
    }

    fn oldest_board(&self, repo_id: &RepoId) -> Result<Url> {
        self.base
            .join(&format!("/p1/repositories/{}/board", repo_id.0))
            .with_context(|| {
                format!(
                    "Could not build URL for oldest board with repo_id `{}`.",
                    repo_id.0
                )
            })
    }
}

fn build_http_client(token: &Token) -> Result<HttpClient> {
    Ok(HttpClient::builder().gzip(true).build()?)
}

fn build_headers(token: &Token) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-authentication-token"),
        HeaderValue::from_str(&token.0)?,
    );
    Ok(headers)
}

static DEFAULT_ENDPOINT: Lazy<Url> =
    Lazy::new(|| Url::parse("https://api.zenhub.com").expect("Default URL is well-formed"));
