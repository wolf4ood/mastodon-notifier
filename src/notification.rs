use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use futures::{Stream, StreamExt, TryStreamExt};
use tokio::sync::Mutex;
use zbus::{
    dbus_proxy, fdo::DBusProxy, Connection, MatchRule, Message, MessageStream, MessageType,
};
use zvariant::Value;

use crate::{client::Notification, APP_NAME};

pub type NotificationStore = Arc<Mutex<HashMap<u32, Notification>>>;

#[derive(Clone)]
pub struct NotificationManager {
    connection: Connection,
    notifications: NotificationStore,
}

pub struct NotificationHandle {
    connection: Connection,
    manager: NotificationManager,
}

#[derive(Debug)]
pub enum ActionResponse {
    Close((Notification, u32)),
    Custom((Notification, String)),
}

impl NotificationHandle {
    pub fn stream(self) -> impl Stream<Item = Result<ActionResponse>> {
        let notifications = self.manager.notifications();
        MessageStream::from(self.connection)
            .filter(|message| {
                futures::future::ready(
                    message
                        .as_ref()
                        .map(|m| m.message_type() == MessageType::Signal)
                        .unwrap_or_default(),
                )
            })
            .map_err(anyhow::Error::from)
            .filter_map(move |message| message_mapper(notifications.clone(), message))
            .boxed()
    }
}

async fn remove_notification(
    notification_id: u32,
    notifications: &NotificationStore,
) -> Option<Notification> {
    notifications.lock().await.remove(&notification_id)
}

async fn message_mapper(
    notifications: NotificationStore,
    message: Result<Arc<Message>>,
) -> Option<Result<ActionResponse>> {
    let message = message.ok()?;
    let header = message.header().ok()?;

    match header.member() {
        Ok(Some(name)) if name == "ActionInvoked" => match message.body::<(u32, String)>() {
            Ok((nid, action)) => remove_notification(nid, &notifications)
                .await
                .map(|noty| Ok(ActionResponse::Custom((noty, action)))),
            _ => None,
        },
        Ok(Some(name)) if name == "NotificationClosed" => match message.body::<(u32, u32)>() {
            Ok((nid, reason)) => remove_notification(nid, &notifications)
                .await
                .map(|noty| Ok(ActionResponse::Close((noty, reason)))),
            _ => None,
        },
        _ => None,
    }
}

impl NotificationManager {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        register_proxy(&connection).await?;

        Ok(NotificationManager {
            connection,
            notifications: NotificationStore::default(),
        })
    }

    pub fn handle(&self) -> NotificationHandle {
        NotificationHandle {
            connection: self.connection.clone(),
            manager: self.clone(),
        }
    }

    pub async fn close(&self, notification_id: u32) -> Result<()> {
        let proxy = NotificationsProxy::new(&self.connection).await?;

        proxy
            .close_notification(notification_id)
            .await
            .map(|_| ())
            .map_err(anyhow::Error::from)
    }
    pub async fn send(&self, noty: Notification, icon: &str, timeout: u32) -> Result<u32> {
        let proxy = NotificationsProxy::new(&self.connection).await?;
        let reply = proxy
            .notify(
                APP_NAME,
                0,
                icon,
                &noty.summary(),
                &noty.body(),
                &["default", "default"],
                &HashMap::new(),
                timeout as i32,
            )
            .await?;

        let mut notifications = self.notifications.lock().await;

        notifications.insert(reply, noty);

        self.schedule_prune(reply, timeout).await;

        Ok(reply)
    }

    async fn schedule_prune(&self, id: u32, timeout: u32) {
        let notifications = self.notifications.clone();
        tokio::task::spawn(async move {
            let delay = timeout + 200;
            tokio::time::sleep(Duration::from_millis(delay.into())).await;
            remove_notification(id, &notifications).await;
        });
    }

    pub fn notifications(&self) -> Arc<Mutex<HashMap<u32, Notification>>> {
        self.notifications.clone()
    }
}

async fn register_proxy(connection: &Connection) -> Result<(), anyhow::Error> {
    let proxy = DBusProxy::new(connection).await?;
    proxy
        .add_match_rule(
            MatchRule::builder()
                .interface("org.freedesktop.Notifications")?
                .member("ActionInvoked")?
                .build(),
        )
        .await?;
    proxy
        .add_match_rule(
            MatchRule::builder()
                .interface("org.freedesktop.Notifications")?
                .member("NotificationClosed")?
                .build(),
        )
        .await?;
    Ok(())
}

#[dbus_proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: &HashMap<&str, &Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    fn close_notification(&self, notification_id: u32) -> zbus::Result<u32>;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::client::{Account, Notification, NotificationType};

    use super::NotificationManager;

    use futures::TryStreamExt;

    #[tokio::test]
    async fn noty_test() {
        let manager = NotificationManager::new().await.unwrap();

        let noty_id = manager
            .send(
                Notification::new(account(), NotificationType::Favourite, None),
                "",
                1000,
            )
            .await
            .unwrap();

        let inner = manager.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            inner.close(noty_id).await.unwrap();
        });

        let handle = manager.handle();

        let mut stream = handle.stream();

        let response = stream.try_next().await.unwrap().unwrap();

        assert!(matches!(response, super::ActionResponse::Close(..)))
    }

    #[tokio::test]
    async fn noty_test_prune() {
        let manager = NotificationManager::new().await.unwrap();

        manager
            .send(
                Notification::new(account(), NotificationType::Favourite, None),
                "",
                500,
            )
            .await
            .unwrap();

        let guard = manager.notifications.lock().await;
        let len = guard.len();

        drop(guard);

        assert_eq!(1, len);

        tokio::time::sleep(Duration::from_millis(2000)).await;

        let guard = manager.notifications.lock().await;
        let len = guard.len();
        assert_eq!(0, len);
    }
    fn account() -> Account {
        Account::new(
            String::from("1"),
            String::from("test"),
            "test".to_string(),
            "test@test.org".to_string(),
        )
    }
}
