use crate::error::{Error, Kind};
use crate::value::{Number, Position, SpannedValue, Value};
use nom::bytes::complete::take_until;
use nom::combinator::eof;
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

pub type Span<'a> = LocatedSpan<&'a str>;

type Result<'a, R> = IResult<Span<'a>, R, Error>;
type ParseResult = std::result::Result<SpannedValue, Error>;

fn boolean(input: Span) -> Result<bool> {
    let parse_true = value(true, tag("true"));

    let parse_false = value(false, tag("false"));

    alt((parse_true, parse_false)).parse(input)
}

fn null(input: Span) -> Result<()> {
    value((), tag("null")).parse(input)
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
            e.value = Kind::InvalidHex(format!("'{}' is an invalid hex number", number));

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
    let start = Position::from(i);

    delimited(
        char('"'),
        fold_many0(parse_char, String::new, |mut string, c| {
            string.push(c);
            string
        }),
        cut(char('"')),
    )(i)
    .map_err(|e| match e {
        Err::Incomplete(_) => panic!("String: Incomplete error happened"),
        Err::Error(mut e) => {
            e.start = start;
            e.value = Kind::InvalidString;
            Err::Error(e)
        }
        Err::Failure(mut e) => {
            e.start = start;
            e.value = Kind::MissingQuote;
            Err::Failure(e)
        }
    })
}

type Sign = Option<char>;
type ParsedNumber<'a> = (Span<'a>, Option<(char, Span<'a>)>);
type Exp<'a> = Option<(char, Option<char>, Span<'a>)>;

fn number(i: Span) -> Result<Number> {
    let (i, matched) = tuple((
        opt(char('-')),
        tuple((
            verify(digit1, |i: &Span| {
                let frag = i.fragment();

                !(frag.len() > 1 && frag.starts_with('0'))
            }),
            opt(complete(pair(char('.'), digit1))),
        )),
        opt(complete(tuple((one_of("eE"), opt(one_of("-+")), digit1)))),
    ))(i)?;

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
        ("", false, false) => formatted.parse().map(Number::PosInt).map_err(Into::into),
        ("-", false, false) => formatted.parse().map(Number::NegInt).map_err(Into::into),
        _ => formatted.parse().map(Number::Float).map_err(Into::into),
    }
    .map_err(nom::Err::Failure);

    Ok((i, result?))
}

fn array(i: Span) -> Result<Vec<SpannedValue>> {
    let start = Position::from(i);

    preceded(
        char('['),
        cut(terminated(
            separated_list0(preceded(multispace0, char(',')), json_value),
            preceded(multispace0, char(']')),
        )),
    )(i)
    .map_err(|e| match e {
        Err::Incomplete(_) => panic!("Array: Incomplete error happened"),
        Err::Error(mut e) => {
            e.value = Kind::InvalidString;
            Err::Error(e)
        }
        Err::Failure(mut e) => {
            // If not nom error then it's another failure of a json value
            if let Kind::NomError(_) = e.value {
                e.value = Kind::MissingArrayBracket;
            };

            Err::Failure(e)
        }
    })
}

fn key_value(i: Span<'_>) -> Result<(String, SpannedValue)> {
    let (i, key) = preceded(multispace0, string)(i).or_else(|e| match e {
        Err::Error(mut e) => {
            let (i, (key, _)) = many_till(anychar, one_of(" :"))(i)?;
            e.value = Kind::InvalidKey(key.into_iter().collect());
            let end = Position {
                line: i.location_line() as usize,
                col: i.naive_get_utf8_column() - 2,
            };
            e.end = end;
            Err(Err::Failure(e))
        }
        Err::Incomplete(_) => panic!("Key Value: Incomplete not supposed to happen"),
        e => Err(e),
    })?;

    let (i, _) = cut(preceded(multispace0, char(':')))(i).map_err(|e: Err<Error>| match e {
        Err::Failure(mut e) => {
            e.value = Kind::MissingColon;
            Err::Failure(e)
        }
        e => e,
    })?;

    let (i, value) = json_value(i)?;

    Ok((i, (key, value)))
}

fn hash(i: Span<'_>) -> Result<HashMap<String, SpannedValue>> {
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

fn json_value(i: Span) -> Result<SpannedValue> {
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

    let end = Position::from_end(pos);

    Ok((i, SpannedValue { start, end, value }))
}

pub fn end_chars(i: Span) -> std::result::Result<(Span, ()), Error> {
    let (rest, _) = unwrap_nom_error(many0(multispace1)(i))?;

    if rest.fragment() == &"" {
        return Ok((rest, ()));
    }

    let (rest, start) = unwrap_nom_error(position(rest))?;

    let start = Position::from(start);

    let (end, _) = unwrap_nom_error(many_till(anychar, eof)(rest))?;
    let (_, end) = unwrap_nom_error(position(end))?;

    Err(Error::new(
        start,
        Position::from_end(end),
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
