use crate::error::{Error, Kind};
use crate::value::{Number, Position, SpannedValue, Value};
use nom::bytes::complete::{take_till, take_until};
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

pub type Span<'a> = LocatedSpan<&'a str>;

pub type Result<'a, R> = IResult<Span<'a>, R, Error>;
pub type ParseResult = std::result::Result<SpannedValue, Error>;

fn take_until_delimiter(i: Span, is_key: bool) -> Result<String> {
    let mut chars = String::from(" ,]}\n");
    if is_key {
        chars = format!("{}{}", chars, ':');
    }

    take_till(move |c| chars.contains(c))(i).map(|(i, found)| (i, String::from(*found.fragment())))
}

fn map_err<'a, P, I, O, E, G>(mut parser: P, mut func: G) -> impl FnMut(I) -> IResult<I, O, E>
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

fn boolean(i: Span) -> Result<bool> {
    let parse_true = value(true, tag("true"));

    let parse_false = value(false, tag("false"));

    alt((parse_true, parse_false)).parse(i)
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
            e.kind = Kind::InvalidHex(format!("'{}' is an invalid hex number", number));

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
            e.kind = Kind::InvalidString;
            Err::Error(e)
        }
        Err::Failure(mut e) => {
            e.start = start;
            e.end.col -= 1;
            e.kind = Kind::MissingQuote;
            Err::Failure(e)
        }
    })
}

type Sign = Option<char>;
type ParsedNumber<'a> = (Span<'a>, Option<(char, Span<'a>)>);
type Exp<'a> = Option<(char, Option<char>, Span<'a>)>;

fn number(i: Span) -> Result<Number> {
    let start = Position::from(i);

    let (i, (sign, digit)) = tuple((
        opt(char('-')),
        verify(digit1, |i: &Span| {
            let frag = i.fragment();

            !(frag.len() > 1 && frag.starts_with('0'))
        }),
    ))(i)
    .map_err(|_: Err<Error>| {
        Err::Error(Error::new(start.clone(), start.clone(), Kind::NotANumber))
    })?;

    let (i, rest) = take_until_delimiter(i, false)?;

    let sign = sign.map(|_| "-").unwrap_or("");

    let formatted = format!("{}{}{}", sign, digit.fragment(), rest);

    let number = (if formatted.starts_with('-') {
        formatted.parse().map(Number::NegInt)
    } else {
        formatted.parse().map(Number::PosInt)
    })
    .or_else(|_| formatted.parse().map(Number::Float))
    .map_err(|_| Err::Error(Error::new(start.clone(), start.clone(), Kind::NotANumber)))?;

    Ok((i, number))
}

fn array(i: Span) -> Result<Vec<SpannedValue>> {
    let start = Position::from(i);

    preceded(
        char('['),
        cut(terminated(
            separated_list0(
                preceded(multispace0, char(',')),
                map_err(json_value, |e| match e {
                    Err::Failure(e) if e.kind == Kind::InvalidValue("".into()) => {
                        // Discarding the failure as empty invalid value means it's an empty array
                        Err::Error(Error::default())
                    }
                    v => v,
                }),
            ),
            preceded(multispace0, char(']')),
        )),
    )(i)
    .map_err(|e| match e {
        Err::Failure(mut e) => {
            println!("Failure: {:?}", e);
            // If not nom error then it's another failure of a json value
            if let Kind::NomError(_) = e.kind {
                e.kind = Kind::MissingArrayBracket;
                e.start = start;
                e.end.col -= 1;
            };

            Err::Failure(e)
        }
        Err::Error(mut e) if e.kind == Kind::NomError(ErrorKind::Char) => {
            e.kind = Kind::NotAnArray;
            Err::Error(e)
        }
        e => e,
    })
}

fn key_value(i: Span<'_>) -> Result<(String, SpannedValue)> {
    let (i, key) = preceded(multispace0, string)(i).or_else(|e| {
        match e {
            Err::Error(mut e) => {
                // Before failing if the key is invalid, we check if the object is empty
                let pos_before_space = Position::from(i);

                let (i, _) = multispace0(i)?;

                if i.starts_with('}') || i.is_empty() {
                    Err(Err::Error(e))
                } else {
                    let (i, key) = take_until_delimiter(i, true)?;
                    if key.is_empty() {
                        e.start = pos_before_space;
                    }
                    e.kind = Kind::InvalidKey(key);
                    let end = Position::from_end(i);
                    e.end = end;

                    Err(Err::Failure(e))
                }
            }
            e => Err(e),
        }
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
    let start = Position::from(i);

    preceded(
        char('{'),
        cut(terminated(
            map(
                separated_list0(preceded(multispace0, char(',')), key_value),
                |tuple_vec| tuple_vec.into_iter().collect(),
            ),
            preceded(multispace0, char('}')),
        )),
    )(i)
    .map_err(|e| match e {
        Err::Failure(mut e) => {
            // If not nom error then it's another failure inside key_value
            if let Kind::NomError(_) = e.kind {
                e.kind = Kind::MissingObjectBracket;
                e.start = start;
                e.end.col -= 1;
            };

            Err::Failure(e)
        }
        Err::Error(mut e) if e.kind == Kind::NomError(ErrorKind::Char) => {
            e.kind = Kind::NotAnObject;
            Err::Error(e)
        }
        e => e,
    })
}

fn json_value(i: Span) -> Result<SpannedValue> {
    let (i, _) = many0(multispace1)(i)?;

    let start = Position::from(i);

    let (i, value) = alt((
        map(hash, Value::Object),
        map(array, Value::Array),
        map(string, Value::String),
        map(number, Value::Number),
        map(boolean, Value::Bool),
        map(null, |_| Value::Null),
    ))(i)
    .or_else(|e| match e {
        Err::Error(e) if e.kind == Kind::NomError(ErrorKind::Alt) => {
            let (i, value) = take_until_delimiter(i, false)?;

            Err(Err::Failure(Error::new(
                start.clone(),
                Position::from_end(i),
                Kind::InvalidValue(value),
            )))
        }
        e => Err(e),
    })?;

    let end = Position::from_end(i);

    Ok((i, SpannedValue { start, end, value }))
}

pub fn end_chars(i: Span) -> std::result::Result<(Span, ()), Error> {
    let (rest, _) = unwrap_nom_error(many0(multispace1)(i))?;

    if rest.fragment() == &"" {
        return Ok((rest, ()));
    }

    let start = Position::from(rest);

    let (end, _) = unwrap_nom_error(many_till(anychar, eof)(rest))?;

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
