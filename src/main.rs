#![deny(unsafe_code)]
#![deny(non_ascii_idents)]
#![deny(pointer_structural_match)]
#![warn(clippy::pedantic)]
#![warn(absolute_paths_not_starting_with_crate)]
#![warn(anonymous_parameters)]
#![warn(deprecated_in_future)]
#![warn(elided_lifetimes_in_paths)]
#![warn(explicit_outlives_requirements)]
#![warn(meta_variable_misuse)]
#![warn(missing_debug_implementations)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_crate_dependencies)]
#![warn(unused_import_braces)]
#![warn(unused_lifetimes)]
#![warn(unused_qualifications)]
#![warn(variant_size_differences)]

mod config;
mod git;
mod github;
mod hooks;
mod http;
mod lock_manager;
mod notifier;
mod runner;
mod signature;

use std::sync::Arc;

use actix::{Actor, SyncArbiter};
use actix_web::{guard, middleware::Logger, web, App, HttpServer};
use color_eyre::eyre;

use crate::runner::Runner;

#[actix_web::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_log::LogTracer::init()?;
    tracing::subscriber::set_global_default(tracing_subscriber::fmt().finish())?;

    let config::Config {
        host,
        port,
        repo_root,
        webhook_secret,
        telegram_token,
        telegram_groups,
        parallel_builds,
    } = envy::prefixed("ADM_").from_env()?;

    let notifier = notifier::Notifier::new(notifier::Config {
        telegram_token,
        telegram_groups,
    })
    .start();
    let lock_manager = Arc::new(lock_manager::LockManager::new());
    let builder = SyncArbiter::start(parallel_builds as usize, move || {
        Runner::new(repo_root.clone(), lock_manager.clone(), notifier.clone())
    });

    HttpServer::new(move || {
        App::new()
            .data(builder.clone())
            .app_data(http::WebhookConfig::new(webhook_secret.clone()))
            .wrap(Logger::default())
            .route(
                "/{repo}",
                web::post()
                    .guard(guard::Header("X-GitHub-Event", "push"))
                    .to(hooks::push_hook),
            )
    })
    .bind((host, port))?
    .run()
    .await
    .map_err(Into::into)
}
