#![allow(clippy::from_over_into)]

use dotenv::dotenv;
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
mod utils;

use app::new_app_state;
use routes::{register_chat_routes, register_server_routes};
use utils::print_log_levels;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // load from .env
    dotenv().ok();
    // get host and port from .env
    let port = std::env::var("PORT")
        .expect("Port must be set by .env")
        .parse::<u16>()
        .unwrap();
    let host = std::env::var("HOST").expect("Host must be set by .env");

    env_logger::init_from_env(env_logger::Env::new().filter("RUST_LOG"));

    print_log_levels();

    let app_state = new_app_state();

    log::info!("starting HTTP server at http://{host}:{port}");

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
    .bind((host, port))?
    .run()
    .await
}
