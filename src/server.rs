use actix::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::game::{GameManager, SessionGame};
use crate::session::{SessionId, WsSession};
use crate::{
    message::{Message, MessageType},
    session,
};

#[derive(Debug)]
pub struct ChatServer {
    pub sessions: HashMap<SessionId, (String, Addr<WsSession>)>,
    pub rooms: HashMap<String, HashSet<SessionId>>,
    pub game_manager: GameManager,
    pub visitor_count: Arc<AtomicUsize>,
}

impl ChatServer {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> ChatServer {
        // default room
        let mut rooms = HashMap::new();
        rooms.insert("mountain".to_owned(), HashSet::new());
        rooms.insert("ocean".to_owned(), HashSet::new());
        rooms.insert("sky".to_owned(), HashSet::new());
        rooms.insert("space".to_owned(), HashSet::new());
        rooms.insert("main".to_owned(), HashSet::new());

        rooms.insert("lobby".to_owned(), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            visitor_count,
            game_manager: GameManager::new(),
        }
    }

    pub fn connect(&mut self, session_id: SessionId, username: &str, addr: Addr<WsSession>) {
        self.sessions
            .insert(session_id, (username.to_string(), addr));

        self.join_room("main", session_id, username);
    }

    pub fn disconnect(&mut self, id: SessionId) {
        // remove session ID from rooms
        if let Some((username, _addr)) = self.sessions.remove(&id) {
            // NOTE:
            // unable to send message to context
            // Addr is not valid tx anymore

            self.leave_all_rooms(id, &username);

            self.leave_all_games(id);

            // decrement visitor count
            self.visitor_count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Main broadcast message used to
    /// message all connected web socket sessions
    pub fn broadcast(&self, room: &str, message: Message, skip_id: SessionId) {
        // only send to people in room
        if let Some(session_ids) = self.rooms.get(room) {
            for id in session_ids {
                // ensure not to send to skip_id if flag set
                if id != &skip_id {
                    // get session addr from server
                    if let Some((_, addr)) = self.sessions.get(id) {
                        addr.do_send(message.clone());
                    }
                }
            }
        }
    }

    /// Main message used to send message only to the
    /// client with given session ID
    pub fn send_client_msg(&self, session_id: SessionId, msg: Message) {
        if let Some((_username, addr)) = self.sessions.get(&session_id) {
            addr.do_send(msg);
        }
    }

    // ---
    // Room Methods
    // ---

    pub fn join_room(&mut self, room_name: &str, session_id: SessionId, username: &str) {
        self.leave_all_rooms(session_id, username);

        self.leave_all_games(session_id);

        // join room which already exists or create new one
        self.rooms
            .entry(room_name.to_string())
            .or_insert_with(HashSet::new)
            .insert(session_id);

        // send message to client that joined room,
        // only if room name is not `lobby`
        if room_name != "lobby" {
            let msg = self.new_server_msg(
                MessageType::Status,
                &format!("You joined the {room_name} room"),
            );
            self.send_client_msg(session_id, msg);

            // notify all users within the room the that
            // session has connected to the room
            let join_content = format!("{username} joined {room_name} room");

            let msg = self.new_server_msg(MessageType::Status, &join_content);
            self.broadcast(room_name, msg, session_id);
        }

        // notify all users with new user list
        // send message to client that joined room
        let user_list_message =
            self.new_server_msg(MessageType::UserList, &self.list_users(room_name).join(","));

        self.broadcast(room_name, user_list_message, 0)
    }

    /// Helper method used for a web socket session to leave all the rooms
    /// they are currently connected to, it should only be one room
    /// all users within that room are notified that the session has
    /// left the room
    pub fn leave_all_rooms(&mut self, session_id: SessionId, username: &str) {
        let mut affected_rooms = Vec::new();

        // remove sessionId from all rooms
        // and get names of rooms the session was in
        // NOTE:
        // should only be ONE room, user can only be in one room
        // at a time
        for (room_name, session_ids) in &self.rooms {
            if session_ids.contains(&session_id) {
                affected_rooms.push(room_name.to_string())
            }
        }

        for room_name in affected_rooms {
            self.leave_room(&room_name, session_id, username);
        }
    }

    fn leave_room(&mut self, room_name: &str, session_id: SessionId, username: &str) {
        // send message to all users that user left room,
        // only if room name is not `lobby`
        if room_name != "lobby" {
            // message for all other users
            let user_left_msg = self.new_server_msg(
                MessageType::Status,
                &format!("{username} left the {room_name} room"),
            );

            self.broadcast(room_name, user_left_msg, session_id);
        }

        // remove the session ID from room
        if let Some(room) = self.rooms.get_mut(room_name) {
            room.remove(&session_id);
        }

        // message for all other users
        // notify all users with new user list
        // send message to client that joined room
        let user_list_message =
            self.new_server_msg(MessageType::UserList, &self.list_users(room_name).join(","));

        self.broadcast(room_name, user_list_message, 0)
    }

    pub fn list_rooms(&self) -> Vec<String> {
        self.rooms
            .iter()
            .map(|room| room.0.to_owned())
            .filter(|room| room != "lobby" && room != "in_game")
            .collect()
    }

    pub fn list_users(&self, room_name: &str) -> Vec<String> {
        let mut usernames = Vec::new();
        if let Some(room) = self.rooms.get(room_name) {
            for session_id in room.iter() {
                if let Some((username, _)) = self.sessions.get(session_id) {
                    usernames.push(username.clone())
                }
            }
        }
        usernames
    }

    // ---
    // End Room Methods
    // ---

    // ---
    // Game methods
    // ---

    pub fn new_game(&mut self, session_id: SessionId, username: &str) {
        self.leave_all_rooms(session_id, username);

        self.join_room("in_game", session_id, username);

        self.game_manager.new_game(username, session_id);

        self.broadcast_games();
    }

    pub fn leave_game(&mut self, game_id: &str, session_id: SessionId) {
        // notify opponent of leaving game
        // NOTE:
        // Must get opponent ID before leaving game
        // otherwise cannot find game that the user is leaving
        // from and the cannot find opponent id within that game
        let opponent_id = self.game_manager.opponent_id(game_id, session_id);
        self.game_manager.leave_game(game_id, session_id);
        let msg = self.new_server_msg(MessageType::Status, "Opponent has left the game");
        self.send_client_msg(opponent_id, msg);

        self.broadcast_games();
    }

    pub fn leave_all_games(&mut self, session_id: SessionId) {
        // notify opponent of leaving game
        // NOTE:
        // Must get opponent ID before leaving game
        // otherwise cannot find game that the user is leaving
        // from and the cannot find opponent id within that game
        let rooms: Vec<String> = self
            .game_manager
            .games()
            .into_iter()
            .map(|game| game.0.to_string())
            .collect();

        for game_id in rooms {
            let opponent_id = self.game_manager.opponent_id(&game_id, session_id);
            self.game_manager.leave_game(&game_id, session_id);
            let msg = self.new_server_msg(MessageType::Status, "Opponent has left the game");
            self.send_client_msg(opponent_id, msg);
        }

        self.broadcast_games();
    }

    pub fn join_game(&mut self, session_id: SessionId, game_id: &str, username: &str) {
        // set active room on server as `lobby`
        self.join_room("lobby", session_id, username);
        self.game_manager.join_game(game_id, session_id);

        // notify opponent of joining game
        let opponent_id = self.game_manager.opponent_id(game_id, session_id);
        let msg = self.new_server_msg(MessageType::Status, "Opponent joined the game");
        self.send_client_msg(opponent_id, msg);

        self.broadcast_games();
    }

    pub fn send_game_move(&mut self, game_id: &str, move_str: &str, session_id: SessionId) {
        // notify opponent of joining game
        let opponent_id = self.game_manager.opponent_id(game_id, session_id);
        let msg = self.new_server_msg(MessageType::GameMove, move_str);
        self.send_client_msg(opponent_id, msg);
    }

    pub fn delete_game(&mut self, game_id: &str) {
        self.game_manager.delete_game(game_id);

        // update all clients `lobby` of new game list
        // for available game and all games
        self.broadcast_games();
    }

    pub fn list_games(&self) -> HashMap<String, SessionGame> {
        self.game_manager.games().clone()
    }

    pub fn available_games(&self) -> Vec<String> {
        self.game_manager.available_games()
    }

    pub fn all_games(&self) -> Vec<String> {
        self.game_manager.all_games()
    }

    // ---
    // Private methods
    // ---
    fn new_server_msg(&self, msg_type: MessageType, content: &str) -> Message {
        Message {
            msg_type,
            from_id: 0,
            username: "server".to_string(),
            content: content.to_string(),
        }
    }

    /// Broadcast all available games,
    /// `MessageType::AvailableGameList` and `MessageType::AllGameList`
    /// to all clients connected to the `lobby` or `in_game` room
    fn broadcast_games(&self) {
        // broadcast new available game list
        let available_game_msg = self.new_server_msg(
            MessageType::AvailableGameList,
            &self.available_games().join(","),
        );

        let all_game_msg =
            self.new_server_msg(MessageType::AllGameList, &self.all_games().join(","));

        for room in ["lobby", "in_game"] {
            self.broadcast(room, available_game_msg.clone(), 0);
            // broadcast new available game list
            self.broadcast(room, all_game_msg.clone(), 0)
        }
    }
}
