#![allow(dead_code)]

mod github;
mod hooks;
mod http;
mod signature;

use actix_web::{web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/{repo}", web::post().to(hooks::push_hook)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
