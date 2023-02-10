#![allow(clippy::from_over_into)]

use std::sync::{atomic::AtomicUsize, Arc, Mutex};

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};

mod app;
mod constants;
mod game;
mod macros;
mod message;
mod routes;
mod server;
mod session;

use app::new_app_state;
use routes::{register_chat_routes, register_server_routes};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let app_state = new_app_state();

    // TODO:
    // set const host and port
    log::info!("starting HTTP server at http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(register_chat_routes())
            .service(register_server_routes())
            .service(Files::new("/static", "./static"))
            .wrap(Logger::default())
            .wrap(Cors::permissive())
    })
    .workers(2)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
