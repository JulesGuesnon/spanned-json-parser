use std::{collections::HashMap, error::Error};

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while},
    character::complete::{char, digit1, multispace0, none_of, one_of},
    combinator::{complete, cut, map, opt, value},
    error::context,
    multi::separated_list0,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult, Parser,
};
use nom_locate::{position, LocatedSpan};

use crate::value::{Number, Position, SpannedValue, Value};

pub type Span<'a> = LocatedSpan<&'a str>;

fn is_sp(c: char) -> bool {
    let chars = " \t\r\n";

    chars.contains(c)
}

fn boolean(input: Span) -> IResult<Span, bool> {
    let parse_true = value(true, tag("true"));

    let parse_false = value(false, tag("false"));

    alt((parse_true, parse_false)).parse(input)
}

fn null(input: Span) -> IResult<Span, ()> {
    value((), tag("null")).parse(input)
}

fn parse_str(i: Span<'_>) -> IResult<Span, &str> {
    println!("Str {}", i);
    let (i, res) = escaped(opt(none_of("\\\"")), '\\', char('"'))(i)?;

    println!("HERE? {}", res);
    Ok((i, res.fragment()))
}

fn string(i: Span<'_>) -> IResult<Span, &str> {
    println!("String {}", i);
    context("string", delimited(char('"'), parse_str, cut(char('"'))))(i)
}

type Sign = Option<char>;
type ParsedNumber<'a> = (Span<'a>, Option<(char, Span<'a>)>);
type Exp<'a> = Option<(char, Option<char>, Span<'a>)>;

fn number(i: Span) -> IResult<Span, Number> {
    let (i, matched) = context(
        "number",
        tuple((
            opt(one_of("-+")),
            tuple((digit1, opt(complete(pair(char('.'), digit1))))),
            opt(complete(tuple((one_of("eE"), opt(one_of("-+")), digit1)))),
        )),
    )(i)?;

    let (sign, (body, decimal), exp): (Sign, ParsedNumber, Exp) = matched;

    let sign = sign.unwrap_or('+');

    let formatted = format!(
        "{}{}{}{}",
        sign,
        body.fragment(),
        match decimal {
            Some((_, number)) => format!(".{}", number.fragment()),
            None => String::new(),
        },
        match exp {
            Some((_, sign, number)) => {
                let sign = sign.map(|c| if c == '+' { "" } else { "-" }).unwrap_or("");

                format!("e{}{}", sign, number.fragment())
            }
            None => String::new(),
        }
    );

    let result = match (sign, decimal.is_some()) {
        ('+', false) => Number::PosInt(formatted.parse().unwrap()),
        ('-', false) => Number::NegInt(formatted.parse().unwrap()),
        (_, true) => Number::Float(formatted.parse().unwrap()),
        _ => panic!("Not supposed to happen"),
    };

    Ok((i, result))
}

fn array(i: Span) -> IResult<Span, Vec<SpannedValue>> {
    context(
        "array",
        preceded(
            char('['),
            cut(terminated(
                separated_list0(preceded(multispace0, char(',')), json_value),
                preceded(multispace0, char(']')),
            )),
        ),
    )(i)
}

fn key_value(i: Span<'_>) -> IResult<Span, (&str, SpannedValue)> {
    separated_pair(
        preceded(multispace0, string),
        cut(preceded(multispace0, char(':'))),
        json_value,
    )
    .parse(i)
}

fn hash(i: Span<'_>) -> IResult<Span, HashMap<&str, SpannedValue>> {
    context(
        "map",
        preceded(
            char('{'),
            cut(terminated(
                map(
                    separated_list0(preceded(multispace0, char(',')), key_value),
                    |tuple_vec| tuple_vec.into_iter().collect(),
                ),
                preceded(multispace0, char('}')),
            )),
        ),
    )(i)
}

fn json_value(i: Span) -> IResult<Span, SpannedValue> {
    let (i, _) = take_while(is_sp)(i)?;

    let (i, pos) = position(i)?;

    let start = Position::from(pos);

    let (i, value) = alt((
        map(hash, Value::Object),
        map(array, Value::Array),
        map(string, Value::String),
        map(number, Value::Number),
        map(boolean, Value::Bool),
        map(null, |_| Value::Null),
    ))(i)?;

    let (i, pos) = position(i)?;

    let end = Position::from(pos);

    Ok((i, SpannedValue { start, end, value }))
}

fn root(i: Span) -> IResult<Span, SpannedValue> {
    let (i, _) = take_while(is_sp)(i)?;

    let (i, value) = json_value(i)?;

    Ok((i, value))
}

pub fn parse(s: &str) -> Result<SpannedValue, String> {
    let span = Span::new(s);

    let (_, value) = root(span).map_err(|e| e.to_string())?;

    Ok(value)
}
