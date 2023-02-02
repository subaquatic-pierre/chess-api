use actix_web::{get, web, web::scope, Error, HttpRequest, Responder, Scope};
use actix_web_actors::ws;
use rand::{self, Rng};
use std::time::Instant;

use crate::app::AppState;
use crate::session;
use crate::{server, unlock};

use crate::message::{Message, MessageType};

/// Entry point for our websocket route
#[get("/ws/{username}")]
async fn chat_route(
    name: web::Path<String>,
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<AppState>,
) -> Result<impl Responder, Error> {
    // register session with random id
    let mut rng = rand::thread_rng();
    let id = rng.gen::<usize>();
    // check name is not taken on server
    ws::start(
        session::WsSession {
            id,
            hb: Instant::now(),
            room: "main".to_owned(),
            username: name.to_owned(),
            chat_server: srv.chat_server.clone(),
        },
        &req,
        stream,
    )
}

#[get("/check-username/{name}")]
async fn check_username(name: web::Path<String>, srv: web::Data<AppState>) -> impl Responder {
    // check server for names
    let chat_server = unlock!(srv.chat_server);

    log::info!("CHECKING USERNAMES...");
    for session in chat_server.sessions.iter() {
        log::info!("Username: {session:?}")
    }

    // check if username exists
    if chat_server
        .sessions
        .iter()
        .map(|(_sessions, (username, _addr))| username.clone())
        .any(|x| x == name.to_string())
    {
        let msg = Message {
            msg_type: MessageType::Error,
            from_id: 0,
            username: "anonymous".to_string(),
            content: "Username is taken".to_string(),
        };
        msg.to_http()
    } else {
        let msg = Message {
            msg_type: MessageType::Info,
            from_id: 0,
            username: "anonymous".to_string(),
            content: "Username available".to_string(),
        };
        msg.to_http()
    }
}

#[get("/sessions")]
async fn sessions(req: HttpRequest, srv: web::Data<AppState>) -> impl Responder {
    // check server for names
    let chat_server = srv.chat_server.lock().unwrap();

    let mut infos = Vec::new();

    for session in chat_server.sessions.iter() {
        let info = format!("Session: {session:?}");
        infos.push(info)
    }

    let msg = Message {
        msg_type: MessageType::Info,
        from_id: 0,
        username: "anonymous".to_string(),
        content: format!("{infos:?}"),
    };

    msg.to_http()
}

pub fn register_chat_routes() -> Scope {
    scope("")
        .service(sessions)
        .service(check_username)
        .service(chat_route)
}
