mod engine;
mod game_log;

use chrono::TimeDelta;
use itertools::Itertools;
use log::{debug, warn};
use std::fmt::Formatter;

// Nominal tick duration is 5 seconds, but our timestamps are post-network-delay so there is
// definite jitter there
const MIN_EXPECTED_TICK_DURATION: TimeDelta = TimeDelta::seconds(3);

struct PrintGameEvents<'a>(&'a [game_log::GameEvent]);
impl<'a> std::fmt::Display for PrintGameEvents<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for event in self.0 {
            write!(f, "\t- {}: ", event.game_id)?;
            let mut lines = event.data.last_update.lines();
            if let Some(line) = lines.next() {
                let num_lines = 1 + lines.count();
                write!(f, "{}", line)?;
                if num_lines > 1 {
                    write!(f, "... ({num_lines} lines)")?;
                }
            } else {
                write!(f, "(empty)")?;
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let game_data = game_log::load_games()?;
    let mut all_events = game_data
        .into_iter()
        .flat_map(|game| game.data)
        .collect_vec();
    all_events.sort_by_key(|game| game.timestamp);

    let start_time = all_events.first().unwrap().timestamp;
    let mut prev_tick_timestamp = None;
    for (tick_timestamp, group) in all_events
        .into_iter()
        .chunk_by(|game| game.timestamp)
        .into_iter()
    {
        let group = group.collect_vec();
        assert_eq!(
            group.iter().duplicates_by(|g| g.game_id).count(),
            0,
            "A tick must not contain multiple events for the same game"
        );

        if let Some(prev_tick_timestamp) = prev_tick_timestamp {
            let tick_duration = tick_timestamp - prev_tick_timestamp;
            if tick_duration < MIN_EXPECTED_TICK_DURATION {
                warn!("Tick duration was only {tick_duration}");
            }
        }
        prev_tick_timestamp = Some(tick_timestamp);

        let time_since_start = tick_timestamp - start_time;
        debug!(
            "Tick at {tick_timestamp} (T+{time_since_start}), with events: \n{}",
            PrintGameEvents(&group)
        );
    }

    Ok(())
}
