use crate::session::SessionId;
use std::collections::HashMap;

type SessionGameId = String;

#[derive(Debug, Clone)]
pub struct SessionGame {
    game_id: SessionGameId,
    white: Option<SessionId>,
    black: Option<SessionId>,
    started: bool,
}

impl SessionGame {
    pub fn new(game_id: SessionGameId, white_session_id: SessionId) -> Self {
        Self {
            game_id,
            white: Some(white_session_id),
            black: None,
            started: false,
        }
    }

    /// main join game method
    /// if there are 2 players in the game
    /// the game is set to `started` = true
    pub fn join_game(&mut self, black: SessionId) {
        self.black = Some(black);

        if self.num_players() == 2 {
            self.started = true;
        }
    }

    /// main leave game method
    /// if after the player leaves there are no
    /// more players in the game, the game is removed
    /// from session manager game list
    pub fn leave_game(&mut self, session_id: SessionId) {
        // check if 2 players in game
        // remove player two from game
        if self.black_id() == session_id {
            self.black = None;
        }
        // remove player two from game
        if self.white_id() == session_id {
            self.white = None;
        }
    }

    /// gets number of current players in game
    pub fn num_players(&self) -> i8 {
        let mut num: i8 = 0;

        if self.white.is_some() {
            num += 1;
        }

        if self.black.is_some() {
            num += 1;
        }
        num
    }

    /// get the ID of the opponent, ie. the opposite
    /// ID of the one which is passed in as an argument
    pub fn opponent_id(&self, session_id: SessionId) -> SessionId {
        if self.white_id() == session_id {
            return self.black_id();
        }

        if self.black_id() == session_id {
            return self.white_id();
        }

        0
    }

    /// If the game is not started the game is joinable
    /// otherwise the game is not joinable
    pub fn is_joinable(&self) -> bool {
        if self.started {
            return false;
        }
        true
    }

    /// get ID of black piece player
    pub fn black_id(&self) -> SessionId {
        if let Some(id) = self.black {
            return id;
        }
        0
    }

    /// get ID of white piece player
    pub fn white_id(&self) -> SessionId {
        if let Some(id) = self.white {
            return id;
        }
        0
    }
}

#[derive(Debug)]
pub struct GameManager {
    games: HashMap<SessionGameId, SessionGame>,
}

impl GameManager {
    pub fn new() -> Self {
        let games = HashMap::new();

        Self { games }
    }

    pub fn new_game(&mut self, username: &str, session_id: SessionId) {
        let game = SessionGame::new(username.to_string(), session_id);
        self.games.insert(username.to_string(), game);
    }

    pub fn get_game(&mut self, game_id: &str) -> Option<&mut SessionGame> {
        self.games.get_mut(game_id)
    }

    pub fn get_games(&self) -> &HashMap<SessionGameId, SessionGame> {
        &self.games
    }

    pub fn join_game(&mut self, game_id: &str, session_id: SessionId) {
        // TODO:
        // handle case if game is already full
        if let Some(game) = self.get_game(game_id) {
            if game.is_joinable() {
                game.join_game(session_id)
            }
        }
    }

    pub fn leave_game(&mut self, game_id: &str, session_id: SessionId) {
        if let Some(game) = self.games.get_mut(game_id) {
            game.leave_game(session_id);

            // if last player leave game
            // remove game from game_manager list
            if game.num_players() == 0 {
                self.games.remove(game_id);
            }
        }
    }

    pub fn opponent_id(&self, game_id: &str, session_id: SessionId) -> SessionId {
        if let Some(game) = self.games.get(game_id) {
            game.opponent_id(session_id)
        } else {
            0
        }
    }

    pub fn delete_game(&mut self, game_id: &str) {
        self.games.remove(game_id);
    }

    pub fn available_games(&self) -> Vec<String> {
        // build vector of strings of available games
        self.games
            .iter()
            .map(|game| game.1)
            .collect::<Vec<&SessionGame>>()
            // all games collected
            .into_iter()
            .filter(|game| game.is_joinable())
            // only games that are joinable filtered
            .into_iter()
            .map(|game| game.game_id.clone())
            // collected filtered games as string of game names
            .collect::<Vec<String>>()
    }

    pub fn all_games(&self) -> Vec<String> {
        // build vector of strings of available games
        self.games
            .iter()
            .map(|game| game.1.game_id.clone())
            .collect::<Vec<String>>()
    }
}
