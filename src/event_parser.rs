use crate::game_log;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until1};
use nom::combinator::{recognize, verify};
use nom::sequence::terminated;
use nom::Parser;
use thiserror::Error;

pub struct ParsedEvent {
    pub data: ParsedEventData,
}

pub enum ParsedEventData {
    PlayBall,
    InningTurnover,
    BatterUp,
    Ball,
    FoulBall,
    StrikeoutLooking,
    StrikeoutSwinging,
}

#[derive(Error, Debug)]
pub enum EventParseError {
    #[error("Couldn't parse description: {0}")]
    FailedToParseDescription(String),
}

pub fn parse_event(event: game_log::GameEvent) -> Result<ParsedEvent, EventParseError> {
    let (_, data) = parse_description
        .parse(&event.data.last_update)
        .map_err(|err| EventParseError::FailedToParseDescription(err.to_string()))?;

    Ok(ParsedEvent { data })
}

pub type ParserError<'a> = nom::error::Error<&'a str>;
pub type ParserResult<'a, Out> = nom::IResult<&'a str, Out, ParserError<'a>>;

pub(crate) fn parse_whole_number(input: &str) -> ParserResult<i64> {
    nom::combinator::map_res(nom::character::complete::digit1, str::parse).parse(input)
}

pub(crate) fn parse_terminated(tag_content: &str) -> impl Fn(&str) -> ParserResult<&str> + '_ {
    move |input| {
        let (input, parsed_value) = if tag_content == "." {
            alt((
                // The Kaj Statter Jr. rule
                verify(
                    recognize(terminated(take_until1(".."), tag("."))),
                    |s: &str| !s.contains('\n'),
                ),
                verify(take_until1(tag_content), |s: &str| !s.contains('\n')),
            ))
            .parse(input)
        } else {
            verify(take_until1(tag_content), |s: &str| !s.contains('\n')).parse(input)
        }?;
        let (input, _) = tag(tag_content).parse(input)?;

        Ok((input, parsed_value))
    }
}

fn parse_description(input: &str) -> ParserResult<ParsedEventData> {
    alt((
        parse_play_ball,
        parse_inning_turnover,
        parse_batter_up,
        parse_ball,
        parse_foul_ball,
        parse_strikeout,
    ))
    .parse(input)
}

fn parse_play_ball(input: &str) -> ParserResult<ParsedEventData> {
    tag("Play ball!")
        .map(|_| ParsedEventData::PlayBall)
        .parse(input)
}

fn parse_inning_turnover(input: &str) -> ParserResult<ParsedEventData> {
    let (input, _) = alt((tag("Top"), tag("Bottom"))).parse(input)?;
    let (input, _) = tag(" of ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag(", ").parse(input)?;
    let (input, _) = parse_terminated(" batting.").parse(input)?;

    Ok((input, ParsedEventData::InningTurnover))
}

fn parse_batter_up(input: &str) -> ParserResult<ParsedEventData> {
    let (input, _) = parse_terminated(" batting for the ").parse(input)?;
    // TODO Parsing just a period is fragile; try porting parse_until_period_eof from Fed
    let (input, _) = parse_terminated(".").parse(input)?;

    Ok((input, ParsedEventData::BatterUp))
}

fn parse_ball(input: &str) -> ParserResult<ParsedEventData> {
    let (input, _) = tag("Ball. ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag("-").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;

    Ok((input, ParsedEventData::Ball))
}

fn parse_foul_ball(input: &str) -> ParserResult<ParsedEventData> {
    let (input, _) = tag("Foul Ball. ").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;
    let (input, _) = tag("-").parse(input)?;
    let (input, _) = parse_whole_number.parse(input)?;

    Ok((input, ParsedEventData::FoulBall))
}

fn parse_strikeout(input: &str) -> ParserResult<ParsedEventData> {
    alt((
        parse_terminated(" strikes out looking.").map(|_| ParsedEventData::StrikeoutLooking),
        parse_terminated(" strikes out swinging.").map(|_| ParsedEventData::StrikeoutSwinging),
    ))
    .parse(input)
}
