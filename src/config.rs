use crate::{auth, opts::Opts, util::open_browser, MastoClient, APP_NAME};

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Password};
use tracing::info;

pub async fn wizard(opts: Opts) -> Result<()> {
    let account = opts.account();
    let client_id = client_id()?;
    let client_secret = client_secret()?;

    open_browser(&MastoClient::login_url(&opts, &client_id))?;

    let code = auth_code()?;

    info!("Fetching authorization token");

    let token = MastoClient::fetch_token(&opts, &client_id, &client_secret, code).await?;

    auth::set(APP_NAME, &account, &token)?;

    info!("Token stored correctly");

    Ok(())
}

pub fn client_id() -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Client key")
        .interact_text()
        .map(Ok)?
}

pub fn client_secret() -> Result<String> {
    Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Client secret")
        .interact()
        .map(Ok)?
}

pub fn auth_code() -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Autentication Code")
        .interact()
        .map(Ok)?
}
