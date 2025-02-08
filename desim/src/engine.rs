use crate::rng::Rng;
use crate::rolls::{rolls_for_update, RollConstrains, RollSpec};
use crate::thresholds::Thresholds;
use crate::{sim, update_parser, RollConstraintOutcome};
use blaseball_api::ChroniclerGameUpdate;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// Engine's job is to:
// - ingest a (hopefully chronological) stream of updates
// - group them into ticks based on chronicler time
// - match all the rolls that happened that tick with rng outcomes, correcting
//   for game update order as it does
// - produce <Thing>Context objects describing every game, tick, and roll for
//   the purpose of rendering
pub struct Engine {
    rng: Rng,
    active_games: HashMap<Uuid, sim::Game>,
    // This stores a list of game updates with identical timestamps which have
    // not yet been processed. As soon as an update with a future timestamp is
    // received, the pending updates are processed. It's TBD what happens to
    // updates that are in this vec when the event stream ends (the safest thing
    // is to drop them, if we can't be sure there weren't any more events that
    // happened on this tick).
    pending_updates: Vec<ChroniclerGameUpdate>,
    // Contains an under-construction DayContext (or None if we haven't started yet)
    current_day: Option<DayContext>,
}

#[derive(Debug, Clone, Error)]
pub enum EngineFatalError {
    // If this ever happens I might add more useful debug info, but I don't
    // think Chron ever returns updates out of order
    #[error(
        "Event out of order: Event with timestamp {new_event_timestamp} came \
        in after event with timestamp {stored_event_timestamp}"
    )]
    EventOutOfOrder {
        stored_event_timestamp: DateTime<Utc>,
        new_event_timestamp: DateTime<Utc>,
    },

    // This is even less likely to happen than events out of order
    #[error(
        "Days out of order: Event for {new_day:?} came \
        in after event for {stored_day:?} from game {in_game}"
    )]
    DaysOutOfOrder {
        stored_day: (i64, i64),
        new_day: (i64, i64),
        in_game: Uuid,
    },
}

#[derive(Serialize)]
struct RollContext {
    outcome: RollConstraintOutcome,
    description: String,
    roll: f64,
}

#[derive(Serialize)]
pub struct GameTickContext {
    game_label: String,
    description: String,
    errors: Vec<String>,
    warnings: Vec<String>,
    rolls: Vec<RollContext>,
}

#[derive(Serialize)]
pub struct TickContext {
    tick_timestamp: DateTime<Utc>,
    games: Vec<GameTickContext>,
}

#[derive(Serialize)]
pub struct DayContext {
    season: i64,
    day: i64,
    ticks: Vec<TickContext>,
}

fn run_roll(
    roll_spec: RollSpec,
    rng: &mut Rng,
) -> RollContext {
    // RNG step-before-value is the convention Resim set
    rng.step(1);
    let roll = rng.value();
    let (outcome, description) = match roll_spec.constraints {
        RollConstrains::Unconstrained { description } => (
            RollConstraintOutcome::TrivialSuccess,
            format!("{description}: Unconstrained ({roll})"),
        ),
        RollConstrains::BelowThreshold {
            threshold,
            negative_description,
            positive_description,
        } => {
            if roll < threshold {
                (
                    RollConstraintOutcome::Success,
                    format!(
                        "{positive_description} ({roll} < {threshold})"
                    ),
                )
            } else {
                (
                    RollConstraintOutcome::Failure,
                    format!(
                        "{negative_description} ({roll} !< {threshold})"
                    ),
                )
            }
        }
        RollConstrains::AboveThreshold {
            threshold,
            negative_description,
            positive_description,
        } => {
            if roll > threshold {
                (
                    RollConstraintOutcome::Success,
                    format!(
                        "{positive_description} ({roll} > {threshold})"
                    ),
                )
            } else {
                (
                    RollConstraintOutcome::Failure,
                    format!(
                        "{negative_description} ({roll} !> {threshold})"
                    ),
                )
            }
        }
        RollConstrains::Unused { description } => (
            RollConstraintOutcome::Unused,
            format!("{description} (Unused: {roll})"),
        ),
    };
    RollContext {
        outcome,
        description,
        roll,
    }
}

fn run_game_tick(
    game: &sim::Game,
    update: ChroniclerGameUpdate,
    th: &Thresholds,
    rng: &mut Rng,
) -> GameTickContext {
    let game_at_tick = game.at_tick(&update);

    let (mut errors, warnings) = game_at_tick.validate(&update);
    let game_label = format!(
        "{} @ {}",
        update.data.away_team_nickname, update.data.home_team_nickname
    );
    match update_parser::parse_update(&update) {
        Ok(parsed_update) => {
            let rolls = rolls_for_update(&parsed_update, th, &game_at_tick)
                .into_iter()
                .map(|roll_spec| run_roll(roll_spec, rng))
                .collect();

            GameTickContext {
                game_label,
                description: update.data.last_update,
                errors,
                warnings,
                rolls,
            }
        }
        Err(err) => {
            errors.push(format!("Parse error: {err}"));
            GameTickContext {
                game_label,
                description: update.data.last_update,
                errors,
                warnings,
                rolls: Vec::new(),
            }
        }
    }
}

impl Engine {
    pub fn new(rng: Rng) -> Engine {
        Engine {
            rng,
            active_games: HashMap::new(),
            pending_updates: Vec::new(),
            current_day: None,
        }
    }

    pub async fn next_update(
        &mut self,
        update: ChroniclerGameUpdate,
        th: &Thresholds,
    ) -> Result<Option<DayContext>, EngineFatalError> {
        // Get any pending update (the first is arbitrarily chosen) to compare
        // its timestamp against the new update
        if let Some(pending_update) = self.pending_updates.first() {
            match update.timestamp.cmp(&pending_update.timestamp) {
                Ordering::Less => {
                    // If the new update is before the pending update, that's an error
                    Err(EngineFatalError::EventOutOfOrder {
                        stored_event_timestamp: pending_update.timestamp,
                        new_event_timestamp: update.timestamp,
                    })
                }
                Ordering::Equal => {
                    // debug!("Appending update for {} to pending updates for {}", update.timestamp, pending_update.timestamp);
                    // If the new update is at the same time as the pending update, store it
                    self.pending_updates.push(update);
                    Ok(None)
                }
                Ordering::Greater => {
                    // If the new update is later than the pending update,
                    // process all pending updates and then store the new one
                    let updates_to_process =
                        std::mem::replace(&mut self.pending_updates, vec![update]);
                    self.tick(updates_to_process, th).await
                }
            }
        } else {
            // If there are no pending updates, just store the new update and continue
            self.pending_updates.push(update);
            Ok(None)
        }
    }

    async fn tick(
        &mut self,
        updates: Vec<ChroniclerGameUpdate>,
        th: &Thresholds,
    ) -> Result<Option<DayContext>, EngineFatalError> {
        // In the future, this will be responsible for figuring out tick order.
        // For now I'm only feeding it ticks with one event at a time.

        let update = updates
            .first()
            .expect("tick() must be called with at least one update");

        // If this event is from the next day, we need to return the current day
        let finished_day = if let Some(in_process_day) = &self.current_day {
            let stored_day = (in_process_day.season, in_process_day.day);
            let new_day = (update.data.season, update.data.day);
            match new_day.cmp(&stored_day) {
                Ordering::Less => {
                    // I would call this very unlikely to happen, but if it
                    // does happen it's an error
                    // TODO this is actually happening, how do I make it output without destroying
                    //   all the other info that's already been output?
                    Err(EngineFatalError::DaysOutOfOrder {
                        stored_day,
                        new_day,
                        in_game: update.game_id,
                    })
                }
                Ordering::Equal => {
                    // If we received a new event for the day in progress,
                    // just continue on as normal
                    Ok(None)
                }
                Ordering::Greater => {
                    // If we received an event for a new day, extract and return
                    // the previous day. Also drop all the `sim::Game`s for the
                    // previous day; they will never be used again
                    self.active_games.clear();
                    Ok(std::mem::replace(&mut self.current_day, None))
                }
            }
        } else {
            // If we don't have a previous day, there can't be a finished one
            Ok(None)
        }?;

        let day = self.current_day.get_or_insert_with(|| {
            debug!(
                "Creating new DayContext for s{}d{}",
                update.data.season + 1,
                update.data.day + 1
            );
            DayContext {
                season: update.data.season,
                day: update.data.day,
                ticks: Vec::new(),
            }
        });

        // Get the timestamp out of `update` now, so it can be dropped and we
        // can move out of `updates`
        let tick_timestamp = update.timestamp;

        // This is where we will have to figure out event order eventually

        // sigh. i want to use iterator methods for this but i can't figure out
        // how to please the borrow checker
        let mut game_updates = Vec::new();
        game_updates.reserve_exact(updates.len());
        for update in updates {
            // Can't use or_insert_with because fetching a game is async
            let game_update = match self.active_games.entry(update.game_id) {
                Entry::Occupied(mut entry) => run_game_tick(entry.get_mut(), update, th, &mut self.rng),
                Entry::Vacant(entry) => {
                    // The first few updates of a game can be skipped and nothing bad happens
                    // (because they don't do any rolls), but if we're starting a game later than
                    // approximately play count 3 something has gone wrong
                    assert!(update.data.play_count < 3);
                    let game_at_tick = sim::Game::from_first_game_update(&update).await;
                    run_game_tick(entry.insert(game_at_tick), update, th, &mut self.rng)
                }
            };

            game_updates.push(game_update);
        }

        day.ticks.push(TickContext {
            tick_timestamp,
            games: game_updates,
        });

        Ok(finished_day)
    }
}
