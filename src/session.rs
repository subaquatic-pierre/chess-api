use std::collections::HashMap;
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
    game: String,
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
    pub game: String,

    /// peer name
    pub username: String,

    pub chat_server: Arc<Mutex<ChatServer>>,
}

impl WsSession {
    fn handle_command(&mut self, msg: &str, ctx: &mut ws::WebsocketContext<Self>) {
        let v: Vec<&str> = msg.splitn(2, ' ').collect();
        match v[0] {
            // ---
            // Chat commands
            // ---
            "/list-rooms" => {
                // Send ListRooms message to chat server and wait for
                // response
                let chat_server = unlock!(self.chat_server);
                let rooms = chat_server.list_rooms();

                let msg = self.new_message(MessageType::RoomList, &rooms.join(","), true);

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/list-users" => {
                // Send ListRooms message to chat server and wait for
                // response
                let chat_server = unlock!(self.chat_server);
                let users = chat_server.list_users(&self.room);

                let msg = self.new_message(MessageType::UserList, &users.join(","), true);

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
                    self.game = "none".to_string();

                    let mut chat_server = unlock!(self.chat_server);

                    chat_server.join_room(&new_room, self.id, &self.username);
                } else {
                    let msg = self.new_message(MessageType::Error, "Room name is required", true);

                    // send message back to client session
                    ctx.text(msg.to_string());
                }
            }

            // ---
            // End chat commands
            // ---

            // ---
            // Game Commands
            // ---
            "/new-game" => {
                let mut server = unlock!(self.chat_server);

                // ensure game with that name does not already exist
                // return early if already exists
                let games = server.list_games();
                if games.contains_key(&self.username) {
                    let msg = self.new_message(
                        MessageType::Error,
                        &format!("Game with the name {} already exists", self.username),
                        true,
                    );

                    ctx.text(msg.to_string());
                    return;
                }

                // set room to `none`
                self.room = "in_game".to_string();
                // set game name
                self.game = self.username.clone();

                // create new game if no error above
                server.new_game(self.id, &self.username);
                let msg = self.new_message(
                    MessageType::Status,
                    &format!(
                        "New game with the name {} created and joined",
                        self.username
                    ),
                    true,
                );

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/join-game" => {
                if v.len() == 2 {
                    // TODO:
                    // Very naive logic for joining room
                    // currently allows any room name to be joined
                    let game_name = v[1].to_owned();

                    let mut server = unlock!(self.chat_server);

                    server.join_game(self.id, &game_name, &self.username);

                    // set room to `in_game`
                    self.room = "is_game".to_string();
                    // set game name
                    self.game = game_name.clone();

                    let msg = self.new_message(
                        MessageType::Status,
                        &format!("New joined {game_name} chess game"),
                        true,
                    );

                    // send message back to client session
                    ctx.text(msg.to_string());
                } else {
                    let msg = self.new_message(MessageType::Error, "Game name is required", true);

                    // send message back to client session
                    ctx.text(msg.to_string());
                }
            }

            "/leave-game" => {
                // TODO:
                // check if currently in game
                // if not return error message
                // saying `You are currently not in a game`

                let mut server = unlock!(self.chat_server);

                server.leave_game(&self.game, self.id);

                server.join_room("main", self.id, &self.username);

                // set room to `main`
                self.room = "main".to_string();
                // set game name
                self.game = "none".to_string();

                let msg = self.new_message(
                    MessageType::Status,
                    &format!("You left {} chess game and joined the main room", self.game),
                    true,
                );

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/game-move" => {
                if v.len() == 2 {
                    // TODO:
                    // ensure user is in a game
                    let move_str = v[1].to_owned();

                    let mut server = unlock!(self.chat_server);

                    server.send_game_move(&self.game, &move_str, self.id);

                    let msg = self.new_message(
                        MessageType::Status,
                        &format!("Game move sent {}", move_str),
                        true,
                    );

                    // send message back to client session
                    ctx.text(msg.to_string());
                } else {
                    let msg = self.new_message(MessageType::Error, "Move string is required", true);

                    // send message back to client session
                    ctx.text(msg.to_string());
                }
            }

            "/list-available-games" => {
                let server = unlock!(self.chat_server);

                let msg = self.new_message(
                    MessageType::AvailableGameList,
                    &server.available_games().join(","),
                    true,
                );

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/list-all-games" => {
                let server = unlock!(self.chat_server);

                let msg = self.new_message(
                    MessageType::AllGameList,
                    &server.all_games().join(","),
                    true,
                );

                // send message back to client session
                ctx.text(msg.to_string());
            }

            "/delete-game" => {
                if v.len() == 2 {
                    let game_name = v[1].to_owned();
                    let mut server = unlock!(self.chat_server);

                    server.delete_game(&game_name);

                    let msg = self.new_message(
                        MessageType::Status,
                        &format!("{} chess game deleted", self.game),
                        true,
                    );

                    // send message back to client session
                    ctx.text(msg.to_string());
                } else {
                    let msg = self.new_message(MessageType::Error, "Game name is required", true);

                    // send message back to client session
                    ctx.text(msg.to_string());
                }
            }

            // ---
            // End Game Commands
            // ---
            "/self-info" => {
                let profile = SessionProfile {
                    username: self.username.clone(),
                    room: self.room.clone(),
                    game: self.game.clone(),
                    id: self.id,
                };

                let msg = self.new_message(MessageType::SelfInfo, &profile.to_json(), true);

                // send message back to client session
                ctx.text(msg.to_string());
            }

            _ => {
                log::info!("UNKNOWN: {msg:?}");
                let msg = self.new_message(
                    MessageType::Error,
                    &format!("Unknown command: {msg:?}"),
                    true,
                );

                // send message back to client session
                ctx.text(msg.to_string())
            }
        }
    }

    fn handle_message(&mut self, msg: &str) {
        let chat_server = unlock!(self.chat_server);

        let msg = self.new_message(MessageType::ClientMessage, msg, false);

        chat_server.broadcast(&self.room, msg, self.id);
    }

    fn new_message(&self, msg_type: MessageType, content: &str, server_msg: bool) -> Message {
        let from_id = if server_msg { 0 } else { self.id };
        let username = if server_msg {
            "server".to_string()
        } else {
            self.username.clone()
        };
        Message {
            msg_type,
            from_id,
            username,
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

        let msg = self.new_message(MessageType::Status, "You connected to the server", true);

        ctx.text(msg.to_json());

        let session_id_msg = self.new_message(MessageType::Connect, &self.id.to_string(), true);

        ctx.text(session_id_msg.to_json())
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
