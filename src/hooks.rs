use crate::{github::PushEvent, http::Webhook};


pub async fn push_hook(Webhook(hook): Webhook<PushEvent>) -> String {
    dbg!(hook);
    "OK".to_string()
}
