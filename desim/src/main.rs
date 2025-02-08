#[macro_use]
extern crate rocket;
mod engine;
mod fragments;
mod rng;
mod rolls;
mod sim;
mod thresholds;
mod update_parser;

use crate::engine::{Engine, EngineFatalError};
use crate::fragments::{load_fragments, Fragments};
use crate::thresholds::Thresholds;
use blaseball_api::chronicler;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use rocket::futures::StreamExt;
use rocket::http::uri::fmt::{Formatter, FromUriParam, UriDisplay};
use rocket::request::FromParam;
use rocket::{response, Request, Response};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::future;
use std::pin::pin;
use thiserror::Error;

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum RollConstraintOutcome {
    /// Succeeded because we don't know enough to confirm that it failed
    TrivialSuccess,
    /// We're confident that this roll obeyed the threshold
    Success,
    /// We're confident that this roll violated the threshold
    Failure,
    /// We're confident that the sim doesn't use the outcome of this roll
    Unused,
}

#[derive(Error, Debug)]
pub enum DesimError {
    #[error("Could not find fragment {0}")]
    UnknownFragment(usize),

    #[error("Couldn't deserialize game: {0:?}")]
    DeserializeGameFailed(serde_json::Error),

    #[error("There were no game events on this day")]
    NoGameEventsThisDay,

    #[error(transparent)]
    EngineError(#[from] EngineFatalError),
}

impl<'r, 'o: 'r> response::Responder<'r, 'o> for DesimError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        #[derive(Serialize)]
        struct ErrContext<'a> {
            header: &'a str,
            body: Option<String>,
        }

        let context = match self {
            DesimError::UnknownFragment(err) => ErrContext {
                header: "Nonexistent fragment requested",
                body: Some(err.to_string()),
            },
            DesimError::DeserializeGameFailed(err) => ErrContext {
                header: "Failed to deserialize game",
                body: Some(err.to_string()),
            },
            DesimError::NoGameEventsThisDay => ErrContext {
                header: "No game events this day",
                body: None,
            },
            DesimError::EngineError(err) => ErrContext {
                header: "Engine error",
                body: Some(err.to_string()),
            },
        };
        let template = Template::render("error", context);
        Response::build_from(template.respond_to(req)?).ok()
    }
}

struct DateTimeParam(DateTime<Utc>);

impl<'a> FromParam<'a> for DateTimeParam {
    type Error = chrono::ParseError;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Ok(Self(DateTime::parse_from_rfc3339(param)?.to_utc()))
    }
}

impl UriDisplay<rocket::http::uri::fmt::Path> for DateTimeParam {
    fn fmt(&self, f: &mut Formatter<'_, rocket::http::uri::fmt::Path>) -> std::fmt::Result {
        UriDisplay::fmt(&self.0.to_rfc3339(), f)
    }
}

impl FromUriParam<rocket::http::uri::fmt::Path, &DateTime<Utc>> for DateTimeParam {
    type Target = DateTimeParam;

    fn from_uri_param(param: &DateTime<Utc>) -> Self::Target {
        DateTimeParam(*param)
    }
}

#[get("/")]
fn index(fragments: &rocket::State<Fragments>) -> Template {
    #[derive(Serialize)]
    struct FragmentContext<'route> {
        title: &'route str,
        uri: rocket::http::uri::Origin<'route>,
    }

    let fragments_view = fragments
        .iter()
        .enumerate()
        .map(|(fragment_index, fragment)| FragmentContext {
            title: &fragment.label,
            uri: uri!(fragment(fragment_index)),
        })
        .collect_vec();

    Template::render(
        "index",
        context! {
            fragments: fragments_view,
        },
    )
}

#[get("/fragment/<fragment_index>")]
async fn fragment(
    fragment_index: usize,
    fragments: &rocket::State<Fragments>,
    th: &rocket::State<Thresholds>,
) -> Result<Template, DesimError> {
    let fragment = fragments
        .get(fragment_index)
        .ok_or(DesimError::UnknownFragment(fragment_index))?;

    // Gather data
    let mut game_updates = pin!(chronicler::game_updates(fragment.start_time)
        .take_while(|update| future::ready(update.timestamp < fragment.end_time)));
    let mut rng = fragment.rng.clone();
    rng.step(fragment.initial_step);

    // Start the engine
    let mut engine = Engine::new(rng);
    
    // I am implementing a poor man's group_by. The man is very poor.
    let mut days = Vec::new();
    while let Some(update) = game_updates.next().await {
        debug!(
            "Processing game update for s{}d{} game {} at time {}",
            update.data.season + 1,
            update.data.day + 1,
            update.data.id,
            update.timestamp,
        );
        if let Some(new_day) = engine.next_update(update, th).await? {
            days.push(new_day);
        }
    }

    Ok(Template::render(
        "fragment",
        context! {
            fragment_label: &fragment.label,
            days: days,
        },
    ))

    // let first_update = game_updates
    //     .as_mut()
    //     .peek()
    //     .await
    //     .ok_or(DesimError::NoGameEventsThisDay)?;
    // let game_id = first_update.game_id;
    // let season = first_update.data.season;
    // let day = first_update.data.day;
    // // To get the theoretical game start, take the timestamp of the first event and zero it out
    // // to the hour mark
    // let game_start_timestamp = first_update
    //     .timestamp
    //     .with_nanosecond(0)
    //     .unwrap()
    //     .with_second(0)
    //     .unwrap()
    //     .with_minute(0)
    //     .unwrap();
    //
    // let game = sim::Game::from_first_game_update(first_update).await;
    //
    // let mut all_events = game_updates
    //     .take_while(|event| future::ready(event.data.id == game_id))
    //     .collect::<Vec<_>>()
    //     .await;
    // all_events.sort_by_key(|game| game.timestamp);
    //
    // #[derive(Serialize)]
    // struct RollContext {
    //     outcome: RollConstraintOutcome,
    //     description: String,
    //     roll: f64,
    // }
    //
    // #[derive(Serialize)]
    // struct EventContext {
    //     game_label: String,
    //     description: String,
    //     errors: Vec<String>,
    //     warnings: Vec<String>,
    //     rolls: Vec<RollContext>,
    // }
    //
    // #[derive(Serialize)]
    // struct TickContext {
    //     tick_index: usize,
    //     tick_timestamp: DateTime<Utc>,
    //     time_since_start: String,
    //     errors: Vec<String>,
    //     warnings: Vec<String>,
    //     events: Vec<EventContext>,
    // }
    //
    // let mut prev_tick_timestamp = None;
    // let ticks = all_events
    //     .into_iter()
    //     .chunk_by(|game| game.timestamp)
    //     .into_iter()
    //     .enumerate()
    //     .map(|(i, (tick_timestamp, tick_events))| {
    //         let tick_events = tick_events.collect_vec();
    //
    //         let time_since_start = tick_timestamp - game_start_timestamp;
    //         let time_since_start_display = HumanTime::from(time_since_start).to_text_en(Accuracy::Precise, Tense::Present);
    //
    //         let mut errors = tick_events.iter()
    //             .duplicates_by(|g| g.game_id)
    //             .map(|duplicate| {
    //                 format!("Contains multiple events for game {}", duplicate.game_id)
    //             })
    //             .collect_vec();
    //
    //         let mut warnings = prev_tick_timestamp
    //             .and_then(|prev_tick_timestamp| {
    //                 let tick_duration = tick_timestamp - prev_tick_timestamp;
    //                 (tick_duration < MIN_EXPECTED_TICK_DURATION).then(|| {
    //                     format!("Tick duration was only {tick_duration} (expected at least {MIN_EXPECTED_TICK_DURATION})")
    //                 })
    //             })
    //             .into_iter()
    //             .collect_vec();
    //         prev_tick_timestamp = Some(tick_timestamp);
    //
    //         let events = tick_events.into_iter()
    //             .map(|update| {
    //                 let game_label = format!("{} @ {}", update.data.home_team_nickname, update.data.away_team_nickname);
    //                 let description = update.data.last_update.clone();
    //                 match update_parser::parse_update(&update) {
    //                     Ok(parsed_event) => {
    //                         let game_at_tick = game.at_tick(&update);
    //                         let (game_errors, game_warnings) = game_at_tick.validate(&update);
    //                         errors.extend(game_errors);
    //                         warnings.extend(game_warnings);
    //
    //                         let rolls = rolls::rolls_for_event(&parsed_event, &th, &game_at_tick).into_iter()
    //                             .map(|roll| {
    //                                 // RNG step-before-value is the convention Resim set
    //                                 rng.step(1);
    //                                 let roll_outcome = rng.value();
    //                                 let (outcome, description) = match roll.constraints {
    //                                     RollConstrains::Unconstrained { description } => {
    //                                         (RollConstraintOutcome::TrivialSuccess, format!("{description}: Unconstrained ({roll_outcome})"))
    //                                     }
    //                                     RollConstrains::BelowThreshold { threshold, negative_description, positive_description } => {
    //                                         if roll_outcome < threshold {
    //                                             (RollConstraintOutcome::Success, format!("{positive_description} ({roll_outcome} < {threshold})"))
    //                                         } else {
    //                                             (RollConstraintOutcome::Failure, format!("{negative_description} ({roll_outcome} !< {threshold})"))
    //                                         }
    //                                     }
    //                                     RollConstrains::AboveThreshold { threshold, negative_description, positive_description } => {
    //                                         if roll_outcome > threshold {
    //                                             (RollConstraintOutcome::Success, format!("{positive_description} ({roll_outcome} > {threshold})"))
    //                                         } else {
    //                                             (RollConstraintOutcome::Failure, format!("{negative_description} ({roll_outcome} !> {threshold})"))
    //                                         }
    //                                     }
    //                                     RollConstrains::Unused { description } => {
    //                                         (RollConstraintOutcome::Unused, format!("{description} (Unused: {roll_outcome})"))
    //                                     }
    //                                 };
    //                                 RollContext {
    //                                     outcome,
    //                                     description,
    //                                     roll: roll_outcome,
    //                                 }
    //                             })
    //                             .collect_vec();
    //
    //                         EventContext {
    //                             game_label,
    //                             description,
    //                             errors: vec![],
    //                             warnings: vec![],
    //                             rolls,
    //                         }
    //                     }
    //                     Err(err) => {
    //                         let errors = vec![
    //                             format!("Failed to parse event: {}", err),
    //                         ];
    //                         EventContext {
    //                             game_label,
    //                             description,
    //                             errors,
    //                             warnings: vec![],
    //                             rolls: vec![],
    //                         }
    //                     }
    //                 }
    //             })
    //             .collect_vec();
    //
    //         TickContext {
    //             tick_index: i,
    //             tick_timestamp,
    //             time_since_start: time_since_start_display,
    //             errors,
    //             warnings,
    //             events,
    //         }
    //     })
    //     .collect_vec();
    //
    // Ok(Template::render(
    //     "game",
    //     context! {
    //         season: season,
    //         day: day,
    //         ticks: ticks
    //     },
    // ))
}

#[launch]
fn rocket() -> _ {
    let fragments = load_fragments().expect("Failed to load fragments");
    let th = Thresholds::load().expect("Failed to load thresholds");

    let static_path = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
    rocket::build()
        .manage(fragments)
        .manage(th)
        .mount("/static", rocket::fs::FileServer::from(static_path))
        .mount("/", routes![index, fragment])
        .attach(Template::fairing())
}
