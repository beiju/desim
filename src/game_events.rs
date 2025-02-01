use serde::{Deserialize};

#[derive(Deserialize)]
pub struct Game {
    
}

static GAME_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/s04d110.json"));

pub fn load_game() -> serde_json::Result<Game> {
    serde_json::from_str(GAME_JSON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_game_succeeds() {
        assert!(load_game().is_ok());
    }
}