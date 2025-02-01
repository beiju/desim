use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEventData {
    pub last_update: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEvent {
    pub timestamp: DateTime<Utc>,
    pub game_id: Uuid,
    pub data: GameEventData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub data: Vec<GameEvent>,
}

static GAME_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/s04d110.json"
));

pub fn load_games() -> serde_json::Result<Vec<Game>> {
    let game = serde_json::from_str(GAME_JSON)?;
    Ok(vec![game])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_game_succeeds() {
        assert!(load_games().is_ok());
    }
}
