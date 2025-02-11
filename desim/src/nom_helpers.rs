use nom::branch::alt;
use nom::bytes::complete::{tag, take_until1};
use nom::character::complete::digit1;
use nom::combinator::{map_res, recognize, verify};
use nom::sequence::terminated;
use nom::Parser;

pub type ParserError<'a> = nom::error::Error<&'a str>;
pub type ParserResult<'a, Out> = nom::IResult<&'a str, Out, ParserError<'a>>;

pub(crate) fn parse_whole_number(input: &str) -> ParserResult<i64> {
    map_res(digit1, str::parse).parse(input)
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
