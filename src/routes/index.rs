use std::sync::atomic::Ordering;

use actix_files::NamedFile;
use actix_web::{get, web, Responder};

use crate::actors::server::ChatServer;

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
