use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt::Debug;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerItem<ItemType: Debug> {
    pub entity_id: Uuid,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
    pub data: ItemType,
}

pub type ChroniclerItems<ItemType> = Vec<ChroniclerItem<ItemType>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerResponse<ItemType: Debug> {
    pub next_page: Option<String>,
    pub items: ChroniclerItems<ItemType>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerGameUpdate {
    pub game_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub data: ChroniclerGameUpdateData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerTeam {
    pub id: Uuid,
    pub card: i64, // TODO enum
    pub bench: Vec<Uuid>,
    pub emoji: String,
    pub state: serde_json::Value,
    pub lineup: Vec<Uuid>,
    pub slogan: String,
    pub bullpen: Vec<Uuid>,
    pub stadium: Uuid,
    pub e_density: f64,
    pub full_name: String,
    pub game_attr: Vec<String>, // i think
    pub location: String,
    pub nickname: String,
    pub perm_attr: Vec<String>, // i think
    pub rotation: Vec<Uuid>,
    pub seas_attr: Vec<String>, // i think
    pub week_attr: Vec<String>, // i think
    pub e_velocity: f64,
    pub main_color: String,
    pub shame_runs: i64, // or float?
    pub shorthand: String,
    pub win_streak: i64,
    pub im_position: f64,
    pub team_spirit: i64,
    pub total_shames: i64,
    pub rotation_slot: i64,
    pub season_shames: i64,
    pub championships: i64,
    pub total_shamings: i64,
    pub season_shamings: i64,
    pub secondary_color: String,
    pub tournament_wins: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChroniclerPlayer {
    pub id: Uuid,
    pub bat: String,
    pub fate: i64,
    pub name: String,
    pub soul: i64,
    pub armor: String,
    pub blood: i64, // TODO enum
    pub moxie: f64,
    pub state: serde_json::Value,
    pub coffee: i64, // TODO enum
    pub ritual: String,
    pub buoyancy: f64,
    pub cinnamon: f64,
    pub coldness: f64,
    pub deceased: bool,
    pub divinity: f64,
    pub e_density: f64,
    pub perm_attr: Vec<String>,
    pub seas_attr: Vec<String>,
    pub week_attr: Vec<String>,
    pub chasiness: f64,
    pub evolution: i64,
    pub hit_streak: i64,
    pub martyrdom: f64,
    pub base_thirst: f64,
    pub indulgence: f64,
    pub musclitude: f64,
    pub tragicness: f64,
    pub omniscience: f64,
    pub patheticism: f64,
    pub suppression: f64,
    pub continuation: f64,
    pub league_team_id: Option<Uuid>,
    pub ruthlessness: f64,
    pub total_fingers: i64,
    pub watchfulness: f64,
    pub defense_rating: f64,
    pub hitting_rating: f64,
    pub laserlikeness: f64,
    pub overpowerment: f64,
    pub peanut_allergy: bool,
    pub tenaciousness: f64,
    pub thwackability: f64,
    pub anticapitalism: f64,
    pub ground_friction: f64,
    pub pitching_rating: f64,
    pub pressurization: f64,
    pub consecutive_hits: i64,
    pub unthwackability: f64,
    pub shakespearianism: f64,
    pub tournament_team_id: Option<Uuid>,
    pub baserunning_rating: f64,
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
    pub away_team: Uuid,
    pub home_odds: f64,
    pub home_outs: i64,
    pub home_team: Uuid,
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
