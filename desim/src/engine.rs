use crate::fragments::{CheckRoll, RollStream};
use crate::rng::Rng;
use crate::rolls::{rolls_for_update, RollData, RollPurpose, RollUsage};
use crate::thresholds::Thresholds;
use crate::{sim, update_parser};
use blaseball_api::{Chronicler, ChroniclerGameUpdate};
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
    // Rolls from resim to check against
    check_rolls: Option<RollStream>,
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
    tick_number: usize,
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

    #[error(
        "Check rolls were provided, but there were not enough to run the whole \
        fragment"
    )]
    RanOutOfCheckRolls,
}

#[derive(Serialize)]
struct FloatDigitsMismatchContext {
    pub mine: f64,
    pub resim: f64,
    pub matching_digits: String,
    pub mismatching_digits: String,
    pub extra_digits: String,
}

#[derive(Serialize)]
#[serde(tag = "match")]
enum FloatMatchContext {
    Matches,
    Mismatch(FloatDigitsMismatchContext),
}

#[derive(Serialize)]
#[serde(tag = "match")]
enum OptionBoolMatchContext {
    Matches,
    MineMissingResimExists { resim: bool },
    MineExistsResimMissing { mine: bool },
    Mismatch { mine: bool, resim: bool },
}

#[derive(Serialize)]
#[serde(tag = "match")]
enum OptionFloatMatchContext {
    Matches,
    MineMissingResimExists { resim: f64 },
    MineExistsResimMissing { mine: f64 },
    Mismatch(FloatDigitsMismatchContext),
}

#[derive(Serialize)]
#[serde(tag = "match")]
enum RollPurposeMatchContext {
    Matches,
    Mismatch { mine: String, resim: String },
}

#[derive(Serialize)]
enum ResimMatchContext {
    Threshold {
        rolls: FloatMatchContext,
        purpose: RollPurposeMatchContext,
        passed: OptionBoolMatchContext,
        threshold: OptionFloatMatchContext,
    },
    Choice {
        rolls: FloatMatchContext,
        purpose: RollPurposeMatchContext,
    },
}

#[derive(Serialize)]
struct RollContext {
    purpose: String,
    rng_state: String,
    roll: f64,
    usage: RollUsage,
    resim_mismatch: Option<ResimMatchContext>,
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
    tick_number: usize,
    tick_timestamp: DateTime<Utc>,
    games: Vec<GameTickContext>,
}

#[derive(Serialize)]
pub struct DayContext {
    season: i64,
    day: i64,
    ticks: Vec<TickContext>,
}

// Surprised this isn't built-in or available from a crate (as far as I can find)
fn longest_common_char_prefix(a: &str, b: &str) -> usize {
    let mut a_iter = a.chars();
    let mut b_iter = b.chars();

    let mut i = 0;
    loop {
        // If we've reached the end of either str, or if the characters don't
        // match, we're done.
        let Some(a_char) = a_iter.next() else {
            return i;
        };
        let Some(b_char) = b_iter.next() else {
            return i;
        };
        if a_char != b_char {
            return i;
        }
        i += 1;
    }
}

impl FloatDigitsMismatchContext {
    pub fn from_values(my_val: f64, resim_val: f64) -> Option<Self> {
        if my_val == resim_val {
            return None;
        }
        let my_val_str = format!("{my_val}");
        let mut resim_val_str = format!("{resim_val}");
        let prefix_len = longest_common_char_prefix(&my_val_str, &resim_val_str);

        // Build a string with the mismatched digits
        let mismatching_digits = resim_val_str[prefix_len..].to_string();

        // Build a string with any extra digits
        let extra_digits = my_val_str
            .get(resim_val_str.len()..)
            .unwrap_or("")
            .to_string();

        // Conveniently we already have a string with the prefix at the start,
        // so just shorten it to fit. truncate mutates the original String
        resim_val_str.truncate(prefix_len);

        Some(Self {
            mine: my_val,
            resim: resim_val,
            matching_digits: resim_val_str,
            mismatching_digits,
            extra_digits,
        })
    }
}

impl FloatMatchContext {
    pub fn from_values(my_val: f64, resim_val: f64) -> Self {
        match FloatDigitsMismatchContext::from_values(my_val, resim_val) {
            None => Self::Matches,
            Some(mismatch) => Self::Mismatch(mismatch),
        }
    }
}

impl OptionBoolMatchContext {
    pub fn from_values(my_val: Option<bool>, resim_val: Option<bool>) -> Self {
        match (my_val, resim_val) {
            (None, None) => Self::Matches,
            (Some(mine), None) => Self::MineExistsResimMissing { mine },
            (None, Some(resim)) => Self::MineMissingResimExists { resim },
            (Some(mine), Some(resim)) if mine == resim => Self::Matches,
            (Some(mine), Some(resim)) => Self::Mismatch { mine, resim },
        }
    }
}

impl OptionFloatMatchContext {
    pub fn from_values(my_val: Option<f64>, resim_val: Option<f64>) -> Self {
        match (my_val, resim_val) {
            (None, None) => Self::Matches,
            (Some(mine), None) => Self::MineExistsResimMissing { mine },
            (None, Some(resim)) => Self::MineMissingResimExists { resim },
            (Some(mine), Some(resim)) => match FloatDigitsMismatchContext::from_values(mine, resim)
            {
                None => Self::Matches,
                Some(mismatch) => Self::Mismatch(mismatch),
            },
        }
    }
}

impl RollPurposeMatchContext {
    pub fn from_values(mine: &RollPurpose, resim: &RollPurpose) -> Self {
        if mine == resim {
            Self::Matches
        } else {
            Self::Mismatch {
                mine: format!("{mine:?}"),
                resim: format!("{resim:?}"),
            }
        }
    }
}

impl ResimMatchContext {
    pub fn for_roll(roll_data: &RollData, check: &CheckRoll) -> Self {
        match roll_data.usage {
            RollUsage::Threshold { threshold, passed } => {
                ResimMatchContext::Threshold {
                    rolls: FloatMatchContext::from_values(roll_data.roll, check.roll),
                    purpose: RollPurposeMatchContext::from_values(
                        &roll_data.purpose,
                        &check.purpose,
                    ),
                    passed: OptionBoolMatchContext::from_values(passed, check.passed()),
                    // TODO This should have some sort of trace so we know when the threshold
                    //   was dependent on an earlier roll outcome
                    threshold: OptionFloatMatchContext::from_values(threshold, check.threshold),
                }
            }
            RollUsage::Choice { .. } => ResimMatchContext::Choice {
                rolls: FloatMatchContext::from_values(roll_data.roll, check.roll),
                purpose: RollPurposeMatchContext::from_values(&roll_data.purpose, &check.purpose),
            },
        }
    }
}

impl RollContext {
    pub fn for_roll(roll_data: RollData, check_roll: Option<CheckRoll>) -> Self {
        let resim_mismatch = check_roll
            .as_ref()
            .map(|cr| ResimMatchContext::for_roll(&roll_data, cr));
        Self {
            purpose: roll_data.purpose.to_string(),
            rng_state: roll_data.state_string,
            roll: roll_data.roll,
            usage: roll_data.usage,
            resim_mismatch,
        }
    }
}

impl Engine {
    pub fn new(rng: Rng, check_rolls: Option<RollStream>) -> Engine {
        Engine {
            rng,
            check_rolls,
            active_games: HashMap::new(),
            pending_updates: Vec::new(),
            current_day: None,
            tick_number: 0,
        }
    }

    pub async fn next_update(
        &mut self,
        update: ChroniclerGameUpdate,
        th: &Thresholds,
        chron: &Chronicler,
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
                    self.tick(updates_to_process, th, chron).await
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
        chron: &Chronicler,
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
                    // previous day; they will never be used again. Also also
                    // reset the tick number. Maybe I should encapsulate all
                    // this per-day stuff in a separate struct.
                    self.tick_number = 0;
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
                Entry::Occupied(mut entry) => run_game_tick(
                    entry.get_mut(),
                    update,
                    th,
                    &mut self.rng,
                    &mut self.check_rolls,
                )?,
                Entry::Vacant(entry) => {
                    // The first few updates of a game can be skipped and nothing bad happens
                    // (because they don't do any rolls), but if we're starting a game later than
                    // approximately play count 3 something has gone wrong
                    assert!(update.data.play_count < 3);
                    let game_at_tick = sim::Game::from_first_game_update(&update, chron).await;
                    run_game_tick(
                        entry.insert(game_at_tick),
                        update,
                        th,
                        &mut self.rng,
                        &mut self.check_rolls,
                    )?
                }
            };

            game_updates.push(game_update);
        }

        day.ticks.push(TickContext {
            tick_number: self.tick_number,
            tick_timestamp,
            games: game_updates,
        });

        self.tick_number += 1;
        Ok(finished_day)
    }
}

// Sadly, this can't be an
fn run_game_tick(
    game: &mut sim::Game,
    update: ChroniclerGameUpdate,
    th: &Thresholds,
    rng: &mut Rng,
    check_rolls: &mut Option<RollStream>,
) -> Result<GameTickContext, EngineFatalError> {
    let game_at_tick = game.at_tick(&update);

    let (mut errors, warnings) = game_at_tick.validate(&update);
    let game_label = format!(
        "{} @ {}",
        update.data.away_team_nickname, update.data.home_team_nickname
    );
    match update_parser::parse_update(&update) {
        Ok(parsed_update) => {
            let rolls = rolls_for_update(rng, parsed_update, th, &game_at_tick)
                .into_iter()
                .map(|roll_data| {
                    if let Some(check_rolls) = check_rolls {
                        // TODO This is the least efficient way to do it
                        if let Some(check_roll) = check_rolls.pop_front() {
                            Ok(RollContext::for_roll(roll_data, Some(check_roll)))
                        } else {
                            Err(EngineFatalError::RanOutOfCheckRolls)
                        }
                    } else {
                        Ok(RollContext::for_roll(roll_data, None))
                    }
                })
                .collect::<Result<_, _>>()?;

            let prev = game_at_tick.runners_at_end;
            game.set_prev(prev);
            Ok(GameTickContext {
                game_label,
                description: update.data.last_update,
                errors,
                warnings,
                rolls,
            })
        }
        Err(err) => {
            errors.push(format!("Parse error: {err}"));
            let prev = game_at_tick.runners_at_end;
            game.set_prev(prev);
            Ok(GameTickContext {
                game_label,
                description: update.data.last_update,
                errors,
                warnings,
                rolls: Vec::new(),
            })
        }
    }
}
