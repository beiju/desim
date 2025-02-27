use crate::rng::Rng;
use crate::sim;
use crate::thresholds::Thresholds;
use crate::update_parser::{ParsedUpdate, ParsedUpdateData};
use serde::Serialize;
use std::fmt::{Display, Formatter};

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

impl Display for RollPurpose {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RollPurpose::PartyTime => {
                write!(f, "Was there a party?")
            }
            RollPurpose::StealFielder => {
                write!(f, "Choose the steal fielder")
            }
            RollPurpose::MildPitch => {
                write!(f, "Mild pitch?")
            }
            RollPurpose::InStrikeZone => {
                write!(f, "Ball in strike zone?")
            }
            RollPurpose::Swing => {
                write!(f, "Did batter swing?")
            }
            RollPurpose::Contact => {
                write!(f, "Did batter make contact?")
            }
            RollPurpose::FairOrFoul => {
                write!(f, "Was the ball fair?")
            }
            RollPurpose::Fielder => {
                write!(f, "Choose the fielder")
            }
            RollPurpose::Out(name) => {
                write!(f, "Did {name} catch the out?")
            }
            RollPurpose::Fly => {
                write!(f, "TODO what is fly")
            }
            RollPurpose::HomeRun => {
                write!(f, "Was it a home run?")
            }
            RollPurpose::Double(name) => {
                write!(f, "Was it a double? (with fielder {name})")
            }
            RollPurpose::Triple(name) => {
                write!(f, "Was it a triple? (with fielder {name})")
            }
            RollPurpose::Steal(base) => {
                write!(f, "Did the runner steal base {base}?")
            }
            RollPurpose::Advance(_) => {
                write!(f, "Did the runner advance?")
            }
            RollPurpose::DoublePlayHappens => {
                write!(f, "Was there a double play?")
            }
            RollPurpose::DoublePlayWhere => {
                write!(f, "Where was the double play?")
            }
            RollPurpose::PartyTargetTeam => {
                write!(f, "Which team partied?")
            }
            RollPurpose::Unparsed(val) => {
                write!(f, "Other: {val}")
            }
        }
    }
}

#[derive(Serialize)]
pub enum RollUsage {
    Threshold {
        threshold: Option<f64>,
        passed: Option<bool>,
    },
    Choice {
        num_options: usize,
        selected_option: Option<usize>,
    },
}

pub struct RollData {
    pub state_string: String,
    pub roll: f64,
    pub purpose: RollPurpose,
    pub usage: RollUsage,
}

impl RollData {
    pub fn for_threshold(
        rng: &mut Rng,
        purpose: RollPurpose,
        threshold: Option<f64>,
        passed: Option<bool>,
    ) -> Self {
        rng.step(1);
        Self {
            state_string: rng.state_string(),
            roll: rng.value(),
            purpose,
            usage: RollUsage::Threshold { threshold, passed },
        }
    }

    pub fn for_choice(
        rng: &mut Rng,
        purpose: RollPurpose,
        num_options: usize,
        selected_option: Option<usize>,
    ) -> Self {
        rng.step(1);
        Self {
            state_string: rng.state_string(),
            roll: rng.value(),
            purpose,
            usage: RollUsage::Choice {
                num_options,
                selected_option,
            },
        }
    }
}

fn standard_rolls(rng: &mut Rng) -> Vec<RollData> {
    let mut rolls = Vec::new();
    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::PartyTime,
        None,
        None,
    ));
    // TODO Supply roll fielder choice when known
    rolls.push(RollData::for_choice(
        rng,
        RollPurpose::StealFielder,
        0,
        None,
    ));

    rolls
}

fn rolls_for_pitch(
    rng: &mut Rng,
    th: &Thresholds,
    game: &sim::GameAtTick,
    in_strike_zone: Option<bool>,
    player_swung: Option<bool>,
) -> Vec<RollData> {
    let mut rolls = standard_rolls(rng);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::MildPitch,
        None,
        None,
    ));

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::InStrikeZone,
        Some(th.in_strike_zone(game)),
        in_strike_zone,
    ));

    let in_strike_zone = rolls.last().unwrap().roll < th.in_strike_zone(game);
    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Swing,
        Some(th.swing(in_strike_zone, game)),
        player_swung,
    ));

    rolls
}

fn rolls_for_contact(
    rng: &mut Rng,
    th: &Thresholds,
    game: &sim::GameAtTick,
    in_strike_zone: Option<bool>,
    made_contact: Option<bool>,
) -> Vec<RollData> {
    let mut rolls = rolls_for_pitch(rng, th, game, in_strike_zone, Some(true));

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Contact,
        Some(th.made_contact()),
        made_contact,
    ));

    rolls
}

fn rolls_for_foul(
    rng: &mut Rng,
    th: &Thresholds,
    game: &sim::GameAtTick,
    in_strike_zone: Option<bool>,
) -> Vec<RollData> {
    let mut rolls = rolls_for_contact(rng, th, game, in_strike_zone, Some(true));

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::FairOrFoul,
        None,
        None,
    ));

    rolls
}

pub fn rolls_for_update(
    rng: &mut Rng,
    update: &ParsedUpdate,
    th: &Thresholds,
    game: &sim::GameAtTick,
) -> Vec<RollData> {
    match update.data {
        // No rolls for these updates
        ParsedUpdateData::Empty => vec![],
        ParsedUpdateData::PlayBall => vec![],
        ParsedUpdateData::InningTurnover => vec![],
        ParsedUpdateData::BatterUp => vec![],
        // Balls are known to not be in the strike zone and the player didn't swing
        ParsedUpdateData::Ball => rolls_for_pitch(rng, th, game, Some(false), Some(false)),
        // Fouls may be in or out of the strike zone
        ParsedUpdateData::FoulBall => rolls_for_foul(rng, th, game, None),
        // Strikeouts looking are known to be in the strike zone and the player didn't swing
        ParsedUpdateData::StrikeLooking => rolls_for_pitch(rng, th, game, Some(true), Some(true)),
        ParsedUpdateData::StrikeoutLooking => {
            rolls_for_pitch(rng, th, game, Some(true), Some(true))
        }
        ParsedUpdateData::StrikeSwinging => rolls_for_contact(rng, th, game, None, Some(true)),
        ParsedUpdateData::StrikeoutSwinging => rolls_for_contact(rng, th, game, None, Some(true)),
    }
}
