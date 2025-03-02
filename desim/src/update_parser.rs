use crate::nom_helpers::{parse_terminated, parse_whole_number, ParserResult};
use blaseball_api::ChroniclerGameUpdate;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::eof;
use nom::Parser;
use thiserror::Error;

pub struct ParsedUpdate {
    pub data: ParsedUpdateData,
}

pub enum ParsedUpdateData {
    Empty,
    PlayBall,
    InningTurnover,
    BatterUp,
    Ball,
    FoulBall,
    StrikeLooking,
    StrikeoutLooking,
    StrikeSwinging,
    StrikeoutSwinging,
    GroundOut,
    Flyout,
    InningEnd,
    Hit,
}

#[derive(Error, Debug)]
pub enum UpdateParseError {
    #[error("Couldn't parse description: {0}")]
    FailedToParseDescription(String),
}

pub fn parse_update(game_update: &ChroniclerGameUpdate) -> Result<ParsedUpdate, UpdateParseError> {
    let (_, data) = parse_description
        .parse(&game_update.data.last_update)
        .map_err(|err| UpdateParseError::FailedToParseDescription(err.to_string()))?;

    Ok(ParsedUpdate { data })
}

fn parse_description(input: &str) -> ParserResult<ParsedUpdateData> {
    alt((
        parse_empty,
        parse_play_ball,
        parse_inning_turnover,
        parse_batter_up,
        parse_ball,
        parse_foul_ball,
        parse_strikeout,
        parse_strike,
        parse_ground_out,
        parse_flyout,
        parse_inning_end,
        parse_hit,
    ))
    .parse(input)
}

fn parse_empty(input: &str) -> ParserResult<ParsedUpdateData> {
    eof.map(|_| ParsedUpdateData::Empty).parse(input)
}

fn parse_play_ball(input: &str) -> ParserResult<ParsedUpdateData> {
    tag("Play ball!")
        .map(|_| ParsedUpdateData::PlayBall)
        .parse(input)
}

fn parse_inning_turnover(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _) = alt((tag("Top"), tag("Bottom"))).parse(input)?;
    let (input, _) = tag(" of ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag(", ").parse(input)?;
    let (input, _) = parse_terminated(" batting.").parse(input)?;

    Ok((input, ParsedUpdateData::InningTurnover))
}

fn parse_batter_up(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _) = parse_terminated(" batting for the ").parse(input)?;
    // TODO Parsing just a period is fragile; try porting parse_until_period_eof from Fed
    let (input, _) = parse_terminated(".").parse(input)?;

    Ok((input, ParsedUpdateData::BatterUp))
}

fn parse_ball(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _) = tag("Ball. ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag("-").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;

    Ok((input, ParsedUpdateData::Ball))
}

fn parse_foul_ball(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _) = tag("Foul Ball. ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag("-").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;

    Ok((input, ParsedUpdateData::FoulBall))
}

fn parse_strikeout(input: &str) -> ParserResult<ParsedUpdateData> {
    alt((
        parse_terminated(" strikes out looking.").map(|_| ParsedUpdateData::StrikeoutLooking),
        parse_terminated(" strikes out swinging.").map(|_| ParsedUpdateData::StrikeoutSwinging),
    ))
    .parse(input)
}

fn parse_strike(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, strike_type) = alt((
        tag("Strike, swinging.").map(|_| ParsedUpdateData::StrikeSwinging),
        tag("Strike, looking.").map(|_| ParsedUpdateData::StrikeLooking),
    ))
    .parse(input)?;

    let (input, _) = tag(" ").parse(input)?;
    let (input, _strikes) = digit1.parse(input)?;
    let (input, _) = tag("-").parse(input)?;
    let (input, _balls) = digit1.parse(input)?;

    Ok((input, strike_type))
}

fn parse_ground_out(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _batter_name) = parse_terminated(" hit a ground out to ").parse(input)?;
    // TODO Parsing just a period is fragile; try porting parse_until_period_eof from Fed
    let (input, _fielder_name) = parse_terminated(".").parse(input)?;

    Ok((input, ParsedUpdateData::GroundOut))
}

fn parse_flyout(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _batter_name) = parse_terminated(" hit a flyout to ").parse(input)?;
    // TODO Parsing just a period is fragile; try porting parse_until_period_eof from Fed
    let (input, _fielder_name) = parse_terminated(".").parse(input)?;

    Ok((input, ParsedUpdateData::Flyout))
}

fn parse_inning_end(input: &str) -> ParserResult<ParsedUpdateData> {
    let (input, _) = tag("Inning ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag(" is now an Outing.").parse(input)?;

    Ok((input, ParsedUpdateData::InningEnd))
}

fn parse_hit(input: &str) -> ParserResult<ParsedUpdateData> {
    // TODO: Other kinds of hit
    let (input, _batter_name) = parse_terminated(" hits a Single!").parse(input)?;

    Ok((input, ParsedUpdateData::Hit))
}
