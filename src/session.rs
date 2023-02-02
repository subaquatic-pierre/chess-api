use std::sync::{Arc, Mutex};
use std::time::Instant;

use actix::prelude::*;
use actix_web_actors::ws::{self, CloseCode, CloseReason};
use serde::Serialize;

use crate::constants::{CLIENT_TIMEOUT, HEARTBEAT_INTERVAL};
use crate::message::{Json, Message, MessageType};
use crate::server::ChatServer;
use crate::unlock;

pub type SessionId = usize;

#[derive(Serialize)]
pub struct SessionProfile {
    username: String,
    room: String,
    id: SessionId,
}

impl Json for SessionProfile {}

#[derive(Debug)]
pub struct WsSession {
    /// unique session id
    pub id: usize,
    pub hb: Instant,

    /// joined room
    pub room: String,

    /// peer name
    pub username: String,

    pub chat_server: Arc<Mutex<ChatServer>>,
}

impl WsSession {
    fn handle_command(&mut self, msg: &str, ctx: &mut ws::WebsocketContext<Self>) {
        let v: Vec<&str> = msg.splitn(2, ' ').collect();
        match v[0] {
            "/list-rooms" => {
                // Send ListRooms message to chat server and wait for
                // response
                let chat_server = unlock!(self.chat_server);
                let rooms = chat_server.list_rooms();

                let msg = self.new_message(MessageType::RoomList, &rooms.join(","));

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/list-users" => {
                // Send ListRooms message to chat server and wait for
                // response
                let chat_server = unlock!(self.chat_server);
                let users = chat_server.list_users(&self.room);

                let msg = self.new_message(MessageType::UserList, &users.join(","));

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/self-info" => {
                let profile = SessionProfile {
                    username: self.username.clone(),
                    room: self.room.clone(),
                    id: self.id,
                };

                let msg = self.new_message(MessageType::Info, &profile.to_json());

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/join-room" => {
                if v.len() == 2 {
                    // TODO:
                    // Very naive logic for joining room
                    // currently allows any room name to be joined
                    let new_room = v[1].to_owned();

                    // the only time the user changes a room is here
                    // the current room of the session is updated here
                    // ONLY
                    self.room = new_room.clone();

                    let mut chat_server = unlock!(self.chat_server);

                    chat_server.join_room(&new_room, self.id, &self.username);
                } else {
                    let msg = self.new_message(MessageType::Error, "Room name is required");

                    // send message back to client session
                    ctx.text(msg.to_string());
                }
            }
            _ => {
                let msg =
                    self.new_message(MessageType::Error, &format!("Unknown command: {msg:?}"));

                // send message back to client session
                ctx.text(msg.to_string())
            }
        }
    }

    fn handle_message(&mut self, msg: &str) {
        let chat_server = unlock!(self.chat_server);

        let msg = self.new_message(MessageType::ClientMessage, msg);

        chat_server.broadcast(&self.room, msg, self.id);
    }

    fn new_message(&self, msg_type: MessageType, content: &str) -> Message {
        Message {
            msg_type,
            from_id: self.id,
            username: self.username.clone(),
            content: content.to_string(),
        }
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<Message> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        // Disconnect message received from server
        // close Web Socket session
        if msg.msg_type == MessageType::Disconnect {
            ctx.close(Some(CloseReason {
                code: CloseCode::Normal,
                description: Some(msg.content),
            }));
        } else {
            ctx.text(msg.to_json());
        }
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // check that message is not an error
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        log::debug!("WEBSOCKET MESSAGE: {msg:?}");

        // handle all possible message types for stream handler
        match msg {
            // main message type, used to communicate with web socket session
            // check if message is a command or message to be sent to chat server
            ws::Message::Text(text) => {
                log::info!("WEBSOCKET MESSAGE: {text:?}");

                let m = text.trim();
                // we check for /sss type of messages
                if m.starts_with('/') {
                    // main method for handling command
                    self.handle_command(m, ctx);
                } else {
                    self.handle_message(m);
                }
            }
            // generic ws message types below
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

// ---
// Below implementations need no further work
// they are boiler plate session implementations
// ---

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with ChatServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);
        let mut chat_server = self.chat_server.lock().unwrap();

        // notify chat server
        chat_server.connect(self.id, &self.username, ctx.address());

        log::info!("CLIENT CONNECTED");

        let msg = self.new_message(
            MessageType::Status,
            "You successfully connected to the server",
        );

        ctx.text(msg.to_json())
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        let mut chat_server = unlock!(self.chat_server);

        log::info!("CLIENT DISCONNECTED");

        // notify chat server
        chat_server.disconnect(self.id);
        Running::Stop
    }
}

impl WsSession {
    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                let mut chat_server = unlock!(act.chat_server);
                chat_server.disconnect(act.id);

                // stop actor
                ctx.stop();
            }

            ctx.ping(b"");
        });
    }
}
