mod app;
mod credentials;
mod edit;
mod github;
mod settings;
mod zenhub;

use anyhow::Result;
use clap::Clap;
use flexi_logger::{opt_format, Logger};
use std::path::PathBuf;
use tokio::runtime::Builder as RuntimeBuilder;
use zi::{self, frontend::crossterm, layout, App as ZiApp};

use crate::{
    app::{App, Properties},
    github::{Client as GithubClient, RepoFullName, Token as GithubToken},
    zenhub::{Client as ZenhubClient, Token as ZenhubToken},
};

#[derive(Debug, Clap)]
struct Args {
    #[clap(long = "zenhub-token")]
    /// Zenhub token to use.
    zenhub_token: Option<ZenhubToken>,

    #[clap(long = "github-token")]
    /// Github token (a personal access token, it should have the `repo` scope enabled).
    github_token: Option<GithubToken>,

    #[clap(long = "settings-path", parse(from_os_str))]
    /// Path to the configuration file. It's usually ~/.config/zee on Linux.
    settings_path: Option<PathBuf>,

    #[clap(long = "create-settings")]
    /// Writes the default configuration to file, if the file doesn't exist
    create_settings: bool,

    #[clap(long = "log")]
    /// Enable debug logging to `zentui.log` file
    enable_logging: bool,

    #[clap(name = "repository")]
    /// Repository to open; the oldest existing Zenhub board will be used.
    repository: RepoFullName,
}

fn configure_logging() -> Result<()> {
    Logger::with_env_or_str("myprog=debug, mylib=debug")
        .log_to_file()
        .format(opt_format)
        .suppress_timestamp()
        .start()
        .map_err(anyhow::Error::from)?;
    Ok(())
}

fn start_app() -> Result<()> {
    let args = Args::parse();
    if args.enable_logging {
        configure_logging()?;
    }

    let github_token = credentials::from_arg_keyring_or_stdin(args.github_token)?;
    let zenhub_token = credentials::from_arg_keyring_or_stdin(args.zenhub_token)?;

    // Read the current settings. If we cannot for any reason, we'll use the
    // default ones -- ensure the editor opens in any environment.
    let settings = args
        .settings_path
        .or_else(|| settings::settings_path().map(Some).unwrap_or(None))
        .map_or_else(Default::default, settings::read_settings);

    let github_client = GithubClient::new(github_token)?;
    let zenhub_client = ZenhubClient::new(zenhub_token)?;

    let mut async_runtime = RuntimeBuilder::new()
        .threaded_scheduler()
        .enable_all()
        .core_threads(1)
        .build()?;

    let repo = async_runtime.block_on(github_client.get_repo(&args.repository))?;

    //     // Create a default settings file if requested by the user
    //     if args.create_settings {
    //         let settings_path = settings::settings_path()?;
    //         if !settings_path.exists() {
    //             settings::create_default_file(&settings_path)?;
    //         } else {
    //             log::warn!(
    //                 "Default settings file won't be created; a file already exists `{}`",
    //                 settings_path.display()
    //             );
    //         }
    //     }

    let mut app = ZiApp::new(layout::component::<App>(Properties {
        async_runtime: async_runtime.handle().clone(),
        github_client: github_client.into(),
        zenhub_client: zenhub_client.into(),
        repo,
    }));

    // Start the UI loop
    app.run_event_loop(zi::frontend::crossterm::incremental()?)?;
    Ok(())
}

fn main() -> Result<()> {
    start_app().map_err(|error| {
        log::error!("Zentui exited with: {}", error);
        error
    })
}
