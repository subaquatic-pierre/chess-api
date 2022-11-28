#![allow(clippy::from_over_into)]

use std::sync::{atomic::AtomicUsize, Arc};

use actix::*;
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};

mod actors;
mod constants;
mod messages;
mod models;
mod routes;

use actors::server;

use routes::{chat, index};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // keep a count of the number of visitors
    let count = Arc::new(AtomicUsize::new(0));

    // start chat server actor
    let server = server::ChatServer::new(count.clone()).start();

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            .service(chat::chat_route)
            .service(index::index)
            .service(index::get_count)
            .service(Files::new("/static", "./static"))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
