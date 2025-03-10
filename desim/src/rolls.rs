use serde::Serialize;
use std::fmt::{Display, Formatter};

use crate::rng::Rng;
use crate::sim::{GameAtTick, PlayerAtTick};
use crate::thresholds::Thresholds;
use crate::update_parser::{ParsedUpdate, ParsedUpdateData};

#[derive(Debug, Clone, PartialEq)]
pub enum RollPurpose {
    Party,
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
            RollPurpose::Party => {
                write!(f, "Did the party roll pass?")
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
                write!(f, "Was it a flyout?")
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
        selected_option_description: Option<String>,
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
        selected_option_description: Option<String>,
    ) -> Self {
        rng.step(1);
        Self {
            state_string: rng.state_string(),
            roll: rng.value(),
            purpose,
            usage: RollUsage::Choice {
                num_options,
                selected_option,
                selected_option_description,
            },
        }
    }
}

fn party_rolls(rng: &mut Rng, th: &Thresholds) -> Vec<RollData> {
    let mut rolls = Vec::new();

    // Party rolls every tick, regardless of party time
    let party_threshold = th.party();
    let party_roll = RollData::for_threshold(
        rng,
        RollPurpose::Party,
        Some(party_threshold),
        None, // I'll fill this in when I encounter a party
    );
    let party_roll_passed = party_roll.roll < party_threshold;
    rolls.push(party_roll);

    // If party roll passed, a team is chosen
    if party_roll_passed {
        rolls.push(RollData::for_choice(
            rng,
            RollPurpose::PartyTargetTeam,
            2,
            None,
            None,
        ))
    }

    rolls
}
fn standard_rolls(rng: &mut Rng, th: &Thresholds, game: &GameAtTick) -> Vec<RollData> {
    let mut rolls = party_rolls(rng, th);

    let _steal_fielder =
        choose_fielder_for_purpose(rng, game, &mut rolls, RollPurpose::StealFielder);

    for (current_base, _runner) in game.runners_at_start() {
        rolls.push(RollData::for_threshold(
            rng,
            RollPurpose::Steal(current_base),
            None,
            None,
        ));
    }

    rolls
}

fn rolls_for_pitch(
    rng: &mut Rng,
    th: &Thresholds,
    game: &GameAtTick,
    in_strike_zone: Option<bool>,
) -> Vec<RollData> {
    let mut rolls = standard_rolls(rng, th, game);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::MildPitch,
        Some(th.mild_pitch()),
        Some(false), // I'll fill this in when I encounter a mild pitch
    ));

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::InStrikeZone,
        Some(th.in_strike_zone(game)),
        in_strike_zone,
    ));

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Swing,
        None, // TODO
        None, // TODO
    ));

    rolls
}

fn rolls_for_contact(rng: &mut Rng, th: &Thresholds, game: &GameAtTick) -> Vec<RollData> {
    let mut rolls = rolls_for_pitch(rng, th, game, None);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Contact,
        None, // TODO
        None, // TODO
    ));

    rolls
}

fn rolls_for_foul_or_fair(
    rng: &mut Rng,
    th: &Thresholds,
    game: &GameAtTick,
    fair: bool,
) -> Vec<RollData> {
    let mut rolls = rolls_for_contact(rng, th, game);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::FairOrFoul,
        Some(th.fair_ball(game)),
        Some(fair),
    ));

    rolls
}

fn rolls_for_foul(rng: &mut Rng, th: &Thresholds, game: &GameAtTick) -> Vec<RollData> {
    rolls_for_foul_or_fair(rng, th, game, false)
}

fn rolls_for_fair(
    rng: &mut Rng,
    th: &Thresholds,
    game: &GameAtTick,
    is_hit: bool,
) -> Vec<RollData> {
    let mut rolls = rolls_for_foul_or_fair(rng, th, game, true);

    let hit_fielder = choose_fielder(rng, game, &mut rolls);

    // TODO Reverse the definition of this roll: it's currently described as
    //   "was it an out" but a hit is the pass condition so it should be
    //   described as "was it a hit"
    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Out(hit_fielder.player.name.clone()),
        Some(th.out(game, &hit_fielder)),
        Some(is_hit),
    ));

    rolls
}

fn rolls_for_out(
    rng: &mut Rng,
    th: &Thresholds,
    game: &GameAtTick,
    is_flyout: bool,
    is_dp: bool,
) -> Vec<RollData> {
    let mut rolls = rolls_for_fair(rng, th, game, false);

    let _fly_fielder = choose_fielder(rng, game, &mut rolls);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Fly,
        Some(th.fly(game)),
        Some(is_flyout),
    ));

    if !is_flyout {
        // Flyouts don't roll displayed fielder again, but ground outs do.
        // Presumably TBG is picking one player to catch the ball and then, if
        // it wasn't a flyout, another player to tag the base/runner.
        let _displayed_fielder = choose_fielder(rng, game, &mut rolls);
    }

    let eligible_for_double_play =
        !game.runners_at_start.is_empty() && game.outs + 1 >= game.max_outs;

    if eligible_for_double_play {
        rolls.push(RollData::for_threshold(
            rng,
            RollPurpose::DoublePlayHappens,
            None,
            None,
        ));

        if is_dp {
            rolls.push(RollData::for_choice(
                rng,
                RollPurpose::DoublePlayWhere,
                game.runners_at_start.len(), // TODO: Remove forced scores?
                None,
                None,
            ));
        }
    }

    rolls
}

fn rolls_for_basic_out(
    rng: &mut Rng,
    th: &Thresholds,
    game: &GameAtTick,
    is_flyout: bool,
) -> Vec<RollData> {
    rolls_for_out(rng, th, game, is_flyout, false)
}

fn rolls_for_hit(rng: &mut Rng, th: &Thresholds, game: &GameAtTick, hit_bases: i64, mut scored: Vec<&str>) -> Vec<RollData> {
    let mut rolls = rolls_for_fair(rng, th, game, true);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::HomeRun,
        Some(th.hr(game)),
        Some(false),
    ));

    let fielder = choose_fielder(rng, game, &mut rolls);

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Double(fielder.player.name.clone()),
        Some(th.double(game, &fielder)),
        Some(hit_bases == 2), // Not sure if this is "double or higher" or "double, exactly"
    ));

    rolls.push(RollData::for_threshold(
        rng,
        RollPurpose::Triple(fielder.player.name.clone()),
        Some(th.triple(game, &fielder)),
        Some(false),
    ));

    // It's more efficient to pop the last element
    scored.reverse();

    println!("runners at start:");
    for (base, runner) in game.runners_at_start() {
        println!("    {}: {}", base, runner.player.name);
    }

    let mut prev_occupied_base = None;
    // Note this list is already in descending order of occupied base
    for (base, runner) in game.runners_at_start() {
        let base_after_automatic_advance = base + hit_bases;

        if let Some(occupied_base) = prev_occupied_base {
            assert!(occupied_base > base_after_automatic_advance,
                    "The base this player automatically advanced to was occupied");
            if base_after_automatic_advance + 1 == occupied_base {
                // Then the batter is not eligible to advance
                prev_occupied_base = Some(base_after_automatic_advance);
                println!("{} not eligible because {} is occupied", runner.player.name, occupied_base);
                continue;
            }
        }

        let base_at_end = game
            .runners_at_end
            .iter()
            .find(|r| r.runner_id == runner.player.id)
            .map(|r| r.base);
        
        let advanced = if let Some(base_at_end) = base_at_end {
            if base_at_end == base_after_automatic_advance {
                false
            } else if base_at_end == base_after_automatic_advance + 1 {
                true
            } else if base_at_end > base_after_automatic_advance {
                panic!("Batter advanced too much! Was on {base_after_automatic_advance} after the automatic advance, then {base_at_end} after the optional advance!")
            } else {
                panic!("Batter advanced a negative amount! Was on {base_after_automatic_advance} after the automatic advance, then {base_at_end} after the optional advance!");
            }
        } else {
            // Assume the player scored
            if base_after_automatic_advance >= 3 {
                // Then they scored from the automatic advance and no roll
                // is necessary
                continue;
            } else {
                // Then they scored off the optional advance 
                assert_eq!(base_after_automatic_advance + 1, 3);
                true
            }
        };
        println!("{} advanced: {}", runner.player.name, advanced);

        rolls.push(RollData::for_threshold(
            rng,
            RollPurpose::Advance((base_after_automatic_advance, advanced)),
            Some(th.advance_on_hit(&runner, &fielder)),
            Some(advanced),
        ));

        if let Some(base) = base_at_end {
            prev_occupied_base = Some(base);
        }
    }

    rolls
}

fn rolls_for_double_play(rng: &mut Rng, th: &Thresholds, game: &GameAtTick) -> Vec<RollData> {
    rolls_for_out(rng, th, game, false, true)
}

fn choose_fielder<'a>(
    rng: &mut Rng,
    game: &'a GameAtTick,
    rolls: &mut Vec<RollData>,
) -> PlayerAtTick<'a> {
    choose_fielder_for_purpose(rng, game, rolls, RollPurpose::Fielder)
}

fn choose_fielder_for_purpose<'a>(
    rng: &mut Rng,
    game: &'a GameAtTick,
    rolls: &mut Vec<RollData>,
    purpose: RollPurpose,
) -> PlayerAtTick<'a> {
    let fielder_idx = (rng.next_value() * game.num_fielders() as f64) as usize;
    let fielder = game.fielder(fielder_idx);
    rolls.push(RollData::for_choice(
        rng,
        purpose,
        game.num_fielders(),
        Some(fielder_idx),
        Some(fielder.player.name.clone()),
    ));
    fielder
}

pub fn rolls_for_update(
    rng: &mut Rng,
    update: ParsedUpdate,
    th: &Thresholds,
    game: &GameAtTick,
) -> Vec<RollData> {
    match update.data {
        // No rolls for these updates
        ParsedUpdateData::Empty => vec![],
        ParsedUpdateData::PlayBall => vec![],
        ParsedUpdateData::InningTurnover => vec![],
        ParsedUpdateData::BatterUp => vec![],
        // Balls are known to not be in the strike zone and the player didn't swing
        ParsedUpdateData::Ball => rolls_for_pitch(rng, th, game, Some(false)),
        // Fouls may be in or out of the strike zone
        ParsedUpdateData::FoulBall => rolls_for_foul(rng, th, game),
        // Strikeouts looking are known to be in the strike zone and the player didn't swing
        ParsedUpdateData::StrikeLooking => rolls_for_pitch(rng, th, game, Some(true)),
        ParsedUpdateData::StrikeoutLooking => rolls_for_pitch(rng, th, game, Some(true)),
        ParsedUpdateData::StrikeSwinging => rolls_for_contact(rng, th, game),
        ParsedUpdateData::StrikeoutSwinging => rolls_for_contact(rng, th, game),
        ParsedUpdateData::GroundOut => rolls_for_basic_out(rng, th, game, false),
        ParsedUpdateData::Flyout => rolls_for_basic_out(rng, th, game, true),
        ParsedUpdateData::InningEnd => vec![],
        ParsedUpdateData::Hit { bases, scored } => rolls_for_hit(rng, th, game, bases, scored),
        ParsedUpdateData::DoublePlay => rolls_for_double_play(rng, th, game),
    }
}
