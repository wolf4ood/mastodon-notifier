use std::time::Duration;

use crate::{
    auth,
    notification::{ActionResponse, NotificationManager},
    opts::Opts,
    util::open_browser,
    MastoClient, APP_NAME,
};

use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use tracing::{error, info};

pub async fn run(opts: Opts) -> Result<()> {
    let account = opts.account();

    let token = fetch_token(&account).await;

    info!("Found stored token for user {}", account);

    let icon = opts.icon.clone().unwrap_or_default();
    let timeout = opts.timeout;

    let client = MastoClient::with_token(opts, token);
    let mut notification_stream = client.notifications().await?;

    let notification_manager = NotificationManager::new().await?;
    let handle = notification_manager.handle();

    info!("Started mastodon notify daemon on account {}", account);

    tokio::task::spawn(async {
        let mut stream = handle.stream();
        while let Ok(response) = stream.try_next().await {
            if let Some(ActionResponse::Custom((noty, _))) = response {
                if let Some(url) = noty.url() {
                    let _ = open_browser(&url);
                }
            }
        }
    });

    while let Some(notification) = notification_stream.next().await {
        match notification {
            Ok(noty) => {
                notification_manager.send(noty, &icon, timeout).await?;
            }
            Err(error) => error!("Notification error {}", error),
        }
    }
    Ok(())
}

async fn fetch_token(account: &String) -> String {
    loop {
        match auth::get(APP_NAME, account) {
            Ok(token) => break token,
            Err(_) => {
                info!("Token for user not found {}. Please run the wizard with --config mode for configuring the authentication token", account);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
