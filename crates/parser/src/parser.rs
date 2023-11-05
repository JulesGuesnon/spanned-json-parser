use crate::error::{Error, Kind};
use crate::input::Input;
use crate::value::{Number, Position, SpannedValue, Value};
use nom::bytes::complete::{take_till, take_until};
use nom::character::complete::digit0;
use nom::combinator::eof;
use nom::error::{ErrorKind, ParseError};
use nom::multi::many_till;
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
use std::collections::HashMap;
use std::num::ParseIntError;
use std::time::Instant;

pub type Span<'a> = Input<'a>;
// pub type Span<'a> = LocatedSpan<&'a str>;

pub type Result<'a, R> = IResult<Span<'a>, R, Error>;
pub type ParseResult = std::result::Result<SpannedValue, Error>;

fn take_until_delimiter(i: Span, is_key: bool) -> Result<String> {
    let mut chars = String::from(" ,]}\n");
    if is_key {
        chars = format!("{}{}", chars, ':');
    }

    take_till(move |c| chars.contains(c))(i).map(|(i, found)| (i, String::from(found.fragment())))
}

fn map_err<P, I, O, E, G>(mut parser: P, mut func: G) -> impl FnMut(I) -> IResult<I, O, E>
where
    P: Parser<I, O, E>,
    G: FnMut(Err<E>) -> Err<E>,
    E: ParseError<I>,
    I: Clone,
{
    move |i: I| {
        let i = i.clone();
        let result = parser.parse(i);

        match result {
            Err(e) => Err(func(e)),
            v => v,
        }
    }
}

fn parse_true(i: Span) -> Result<bool> {
    value(true, tag("rue"))(i).or_else(|_: Err<Error>| {
        let start = Position::from_ahead(i);

        let (i, invalid_rest) = take_until_delimiter(i, false)?;

        let mut value = String::from('t');
        value.push_str(&invalid_rest);
        drop(invalid_rest);

        Err(Err::Failure(Error::new(
            start,
            Position::from_ahead(i),
            Kind::InvalidValue(value),
        )))
    })
}

fn parse_false(i: Span) -> Result<bool> {
    value(false, tag("alse"))(i).or_else(|_: Err<Error>| {
        let start = Position::from_ahead(i);

        let (i, invalid_rest) = take_until_delimiter(i, false)?;

        let mut value = String::from('f');
        value.push_str(&invalid_rest);
        drop(invalid_rest);

        Err(Err::Failure(Error::new(
            start,
            Position::from_ahead(i),
            Kind::InvalidValue(value),
        )))
    })
}

fn null(i: Span) -> Result<()> {
    value((), tag("ull"))(i).or_else(|_: Err<Error>| {
        let start = Position::from_ahead(i);

        let (i, invalid_rest) = take_until_delimiter(i, false)?;

        let mut value = String::from('n');
        value.push_str(&invalid_rest);
        drop(invalid_rest);

        Err(Err::Failure(Error::new(
            start,
            Position::from_ahead(i),
            Kind::InvalidValue(value),
        )))
    })
}

fn u16_hex(i: Span) -> Result<u16> {
    map_res(take(4usize), |s: Span| {
        u16::from_str_radix(s.fragment(), 16)
    })(i)
    .map_err(|mut e: Err<Error>| match e {
        Err::Error(mut e) => {
            let mut end = e.start.clone();
            end.col += 4;

            let number = i.fragment().get(0..4).unwrap_or("");

            e.end = end;
            e.kind = Kind::NotAnHex(format!("'{}' is an invalid hex number", number));

            Err::Error(e)
        }
        _ => panic!("map_res didn't return Error"),
    })
}

fn unicode_escape(i: Span) -> Result<char> {
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

fn parse_char(i: Span) -> Result<char> {
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

fn string(i: Span<'_>) -> Result<String> {
    let start = Position::from_ahead(i);

    terminated(
        fold_many0(parse_char, String::new, |mut string, c| {
            string.push(c);
            string
        }),
        cut(char('"')),
    )(i)
    .map_err(|e| match e {
        Err::Failure(mut e) => {
            e.start = start;
            e.end.col -= 1;
            e.kind = Kind::MissingQuote;
            Err::Failure(e)
        }
        e => e,
    })
}

type Sign = Option<char>;
type ParsedNumber<'a> = (Span<'a>, Option<(char, Span<'a>)>);
type Exp<'a> = Option<(char, Option<char>, Span<'a>)>;

fn number(first_char: char) -> impl FnMut(Span) -> IResult<Span, Number, Error>
where
{
    move |i: Span| {
        let start = Position::from_ahead(i);

        let (i, digit) = verify(digit0, |i: &Span| {
            let frag = i.fragment();

            frag.is_empty() || first_char != '0'
        })(i)
        .map_err(|_: Err<Error>| {
            Err::Error(Error::new(start.clone(), start.clone(), Kind::NotANumber))
        })?;

        let (i, rest) = take_until_delimiter(i, false)?;

        let formatted = format!("{}{}{}", first_char, digit.fragment(), rest);

        let number =
            (if formatted.contains('.') || formatted.contains('e') || formatted.contains('E') {
                formatted.parse().map(Number::Float).map_err(|_| ())
            } else if first_char == '-' {
                formatted
                    .parse()
                    .map(Number::NegInt)
                    // Parsing too big numbers into float
                    .or_else(|_| formatted.parse().map(Number::Float))
                    .map_err(|_| ())
            } else {
                formatted
                    .parse()
                    .map(Number::PosInt)
                    // Parsing too big numbers into float
                    .or_else(|_| formatted.parse().map(Number::Float))
                    .map_err(|_| ())
            })
            .map_err(|_| {
                Err::Failure(Error::new(
                    start,
                    Position::from_ahead(i),
                    Kind::InvalidValue(formatted),
                ))
            })?;

        Ok((i, number))
    }
}

fn array(i: Span) -> Result<Vec<SpannedValue>> {
    let start = Position::from_ahead(i);

    let (i, _) = multispace0(i)?;

    if i.starts_with(']') {
        let (i, _) = anychar(i)?;

        Ok((i, Vec::new()))
    } else if i.is_empty() {
        let mut end = start.clone();
        end.col += 1;
        Err(Err::Failure(Error::new(
            start,
            end,
            Kind::MissingArrayBracket,
        )))
    } else {
        cut(terminated(
            separated_list0(preceded(multispace0, char(',')), json_value),
            preceded(multispace0, char(']')),
        ))(i)
        .map_err(|e| match e {
            Err::Failure(mut e) => {
                // If not nom error then it's another failure of a json value
                if let Kind::NomError(_) = e.kind {
                    e.kind = Kind::MissingArrayBracket;
                    e.start = start;
                    e.end.col -= 1;
                };

                Err::Failure(e)
            }
            e => e,
        })
    }
}

fn key_value(i: Span<'_>) -> Result<(String, SpannedValue)> {
    let pos_before_space = Position::from(i);

    let (i, _) = multispace0(i)?;

    if i.starts_with('}') || i.is_empty() {
        // Key value is called in a loop, and only an error can stop it
        return Err(Err::Error(Error::default()));
    }

    let (i, key) = preceded(char('"'), string)(i).or_else(|e| match e {
        Err::Error(mut e) => {
            let (i, key) = take_until_delimiter(i, true)?;
            if key.is_empty() {
                e.start = pos_before_space;
            }
            e.kind = Kind::InvalidKey(key);
            let end = Position::from_ahead(i);
            e.end = end;

            Err(Err::Failure(e))
        }
        e => Err(e),
    })?;

    let (i, _) = cut(preceded(multispace0, char(':')))(i).map_err(|e: Err<Error>| match e {
        Err::Failure(mut e) => {
            e.kind = Kind::MissingColon;
            let pos = Position::from(i);
            e.start = pos.clone();
            e.end = pos;
            Err::Failure(e)
        }
        e => e,
    })?;

    let (i, value) = json_value(i)?;

    Ok((i, (key, value)))
}

fn hash(i: Span<'_>) -> Result<HashMap<String, SpannedValue>> {
    let start = Position::from_ahead(i);

    cut(terminated(
        map(
            separated_list0(preceded(multispace0, char(',')), key_value),
            |tuple_vec| tuple_vec.into_iter().collect(),
        ),
        preceded(multispace0, char('}')),
    ))(i)
    .map_err(|e| match e {
        Err::Failure(mut e) => {
            println!("Failure: {:?}", e);
            // If not nom error then it's another failure inside key_value
            if let Kind::NomError(_) = e.kind {
                e.kind = Kind::MissingObjectBracket;
                e.start = start;
                e.end.col -= 1;
            };

            Err::Failure(e)
        }
        e => e,
    })
}

fn json_value(i: Span) -> Result<SpannedValue> {
    let (i, _) = many0(multispace1)(i)?;

    println!("Input before: {:?}", i);
    let start = Position::from(i);

    let (i, first_char) = anychar(i)?;

    let (i, value) = match first_char {
        '{' => map(hash, Value::Object)(i),
        '[' => map(array, Value::Array)(i),
        '"' => map(string, Value::String)(i),
        '-' | '0'..='9' => map(number(first_char), Value::Number)(i),
        't' => map(parse_true, Value::Bool)(i),
        'f' => map(parse_false, Value::Bool)(i),
        'n' => map(null, |_| Value::Null)(i),
        c => {
            let (i, v) = take_until_delimiter(i, false)?;

            let mut value = String::from(c);
            value.push_str(&v);
            drop(v);

            Err(Err::Failure(Error::new(
                start.clone(),
                Position::from_ahead(i),
                Kind::InvalidValue(value),
            )))
        }
    }?;

    println!("Input after: {:?}", i);
    let end = Position::from_ahead(i);

    Ok((i, SpannedValue { start, end, value }))
}

pub fn end_chars(i: Span) -> std::result::Result<(Span, ()), Error> {
    let (rest, _) = unwrap_nom_error(many0(multispace1)(i))?;

    if rest.fragment() == "" {
        return Ok((rest, ()));
    }

    let start = Position::from(rest);

    let (end, _) = unwrap_nom_error(many_till(anychar, eof)(rest))?;

    Err(Error::new(
        start,
        Position::from_ahead(end),
        Kind::CharsAfterRoot(format!(
            "Unexpected chararacters at the end: {}",
            rest.fragment()
        )),
    ))
}

pub fn unwrap_nom_error<T>(value: Result<T>) -> std::result::Result<(Span, T), Error> {
    match value {
        Ok(v) => Ok(v),
        Err(nom::Err::Error(e)) => Err(e),
        Err(nom::Err::Failure(e)) => Err(e),
        Err(nom::Err::Incomplete(_)) => panic!("Got Incomplete error"),
    }
}

pub fn parse(s: &str) -> ParseResult {
    let span = Span::new(s);

    let (i, value) = unwrap_nom_error(json_value(span))?;

    let _ = end_chars(i)?;

    Ok(value)
}
