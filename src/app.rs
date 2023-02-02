#![allow(clippy::from_over_into)]

use std::sync::{atomic::AtomicUsize, Arc, Mutex};

use actix_web::{web::Data, App};

use crate::server::ChatServer;

pub struct AppState {
    pub app_name: String,
    pub chat_server: Arc<Mutex<ChatServer>>,
}

pub fn new_app_state() -> Data<AppState> {
    // keep a count of the number of visitors
    let count = Arc::new(AtomicUsize::new(0));

    // start chat server actor
    let server = ChatServer::new(count);

    Data::new(AppState {
        app_name: "Chat Server".to_string(),
        chat_server: Arc::new(Mutex::new(server)),
    })
}
