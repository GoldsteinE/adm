use std::sync::Arc;

use askama::Template;
use color_eyre::eyre::{self, WrapErr as _};
use secstr::SecUtf8;

use super::Status;
use crate::runner::Task;

// Keep unused variants for documentation
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, serde::Serialize)]
enum ParseMode {
    HTML,
    Markdown,
    MarkdownV2,
}

#[derive(Debug, serde::Serialize)]
struct SendMessage<'a> {
    chat_id: i64,
    text: &'a str,
    parse_mode: ParseMode,
}

#[derive(Debug, Template)]
#[template(path = "telegram-message.html")]
struct MessageTemplate<'a> {
    pub task: &'a Task,
    pub status: &'a Status,
}

impl<'a> MessageTemplate<'a> {
    fn new(task: &'a Task, status: &'a Status) -> Self {
        Self { task, status }
    }
}

#[derive(Clone)]
pub struct Telegram {
    http: Arc<awc::Client>,
    url: SecUtf8,
    pub chats: Vec<i64>,
}

impl Telegram {
    pub fn new(http: Arc<awc::Client>, token: &SecUtf8, chats: Vec<i64>) -> Self {
        let url = SecUtf8::from(format!(
            "https://api.telegram.org/bot{}/sendMessage",
            token.unsecure()
        ));
        Self { http, url, chats }
    }

    async fn try_notify(&self, task: Arc<Task>, status: Arc<Status>) -> eyre::Result<()> {
        let text = &MessageTemplate::new(&task, &status)
            .render()
            .wrap_err("Failed to render message template")?;

        for chat_id in self.chats.iter().copied() {
            let message = SendMessage {
                chat_id,
                text,
                parse_mode: ParseMode::HTML,
            };

            let mut resp = self
                .http
                .post(self.url.unsecure())
                .send_json(&message)
                .await
                .map_err(|err| eyre::eyre!("Failed to send request to Telegram: {}", err))?;

            if resp.status().as_u16() >= 400 {
                eyre::eyre!(
                    "Telegram API returned error: {}\n{}",
                    resp.status(),
                    String::from_utf8_lossy(
                        resp.body()
                            .await
                            .wrap_err("Failed to fetch Telegram response body")?
                            .as_ref()
                    )
                );
            }
        }

        Ok(())
    }

    pub async fn notify(self: Arc<Self>, task: Arc<Task>, status: Arc<Status>) {
        if let Err(err) = self.try_notify(task, status).await {
            tracing::error!("Failed sending Telegram notification: {}", err);
        }
    }
}
