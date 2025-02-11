use crate::sim;
use crate::thresholds::Thresholds;
use crate::update_parser::{ParsedUpdate, ParsedUpdateData};

// TODO Refactor this, it no longer does a good job of serving its purpose
pub enum RollConstrains {
    Unconstrained {
        // We may know the threshold without knowing whether the roll should
        // be above or below it
        threshold: Option<f64>,
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
        // We may know the threshold without knowing whether the roll should
        // be above or below it
        threshold: Option<f64>,
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RollPurpose {
    PartyTime,
    StealFielder,
    MildPitch,
    InStrikeZone,
    Swing,
    Contact,
    FairOrFoul,
    Fielder,
    Out(String),
    Fly,
    HomeRun,
    Double(String),
    Triple(String),
    Steal(i64),
    Advance((i64, bool)),
    DoublePlayHappens,
    DoublePlayWhere,
    PartyTargetTeam, // TODO is this right?
    // This is our escape hatch. Anything we don't support ends up here. Don't
    // ever use this value; instead, add a parser for the thing you're using
    Unparsed(String),
}

pub struct RollSpec {
    pub purpose: RollPurpose,
    pub constraints: RollConstrains,
}

impl RollSpec {
    pub fn new(purpose: RollPurpose, constraints: RollConstrains) -> Self {
        Self {
            purpose,
            constraints,
        }
    }
}

fn standard_rolls() -> Vec<RollSpec> {
    let mut rolls = Vec::new();
    rolls.push(RollSpec::new(
        RollPurpose::PartyTime,
        RollConstrains::Unused {
            threshold: None,
            description: "Party time".to_string(),
        },
    ));
    rolls.push(RollSpec::new(
        RollPurpose::StealFielder,
        RollConstrains::Unused {
            threshold: None,
            description: "Steal fielder".to_string(),
        },
    ));

    rolls
}

fn rolls_for_pitch(
    th: &Thresholds,
    game: &sim::GameAtTick,
    in_strike_zone: Option<bool>,
    player_swung: Option<bool>,
) -> Vec<RollSpec> {
    let mut rolls = standard_rolls();
    rolls.push(RollSpec::new(
        RollPurpose::MildPitch,
        RollConstrains::AboveThreshold {
            threshold: th.mild_pitch(),
            positive_description: "No mild pitch".to_string(),
            negative_description: "Expected no mild pitch".to_string(),
        },
    ));

    let strike_zone_constraint = match in_strike_zone {
        None => RollConstrains::Unconstrained {
            threshold: None,
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
    rolls.push(RollSpec::new(
        RollPurpose::InStrikeZone,
        strike_zone_constraint,
    ));
    let swing_constraint = match player_swung {
        None => RollConstrains::Unconstrained {
            threshold: None,
            description: "Did player swing?".to_string(),
        },
        Some(true) => RollConstrains::BelowThreshold {
            threshold: th.swing(true /* TODO this value comes from rng */, game),
            positive_description: "Player swung".to_string(),
            negative_description: "Expected player to swing, but they didn't".to_string(),
        },
        Some(false) => RollConstrains::AboveThreshold {
            threshold: th.swing(true /* TODO this value comes from rng */, game),
            positive_description: "Player did not swing".to_string(),
            negative_description: "Expected player to not swing, but they did".to_string(),
        },
    };
    rolls.push(RollSpec::new(RollPurpose::Swing, swing_constraint));

    rolls
}

fn rolls_for_contact(
    th: &Thresholds,
    game: &sim::GameAtTick,
    in_strike_zone: Option<bool>,
    made_contact: Option<bool>,
) -> Vec<RollSpec> {
    let mut rolls = rolls_for_pitch(th, game, in_strike_zone, Some(true));
    let constrains = match made_contact {
        None => RollConstrains::Unconstrained {
            threshold: None,
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
    rolls.push(RollSpec::new(RollPurpose::Contact, constrains));

    rolls
}

fn rolls_for_foul(
    th: &Thresholds,
    game: &sim::GameAtTick,
    in_strike_zone: Option<bool>,
) -> Vec<RollSpec> {
    let mut rolls = rolls_for_contact(th, game, in_strike_zone, Some(true));
    rolls.push(RollSpec::new(
        RollPurpose::FairOrFoul,
        RollConstrains::Unconstrained {
            threshold: None,
            description: "Fair or foul?".to_string(),
        },
    ));

    rolls
}

pub fn rolls_for_update(
    update: &ParsedUpdate,
    th: &Thresholds,
    game: &sim::GameAtTick,
) -> Vec<RollSpec> {
    match update.data {
        // No rolls for these updates
        ParsedUpdateData::Empty => vec![],
        ParsedUpdateData::PlayBall => vec![],
        ParsedUpdateData::InningTurnover => vec![],
        ParsedUpdateData::BatterUp => vec![],
        // Balls are known to not be in the strike zone and the player didn't swing
        ParsedUpdateData::Ball => rolls_for_pitch(th, game, Some(false), Some(false)),
        // Fouls may be in or out of the strike zone
        ParsedUpdateData::FoulBall => rolls_for_foul(th, game, None),
        // Strikeouts looking are known to be in the strike zone and the player didn't swing
        ParsedUpdateData::StrikeLooking => rolls_for_pitch(th, game, Some(true), Some(true)),
        ParsedUpdateData::StrikeoutLooking => rolls_for_pitch(th, game, Some(true), Some(true)),
        ParsedUpdateData::StrikeSwinging => rolls_for_contact(th, game, None, Some(true)),
        ParsedUpdateData::StrikeoutSwinging => rolls_for_contact(th, game, None, Some(true)),
    }
}
