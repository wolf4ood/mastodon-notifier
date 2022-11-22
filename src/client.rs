use anyhow::Result;
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use serde::{Deserialize, Serialize};

use crate::opts::Opts;
use futures::{Stream, StreamExt};

const REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";
const SCOPES: &str = "read";
const GRANT_TYPE: &str = "authorization_code";

pub struct MastoClient {
    settings: Opts,
    token: String,
}

#[derive(Serialize)]
struct TokenRequest<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    code: String,
    redirect_uri: &'a str,
    grant_type: &'a str,
    scope: &'a str,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

impl MastoClient {
    pub async fn fetch_token(
        settings: &Opts,
        client_id: &str,
        client_secret: &str,
        code: String,
    ) -> Result<String> {
        let client = reqwest::Client::new();

        let req = TokenRequest {
            client_id,
            client_secret,
            scope: SCOPES,
            redirect_uri: REDIRECT_URI,
            grant_type: GRANT_TYPE,
            code,
        };

        let token = client
            .post(format!("https://{}/oauth/token", settings.host))
            .json(&req)
            .send()
            .await?
            .json::<TokenResponse>()
            .await?;

        Ok(token.access_token)
    }

    pub fn with_token(settings: Opts, token: String) -> MastoClient {
        MastoClient { settings, token }
    }

    pub fn login_url(settings: &Opts, client_id: &str) -> String {
        format!(
            "https://{}/oauth/authorize?response_type=code&redirect_uri={}&scope={}&client_id={}",
            settings.host, REDIRECT_URI, SCOPES, client_id
        )
    }

    pub async fn notifications(&self) -> Result<impl Stream<Item = Result<Notification>>> {
        let url = format!(
            "wss://{}/api/v1/streaming/?stream=user&access_token={}",
            self.settings.host, self.token
        );

        let (connection, _) = connect_async(url).await?;

        Ok(connection.filter_map(map_to_notification).boxed())
    }
}

// TODO extract into a speratated task
async fn map_to_notification(
    msg: std::result::Result<Message, async_tungstenite::tungstenite::Error>,
) -> Option<Result<Notification>> {
    match msg {
        Ok(Message::Text(text)) => map_to_notification_internal(text).transpose(),
        Ok(_) => None,
        Err(e) => Some(Err(e.into())),
    }
}

fn map_to_notification_internal(text: String) -> Result<Option<Notification>> {
    let parsed = serde_json::from_str::<StreamMessage>(&text)?;
    if let EventType::Notification = parsed.event {
        serde_json::from_str::<Notification>(&parsed.payload)
            .map_err(anyhow::Error::from)
            .map(Some)
    } else {
        Ok(None)
    }
}

#[derive(Deserialize, Debug)]
pub struct Notification {
    pub account: Account,
    #[serde(rename = "type")]
    pub kind: NotificationType,
    pub status: Option<Status>,
}

impl Notification {
    pub fn new(account: Account, kind: NotificationType, status: Option<Status>) -> Self {
        Self {
            account,
            kind,
            status,
        }
    }

    pub fn summary(&self) -> String {
        match self.kind {
            NotificationType::Mention => format!("{} mentioned you", self.account.name()),
            NotificationType::Follow => "Follow".to_string(),
            NotificationType::Reblog => format!("{} boosted your status", self.account.name()),
            NotificationType::Favourite => {
                format!("{} favourited your status", self.account.name())
            }
            NotificationType::Status => format!("{} just posted", self.account.name()),
            NotificationType::Others => String::default(),
        }
    }

    pub fn body(&self) -> String {
        match self.kind {
            NotificationType::Mention
            | NotificationType::Reblog
            | NotificationType::Favourite
            | NotificationType::Status => self
                .status
                .as_ref()
                .map(|status| status.plain_content())
                .unwrap_or_default(),
            NotificationType::Follow => format!("{} is now following you", self.account.name()),
            NotificationType::Others => String::default(),
        }
    }

    pub fn url(&self) -> Option<String> {
        match self.kind {
            NotificationType::Favourite
            | NotificationType::Mention
            | NotificationType::Reblog
            | NotificationType::Status => self.status.as_ref().map(|status| status.url.clone()),
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Mention,
    Follow,
    Reblog,
    Favourite,
    Status,
    #[serde(other)]
    Others,
}

#[derive(Deserialize, Debug)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub acct: String,
}

impl Account {
    pub fn new(id: String, username: String, display_name: String, acct: String) -> Self {
        Self {
            id,
            username,
            display_name,
            acct,
        }
    }

    pub fn name(&self) -> &str {
        if self.display_name.is_empty() {
            &self.username
        } else {
            &self.display_name
        }
    }
}
#[derive(Deserialize, Debug)]
pub struct StreamMessage {
    pub event: EventType,
    pub payload: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Notification,
    #[serde(other)]
    Others,
}

#[derive(Deserialize, Debug)]
pub struct Status {
    pub id: String,
    pub url: String,
    pub account: Account,
    pub content: String,
    pub reblog: Option<Box<Status>>,
    pub in_reply_to_id: Option<String>,
}

impl Status {
    pub fn plain_content(&self) -> String {
        html2text::from_read(self.content.as_bytes(), 20)
    }
}
