#![allow(dead_code)]

mod config;
mod git;
mod github;
mod hooks;
mod http;
mod lock_manager;
mod runner;
mod signature;
mod telegram;

use std::sync::Arc;

use actix::SyncArbiter;
use actix_web::{middleware::Logger, web, App, HttpServer};
use color_eyre::eyre;

use crate::runner::Runner;

#[actix_web::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_log::LogTracer::init()?;
    tracing::subscriber::set_global_default(tracing_subscriber::fmt().finish())?;

    let config::Config {
        repo_root,
        webhook_secret,
        // telegram_token,
        // telegram_groups,
        parallel_builds,
        ..
    } = envy::prefixed("ADM_").from_env()?;

    let lock_manager = Arc::new(lock_manager::LockManager::new());
    let addr = SyncArbiter::start(parallel_builds as usize, move || {
        Runner::new(repo_root.clone(), lock_manager.clone())
    });

    HttpServer::new(move || {
        App::new()
            .data(addr.clone())
            .app_data(http::WebhookConfig {
                key: Some(webhook_secret.clone()),
            })
            .wrap(Logger::default())
            .route("/{repo}", web::post().to(hooks::push_hook))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
    .map_err(Into::into)
}
