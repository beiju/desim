#[macro_use]
extern crate rocket;
mod engine;
mod event_parser;
mod game_log;
mod rng;
mod thresholds;

use crate::engine::RollConstrains;
use crate::thresholds::Thresholds;
use chrono::{DateTime, TimeDelta, Timelike, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use itertools::Itertools;
use rocket::{response, Request, Response};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use thiserror::Error;

// Nominal tick duration is 5 seconds, but our timestamps are post-network-delay so there is
// definite jitter there
const MIN_EXPECTED_TICK_DURATION: TimeDelta = TimeDelta::seconds(3);

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
    #[error("Couldn't deserialize game: {0:?}")]
    DeserializeGameFailed(serde_json::Error),

    #[error("There were no game events on this day")]
    NoGameEventsThisDay,
}

impl<'r, 'o: 'r> response::Responder<'r, 'o> for DesimError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        #[derive(Serialize)]
        struct ErrContext<'a> {
            header: &'a str,
            body: Option<String>,
        }

        let context = match self {
            DesimError::DeserializeGameFailed(err) => ErrContext {
                header: "Failed to deserialize game",
                body: Some(err.to_string()),
            },
            DesimError::NoGameEventsThisDay => ErrContext {
                header: "No game events this day",
                body: None,
            },
        };
        let template = Template::render("error", context);
        Response::build_from(template.respond_to(req)?).ok()
    }
}

#[get("/")]
fn index(th: &rocket::State<Thresholds>) -> Result<Template, DesimError> {
    let game_data = game_log::load_games().map_err(DesimError::DeserializeGameFailed)?;
    // TODO Load from config file
    let xs128p_state = (15344562644745423164, 10882960106955666841);
    let mut rng = rng::Rng::new(xs128p_state, 23);
    // This appears to be due to the convention resim uses to report rng states
    rng.step(1);
    // These would be attributed to Let's Go, but chron missed the Let's Go on the game I'm
    // currently hard-coding
    rng.step(2);

    let mut all_events = game_data
        .into_iter()
        .flat_map(|game| game.data)
        .collect_vec();
    all_events.sort_by_key(|game| game.timestamp);

    let first_event = all_events.first().ok_or(DesimError::NoGameEventsThisDay)?;
    // Clone some simple data out of first_event so it can be dropped and all_events can be
    // mutably borrowed
    let season = first_event.data.season;
    let day = first_event.data.day;
    // To get the theoretical game start, take the timestamp of the first event and zero it out
    // to the hour mark
    let game_start_timestamp = first_event
        .timestamp
        .with_nanosecond(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_minute(0)
        .unwrap();

    #[derive(Serialize)]
    struct RollContext {
        outcome: RollConstraintOutcome,
        description: String,
        roll: f64,
    }

    #[derive(Serialize)]
    struct EventContext {
        game_label: String,
        description: String,
        errors: Vec<String>,
        warnings: Vec<String>,
        rolls: Vec<RollContext>,
    }

    #[derive(Serialize)]
    struct TickContext {
        tick_index: usize,
        tick_timestamp: DateTime<Utc>,
        time_since_start: String,
        errors: Vec<String>,
        warnings: Vec<String>,
        events: Vec<EventContext>,
    }

    let mut prev_tick_timestamp = None;
    let ticks = all_events
        .into_iter()
        .chunk_by(|game| game.timestamp)
        .into_iter()
        .enumerate()
        .map(|(i, (tick_timestamp, tick_events))| {
            let tick_events = tick_events.collect_vec();

            let time_since_start = tick_timestamp - game_start_timestamp;
            let time_since_start_display = HumanTime::from(time_since_start).to_text_en(Accuracy::Precise, Tense::Present);

            let errors = tick_events.iter()
                .duplicates_by(|g| g.game_id)
                .map(|duplicate| {
                    format!("Contains multiple events for game {}", duplicate.game_id)
                })
                .collect_vec();

            let warnings = prev_tick_timestamp
                .and_then(|prev_tick_timestamp| {
                    let tick_duration = tick_timestamp - prev_tick_timestamp;
                    (tick_duration < MIN_EXPECTED_TICK_DURATION).then(|| {
                        format!("Tick duration was only {tick_duration} (expected at least {MIN_EXPECTED_TICK_DURATION})")
                    })
                })
                .into_iter()
                .collect_vec();
            prev_tick_timestamp = Some(tick_timestamp);

            let events = tick_events.into_iter()
                .map(|event| {
                    let game_label = format!("{} @ {}", event.data.home_team_nickname, event.data.away_team_nickname);
                    let description = event.data.last_update.clone();
                    match event_parser::parse_event(event) {
                        Ok(event) => {
                            let rolls = engine::rolls_for_event(&event, &th).into_iter()
                                .map(|roll| {
                                    let roll_outcome = rng.value();
                                    rng.step(1);
                                    let (outcome, description) = match roll.constraints {
                                        RollConstrains::Unconstrained { description } => {
                                            (RollConstraintOutcome::TrivialSuccess, format!("{description}: Unconstrained ({roll_outcome})"))
                                        }
                                        RollConstrains::BelowThreshold { threshold, negative_description, positive_description } => {
                                            if roll_outcome < threshold {
                                                (RollConstraintOutcome::Success, format!("{positive_description} ({roll_outcome} < {threshold})"))
                                            } else {
                                                (RollConstraintOutcome::Failure, format!("{negative_description} ({roll_outcome} !< {threshold})"))
                                            }
                                        }
                                        RollConstrains::AboveThreshold { threshold, negative_description, positive_description } => {
                                            if roll_outcome > threshold {
                                                (RollConstraintOutcome::Success, format!("{positive_description} ({roll_outcome} > {threshold})"))
                                            } else {
                                                (RollConstraintOutcome::Failure, format!("{negative_description} ({roll_outcome} !> {threshold})"))
                                            }
                                        }
                                        RollConstrains::Unused { description } => {
                                            (RollConstraintOutcome::Unused, format!("{description} (Unused: {roll_outcome})"))
                                        }
                                    };
                                    RollContext {
                                        outcome,
                                        description,
                                        roll: roll_outcome,
                                    }
                                })
                                .collect_vec();

                            EventContext {
                                game_label,
                                description,
                                errors: vec![],
                                warnings: vec![],
                                rolls,
                            }
                        }
                        Err(err) => {
                            let errors = vec![
                                format!("Failed to parse event: {}", err),
                            ];
                            EventContext {
                                game_label,
                                description,
                                errors,
                                warnings: vec![],
                                rolls: vec![],
                            }
                        }
                    }
                })
                .collect_vec();

            TickContext {
                tick_index: i,
                tick_timestamp,
                time_since_start: time_since_start_display,
                errors,
                warnings,
                events,
            }
        })
        .collect_vec();

    Ok(Template::render(
        "index",
        context! {
            season: season,
            day: day,
            ticks: ticks
        },
    ))
}

#[launch]
fn rocket() -> _ {
    let th = Thresholds::load().expect("Failed to load thresholds");

    rocket::build()
        .manage(th)
        .mount("/static", rocket::fs::FileServer::from("static"))
        .mount("/", routes![index])
        .attach(Template::fairing())
}
