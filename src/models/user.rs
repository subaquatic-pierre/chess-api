use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

#[derive(Debug)]
pub struct UserInfo {
    pub nickname: String,
}

impl UserInfo {
    pub fn new(user_name: &str) -> Self {
        Self {
            nickname: user_name.to_owned(),
        }
    }
}

impl Default for UserInfo {
    fn default() -> Self {
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();

        let nickname = format!("User-{}", rand_string);

        UserInfo { nickname }
    }
}
