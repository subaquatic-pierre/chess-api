use actix_web::{http::header::ContentType, HttpResponse};

use actix::prelude::*;
use actix_web_actors::ws;
use serde::Serialize;
use std::fmt::Display;

use crate::session::SessionId;

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum MessageType {
    Info,
    ClientMessage,
    RoomList,
    UserList,
    GameMove,
    Status,
    Error,
    Disconnect,
}

/// Chat server sends this messages to session
#[derive(Message, Debug, Clone, Serialize)]
#[rtype(result = "()")]
pub struct Message {
    pub msg_type: MessageType,
    pub from_id: SessionId,
    pub username: String,
    pub content: String,
}

impl Message {
    pub fn to_http(&self) -> HttpResponse {
        if self.msg_type == MessageType::Error {
            HttpResponse::BadRequest()
                .content_type(ContentType::json())
                .json(self.to_string())
        } else {
            {
                HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .json(self.to_string())
            }
        }
    }
}

impl Json for Message {}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body = serde_json::to_string(&self).unwrap();
        write!(f, "{body}")
    }
}

pub trait Json
where
    Self: Serialize,
{
    fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}
