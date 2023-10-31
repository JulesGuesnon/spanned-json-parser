use std::{collections::HashMap, error::Error, result};

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take, take_while},
    character::complete::{anychar, char, digit1, multispace0, multispace1, none_of, one_of},
    combinator::{complete, cut, map, map_opt, map_res, opt, value, verify},
    error::context,
    multi::{fold_many0, many0, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Err, IResult, Parser,
};
use nom_locate::{position, LocatedSpan};

use crate::value::{Number, Position, SpannedValue, Value};

pub type Span<'a> = LocatedSpan<&'a str>;

fn boolean(input: Span) -> IResult<Span, bool> {
    let parse_true = value(true, tag("true"));

    let parse_false = value(false, tag("false"));

    alt((parse_true, parse_false)).parse(input)
}

fn null(input: Span) -> IResult<Span, ()> {
    value((), tag("null")).parse(input)
}

fn u16_hex(i: Span) -> IResult<Span, u16> {
    map_res(take(4usize), |s: Span| {
        u16::from_str_radix(s.fragment(), 16)
    })(i)
}

fn unicode_escape(i: Span) -> IResult<Span, char> {
    map_opt(
        alt((
            // Not a surrogate
            map(verify(u16_hex, |cp| !(0xD800..0xE000).contains(cp)), |cp| {
                cp as u32
            }),
            // See https://en.wikipedia.org/wiki/UTF-16#Code_points_from_U+010000_to_U+10FFFF for details
            map(
                verify(
                    separated_pair(u16_hex, tag("\\u"), u16_hex),
                    |(high, low)| (0xD800..0xDC00).contains(high) && (0xDC00..0xE000).contains(low),
                ),
                |(high, low)| {
                    let high_ten = (high as u32) - 0xD800;
                    let low_ten = (low as u32) - 0xDC00;
                    (high_ten << 10) + low_ten + 0x10000
                },
            ),
        )),
        // Could probably be replaced with .unwrap() or _unchecked due to the verify checks
        std::char::from_u32,
    )(i)
}

fn parse_char(i: Span) -> IResult<Span, char> {
    let (i, c) = none_of("\"")(i)?;

    if c == '\\' {
        alt((
            map_res(anychar, |c| {
                Ok(match c {
                    '"' | '\\' | '/' => c,
                    'b' => '\x08',
                    'f' => '\x0C',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => return Err(()),
                })
            }),
            preceded(char('u'), unicode_escape),
        ))(i)
    } else {
        Ok((i, c))
    }
}

fn string(i: Span<'_>) -> IResult<Span, String> {
    context(
        "string",
        delimited(
            char('"'),
            fold_many0(parse_char, String::new, |mut string, c| {
                string.push(c);
                string
            }),
            cut(char('"')),
        ),
    )(i)
}

type Sign = Option<char>;
type ParsedNumber<'a> = (Span<'a>, Option<(char, Span<'a>)>);
type Exp<'a> = Option<(char, Option<char>, Span<'a>)>;

fn number(i: Span) -> IResult<Span, Number> {
    let (i, matched) = context(
        "number",
        tuple((
            opt(char('-')),
            tuple((
                verify(digit1, |i: &Span| {
                    let frag = i.fragment();

                    !(frag.len() > 1 && frag.starts_with('0'))
                }),
                opt(complete(pair(char('.'), digit1))),
            )),
            opt(complete(tuple((one_of("eE"), opt(one_of("-+")), digit1)))),
        )),
    )(i)?;

    let (sign, (body, decimal), exp): (Sign, ParsedNumber, Exp) = matched;

    let sign = sign.map(|_| "-").unwrap_or("");

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

    let result = match (sign, decimal.is_some(), exp.is_some()) {
        ("", false, false) => formatted
            .parse()
            .map(Number::PosInt)
            .map_err(|_| nom::error::Error::new(i, nom::error::ErrorKind::TooLarge)),
        ("-", false, false) => formatted
            .parse()
            .map(Number::NegInt)
            .map_err(|_| nom::error::Error::new(i, nom::error::ErrorKind::TooLarge)),
        _ => formatted
            .parse()
            .map(Number::Float)
            .map_err(|_| nom::error::Error::new(i, nom::error::ErrorKind::TooLarge)),
        _ => panic!("Not supposed to happen"),
    }
    .map_err(nom::Err::Failure);

    Ok((i, result?))
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

fn key_value(i: Span<'_>) -> IResult<Span, (String, SpannedValue)> {
    separated_pair(
        preceded(multispace0, string),
        cut(preceded(multispace0, char(':'))),
        json_value,
    )
    .parse(i)
}

fn hash(i: Span<'_>) -> IResult<Span, HashMap<String, SpannedValue>> {
    let (i, result) = preceded(
        char('{'),
        cut(terminated(
            map(
                separated_list0(preceded(multispace0, char(',')), key_value),
                |tuple_vec| tuple_vec.into_iter().collect(),
            ),
            preceded(multispace0, char('}')),
        )),
    )(i)?;

    Ok((i, result))
}

fn json_value(i: Span) -> IResult<Span, SpannedValue> {
    let (i, _) = many0(multispace1)(i)?;

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

pub fn parse(s: &str) -> Result<SpannedValue, String> {
    let span = Span::new(s);

    let (i, value) = json_value(span).map_err(|e| e.to_string())?;

    let (rest, _) =
        many0(multispace1)(i).map_err(|e: nom::Err<nom::error::Error<Span>>| e.to_string())?;

    if rest.fragment() != &"" {
        Err(String::from(*rest.fragment()))
    } else {
        Ok(value)
    }
}
