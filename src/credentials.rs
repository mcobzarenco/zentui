use anyhow::{anyhow, Context, Result};
use keyring::{Keyring, KeyringError};
use std::io::{self, Write};

use crate::github::Token as GithubToken;
use crate::zenhub::Token as ZenhubToken;

pub fn from_arg_keyring_or_stdin<T: ServiceToken>(arg_token: Option<T>) -> Result<T> {
    let token = match arg_token {
        Some(token) => {
            if let Err(error) = set_keyring_token(&token) {
                log::warn!("{}", error);
            }
            token
        }
        None => match get_keyring_token()
            .map_err(|error| {
                log::warn!("{}", error);
            })
            .ok()
            .flatten()
        {
            Some(token) => token,
            None => {
                eprintln!(concat!(
                    "Generate a Github personal access token: https://github.com/settings/tokens ",
                    "(the token will be stored in your system's keyring)"
                ));
                let token = read_token_from_stdin::<T>()?.into();
                if let Err(error) = set_keyring_token(&token) {
                    log::warn!("{}", error);
                }
                token
            }
        },
    };
    Ok(token)
}

fn get_keyring_token<T: ServiceToken>() -> Result<Option<T>> {
    match keyring_for::<T>().get_password() {
        Ok(password) => Ok(Some(password.into())),
        Err(KeyringError::NoPasswordFound) => Ok(None),
        Err(error) => Err(anyhow!(
            "Could not get Github token from keyring: {}",
            error
        )),
    }
}

fn set_keyring_token<T: ServiceToken>(token: &T) -> Result<()> {
    keyring_for::<T>()
        .set_password(&token.as_str())
        .map_err(|error| anyhow!("Could not store Github token in the keyring: {}", error))
}

fn keyring_for<T: ServiceToken>() -> Keyring<'static> {
    Keyring::new(APPLICATION_NAME, T::key())
}

fn read_token_from_stdin<T: ServiceToken>() -> Result<T> {
    let mut input = String::new();
    loop {
        input.clear();
        write!(io::stderr(), "{} API token: ", T::name())
            .and_then(|_| io::stderr().flush())
            .and_then(|_| io::stdin().read_line(&mut input))
            .with_context(|| "Failed to read API token from stdin.")?;
        input = input.trim().into();

        if !input.is_empty() {
            break;
        }
    }
    Ok(input.into())
}

pub trait ServiceToken: From<String> {
    fn name() -> &'static str;

    fn key() -> &'static str;

    fn as_str(&self) -> &str;
}

impl ServiceToken for GithubToken {
    fn name() -> &'static str {
        "Github"
    }

    fn key() -> &'static str {
        "token@github"
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl ServiceToken for ZenhubToken {
    fn name() -> &'static str {
        "Zenhub"
    }

    fn key() -> &'static str {
        "token@zenhub"
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

const APPLICATION_NAME: &str = "zentui";

// pub fn get_zenhub_token() -> Result<Option<ZenhubToken>> {
//     match zenhub_keyring().get_password() {
//         Ok(password) => Ok(Some(password.into())),
//         Err(KeyringError::NoPasswordFound) => Ok(None),
//         Err(error) => Err(anyhow!(
//             "Could not get Zenhub token from keyring: {}",
//             error
//         )),
//     }
// }

// pub fn set_zenhub_token(token: &ZenhubToken) -> Result<()> {
//     zenhub_keyring()
//         .set_password(&token.0)
//         .map_err(|error| anyhow!("Could not store Zenhub token in the keyring: {}", error))
// }

// fn zenhub_keyring() -> Keyring<'static> {
//     Keyring::new(APPLICATION_NAME, KEYRING_ZENHUB_USERNAME)
// }

// const KEYRING_GITHUB_USERNAME: &str = "token@github";
// const KEYRING_ZENHUB_USERNAME: &str = "token@zenhub";
