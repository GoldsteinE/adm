use std::fmt;

mod status;
mod telegram;
use std::sync::Arc;

pub use self::status::Status;

use actix::prelude::*;
use secstr::SecUtf8;

use crate::runner::Task;

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct Notification {
    pub task: Arc<Task>,
    pub status: Arc<Status>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub telegram_token: Option<SecUtf8>,
    pub telegram_groups: Option<Vec<i64>>,
}

#[derive(Clone)]
pub struct Notifier {
    telegram: Option<Arc<telegram::Telegram>>,
}

impl fmt::Debug for Notifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        struct Disabled;

        f.debug_struct("Notifier")
            .field(
                "telergam",
                match &self.telegram {
                    Some(telegram) => &telegram.chats,
                    None => &Disabled,
                },
            )
            .finish()
    }
}

impl Notifier {
    pub fn new(config: Config) -> Self {
        let http = Arc::new(awc::Client::new());
        let Config {
            telegram_token,
            telegram_groups,
        } = config;
        let telegram = telegram_token.and_then(|token| {
            telegram_groups.map(|groups| Arc::new(telegram::Telegram::new(http, &token, groups)))
        });
        Self { telegram }
    }
}

impl Actor for Notifier {
    type Context = Context<Self>;
}

impl Handler<Notification> for Notifier {
    type Result = <Notification as Message>::Result;

    fn handle(&mut self, msg: Notification, ctx: &mut Self::Context) -> Self::Result {
        let Notification { task, status } = msg;
        if let Some(telegram) = &self.telegram {
            ctx.spawn(telegram.clone().notify(task, status).into_actor(self));
        }
    }
}
