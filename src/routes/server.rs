use actix_web::{get, web, web::scope, Responder, Scope};

use std::sync::atomic::Ordering;

use crate::server::ChatServer;
use actix_files::NamedFile;

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

/// Displays state
#[get("count")]
async fn get_count(app_state: web::Data<ChatServer>) -> impl Responder {
    let current_count = app_state.visitor_count.load(Ordering::SeqCst);
    format!("Visitors: {current_count}")
}

pub fn register_server_routes() -> Scope {
    scope("").service(index).service(get_count)
}
