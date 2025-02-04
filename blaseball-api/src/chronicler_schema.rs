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
    pub data: ChroniclerGameUpdateData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChroniclerGameUpdateData {
    pub id: Uuid,
    pub day: i64,
    pub phase: i64, // TODO enum
    pub rules: Option<Uuid>,
    pub shame: bool,
    pub inning: i64,
    pub season: i64,
    pub weather: i64, // TODO enum
    pub away_odds: f64,
    pub away_outs: i64,
    pub away_team: Option<Uuid>,
    pub home_odds: f64,
    pub home_outs: i64,
    pub home_team: Option<Uuid>,
    pub outcomes: Vec<String>,
    pub away_balls: i64,
    pub away_bases: i64,
    pub away_score: f64,
    pub finalized: bool,
    pub game_start: bool,
    pub home_balls: i64,
    pub home_bases: i64,
    pub home_score: f64,
    pub play_count: i64,
    pub stadium_id: Option<Uuid>,
    pub statsheet: Option<Uuid>,
    pub at_bat_balls: i64,
    pub away_batter: Option<Uuid>,
    pub home_batter: Option<Uuid>,
    pub last_update: String,
    pub tournament: i64,
    pub away_pitcher: Option<Uuid>,
    pub away_strikes: i64,
    pub base_runners: Vec<Uuid>,
    pub home_pitcher: Option<Uuid>,
    pub home_strikes: i64,
    pub repeat_count: i64,
    pub score_ledger: String,
    pub score_update: String,
    pub series_index: i64,
    pub terminology: Uuid,
    pub top_of_inning: bool,
    pub at_bat_strikes: i64,
    pub away_team_name: String,
    pub game_complete: bool,
    pub home_team_name: String,
    pub is_postseason: bool,
    pub is_title_match: bool,
    pub series_length: i64,
    pub away_batter_mod: String,
    pub away_team_color: String,
    pub away_team_emoji: String,
    pub bases_occupied: Vec<i64>,
    pub home_batter_mod: String,
    pub home_team_color: String,
    pub home_team_emoji: String,
    pub away_batter_name: String,
    pub away_pitcher_mod: String,
    pub base_runner_mods: Vec<String>,
    pub game_start_phase: i64,
    pub half_inning_outs: i64,
    pub home_batter_name: String,
    pub home_pitcher_mod: String,
    pub new_inning_phase: i64,
    pub top_inning_score: f64,
    pub away_pitcher_name: String,
    pub base_runner_names: Vec<String>,
    pub baserunner_count: i64,
    pub half_inning_score: f64,
    pub home_pitcher_name: String,
    pub away_team_nickname: String,
    pub home_team_nickname: String,
    pub secret_baserunner: Option<Uuid>,
    pub bottom_inning_score: i64,
    pub away_team_batter_count: i64,
    pub home_team_batter_count: i64,
    pub away_team_secondary_color: String,
    pub home_team_secondary_color: String,
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
