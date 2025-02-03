use crate::event_parser::{ParsedEvent, ParsedEventData};
use crate::thresholds::Thresholds;

pub enum RollConstrains {
    Unconstrained {
        description: String,
    },
    BelowThreshold {
        threshold: f64,
        positive_description: String,
        negative_description: String,
    },
    AboveThreshold {
        threshold: f64,
        positive_description: String,
        negative_description: String,
    },
    Unused {
        description: String,
    },
}

pub struct Roll {
    pub key: &'static str,
    pub constraints: RollConstrains,
}

impl Roll {
    pub fn new(key: &'static str, constraints: RollConstrains) -> Self {
        Self { key, constraints }
    }
}

fn standard_rolls(th: &Thresholds) -> Vec<Roll> {
    let mut rolls = Vec::new();
    rolls.push(Roll::new(
        "weather",
        RollConstrains::AboveThreshold {
            threshold: th.weather,
            negative_description: "Weather procced".to_string(),
            positive_description: "Weather did not proc".to_string(),
        },
    ));
    rolls.push(Roll::new(
        "steal_fielder",
        RollConstrains::Unused {
            description: "Steal fielder".to_string(),
        },
    ));

    rolls
}

fn rolls_for_pitch(
    th: &Thresholds,
    in_strike_zone: Option<bool>,
    player_swung: Option<bool>,
) -> Vec<Roll> {
    let mut rolls = standard_rolls(th);
    let strike_zone_constraint = match in_strike_zone {
        None => RollConstrains::Unconstrained {
            description: "In strike zone?".to_string(),
        },
        Some(true) => RollConstrains::BelowThreshold {
            threshold: th.in_strike_zone(),
            positive_description: "Pitch in strike zone".to_string(),
            negative_description: "Expected pitch in strike zone, but it was outside".to_string(),
        },
        Some(false) => RollConstrains::AboveThreshold {
            threshold: th.in_strike_zone(),
            positive_description: "Pitch outside strike zone".to_string(),
            negative_description: "Expected pitch outside strike zone, but it was inside"
                .to_string(),
        },
    };
    rolls.push(Roll::new("in_strike_zone", strike_zone_constraint));
    let swing_constraint = match player_swung {
        None => RollConstrains::Unconstrained {
            description: "Did player swing?".to_string(),
        },
        Some(true) => RollConstrains::BelowThreshold {
            threshold: th.in_strike_zone(),
            positive_description: "Player swung".to_string(),
            negative_description: "Expected player to swing, but they didn't".to_string(),
        },
        Some(false) => RollConstrains::AboveThreshold {
            threshold: th.in_strike_zone(),
            positive_description: "Player did not swing".to_string(),
            negative_description: "Expected player to not swing, but they did".to_string(),
        },
    };
    rolls.push(Roll::new("swing", swing_constraint));

    rolls
}

fn rolls_for_contact(
    th: &Thresholds,
    in_strike_zone: Option<bool>,
    made_contact: Option<bool>,
) -> Vec<Roll> {
    let mut rolls = rolls_for_pitch(th, in_strike_zone, Some(true));
    let constrains = match made_contact {
        None => RollConstrains::Unconstrained {
            description: "Contact?".to_string(),
        },
        Some(true) => RollConstrains::BelowThreshold {
            threshold: th.made_contact(),
            positive_description: "Swing made contact".to_string(),
            negative_description: "Expected swing to make contact, but it didn't".to_string(),
        },
        Some(false) => RollConstrains::AboveThreshold {
            threshold: th.made_contact(),
            positive_description: "Swing did not make contact".to_string(),
            negative_description: "Expected swing to not make contact, but it did".to_string(),
        },
    };
    rolls.push(Roll::new("contact", constrains));

    rolls
}

fn rolls_for_foul(th: &Thresholds, in_strike_zone: Option<bool>) -> Vec<Roll> {
    let mut rolls = rolls_for_contact(th, in_strike_zone, Some(true));
    rolls.push(Roll::new(
        "fair",
        RollConstrains::Unconstrained {
            description: "Fair or foul?".to_string(),
        },
    ));

    rolls
}

pub fn rolls_for_event(event: &ParsedEvent, th: &Thresholds) -> Vec<Roll> {
    match event.data {
        // No rolls for these events
        ParsedEventData::PlayBall => vec![],
        ParsedEventData::InningTurnover => vec![],
        ParsedEventData::BatterUp => vec![],
        // Balls are known to not be in the strike zone and the player didn't swing
        ParsedEventData::Ball => rolls_for_pitch(th, Some(false), Some(false)),
        // Fouls may be in or out of the strike zone
        ParsedEventData::FoulBall => rolls_for_foul(th, None),
        // Strikeouts looking are known to be in the strike zone and the player didn't swing
        ParsedEventData::StrikeoutLooking => rolls_for_pitch(th, Some(true), Some(true)),
        ParsedEventData::StrikeoutSwinging => rolls_for_contact(th, None, Some(true)),
    }
}
