use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerItem {
    pub entity_id: Uuid,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
    pub data: value::Value,
}

pub type ChroniclerItems = Vec<ChroniclerItem>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerResponse {
    pub next_page: Option<String>,
    pub items: ChroniclerItems,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerGameUpdate {
    pub game_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub data: value::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerGameUpdatesResponse {
    pub next_page: Option<String>,
    pub data: Vec<ChroniclerGameUpdate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerGame {
    pub game_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub data: value::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerGamesResponse {
    pub next_page: Option<String>,
    pub data: Vec<ChroniclerGame>,
}
