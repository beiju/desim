use crate::event_parser::{ParsedEvent, ParsedEventData};

pub enum RollConstrains {
    Unconstrained,
}

pub struct Roll {
    pub key: &'static str,
    pub description: String,
    pub constraints: RollConstrains,
}

impl Roll {
    pub fn new(key: &'static str, description: String, constraints: RollConstrains) -> Roll {
        Roll {
            key,
            description,
            constraints,
        }
    }
}

fn standard_rolls() -> Vec<Roll> {
    let mut rolls = Vec::new();
    rolls.push(Roll::new(
        "weather",
        "Weather proc?".to_string(),
        RollConstrains::Unconstrained,
    ));
    rolls.push(Roll::new(
        "steal_fielder",
        "Steal fielder (not necessarily used)".to_string(),
        RollConstrains::Unconstrained,
    ));

    rolls
}

fn rolls_for_pitch() -> Vec<Roll> {
    let mut rolls = standard_rolls();
    rolls.push(Roll::new(
        "strike",
        "Strike or ball?".to_string(),
        RollConstrains::Unconstrained,
    ));
    rolls.push(Roll::new(
        "swing",
        "Did player swing?".to_string(),
        RollConstrains::Unconstrained,
    ));

    rolls
}

fn rolls_for_contact() -> Vec<Roll> {
    let mut rolls = rolls_for_pitch();
    rolls.push(Roll::new(
        "contact",
        "Contact?".to_string(),
        RollConstrains::Unconstrained,
    ));

    rolls
}

fn rolls_for_foul() -> Vec<Roll> {
    let mut rolls = rolls_for_contact();
    rolls.push(Roll::new(
        "fair",
        "Fair or foul?".to_string(),
        RollConstrains::Unconstrained,
    ));

    rolls
}

pub fn rolls_for_event(event: &ParsedEvent) -> Vec<Roll> {
    match event.data {
        // No rolls for these events
        ParsedEventData::PlayBall => vec![],
        ParsedEventData::InningTurnover => vec![],
        ParsedEventData::BatterUp => vec![],
        ParsedEventData::Ball => rolls_for_pitch(),
        ParsedEventData::FoulBall => rolls_for_foul(),
    }
}
