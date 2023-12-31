use crate::error::{Error, Kind};
use crate::input::Input;
use crate::value::{Number, Position, SpannedValue, Value};
use nom::bytes::complete::take_till;
use nom::character::complete::digit0;
use nom::combinator::{eof, opt};
use nom::error::ParseError;
use nom::multi::many_till;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{anychar, char, multispace0, multispace1, none_of},
    combinator::{cut, map, map_opt, map_res, value, verify},
    multi::{fold_many0, many0, separated_list0},
    sequence::{preceded, separated_pair, terminated},
    Err, IResult, Parser,
};
use std::collections::HashMap;

pub type Span<'a> = Input<'a>;

pub type Result<'a, R> = IResult<Span<'a>, R, Error>;
pub type ParseResult = std::result::Result<SpannedValue, Error>;

fn take_until_delimiter(i: Span, is_key: bool) -> Result<String> {
    let mut chars = String::from(" ,]}\n");
    if is_key {
        chars = format!("{}{}", chars, ':');
    }

    take_till(move |c| chars.contains(c))(i).map(|(i, found)| (i, String::from(found.fragment())))
}

fn or_else<P, I, O, E, G>(mut parser: P, mut func: G) -> impl FnMut(I) -> IResult<I, O, E>
where
    P: Parser<I, O, E>,
    G: FnMut(Err<E>, I) -> IResult<I, O, E>,
    E: ParseError<I>,
    I: Clone,
{
    move |i: I| {
        let result = parser.parse(i.clone());

        match result {
            Err(e) => func(e, i),
            v => v,
        }
    }
}

pub fn map_parser<I, O1, O2, E: ParseError<I>, F, G>(
    mut parser: F,
    mut applied_parser: G,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: Parser<I, O1, E>,
    G: FnMut((I, O1)) -> IResult<I, O2, E>,
{
    move |input: I| applied_parser(parser.parse(input)?)
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
    .map_err(|e: Err<Error>| match e {
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
        terminated(
            separated_list0(
                preceded(
                    multispace0,
                    or_else(char(','), |e: Err<Error>, i| {
                        let (i, _) = multispace0(i)?;

                        match e {
                            Err::Error(mut e) if !i.is_empty() && !i.starts_with(']') => {
                                e.kind = Kind::MissingComma;
                                e.start = start.clone();
                                e.end.col -= 1;

                                Err(Err::Failure(e))
                            }
                            e => Err(e),
                        }
                    }),
                ),
                or_else(json_value, |e: Err<Error>, i| {
                    // If it succeeds, it means that it's a trailing comma
                    let _ = preceded(multispace0, char(']'))(i).map_err(|_: Err<Error>| e)?;

                    Err(Err::Failure(Error::new(
                        Position::from_ahead(i),
                        Position::from_ahead(i),
                        Kind::TrailingComma,
                    )))
                }),
            ),
            preceded(
                multispace0,
                or_else(char(']'), |e: Err<Error>, _| match e {
                    Err::Error(mut e) => {
                        e.kind = Kind::MissingArrayBracket;
                        e.start = start.clone();
                        e.end.col -= 1;

                        Err(Err::Failure(e))
                    }
                    e => Err(e),
                }),
            ),
        )(i)
    }
}

fn key_value(i: Span<'_>) -> Result<(String, SpannedValue)> {
    let (i, comma) = opt(char(','))(i)?;

    let pos_before_space = Position::from(i);

    let (i, _) = multispace0(i)?;

    if (i.starts_with('}') || i.is_empty()) && comma.is_none() {
        // Key value is called in a loop, and only an error can stop it
        return Err(Err::Error(Error::default()));
    }

    let (i, key) = preceded(char('"'), string)(i).or_else(|e| match e {
        Err::Error(mut e) => {
            let (i, key) = take_until_delimiter(i, true)?;

            let end = Position::from_ahead(i);

            if key.is_empty() {
                e.start = pos_before_space.clone();
            }
            e.kind = Kind::InvalidKey(key);
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

    let result: Result<HashMap<String, SpannedValue>> = terminated(
        map(
            separated_list0(
                preceded(
                    multispace0,
                    or_else(
                        map_parser(char(','), |(i, _): (Span, char)| {
                            let (j, _) = multispace0(i)?;

                            if j.starts_with('}') {
                                let position = Position::from_ahead(i);
                                Err(Err::Failure(Error::new(
                                    position.clone(),
                                    position,
                                    Kind::TrailingComma,
                                )))
                            } else {
                                Ok((i, ','))
                            }
                        }),
                        |e: Err<Error>, i| {
                            let (i, _) = multispace0(i)?;

                            match e {
                                Err::Error(mut e) if !i.is_empty() && !i.starts_with('}') => {
                                    e.kind = Kind::MissingComma;
                                    e.start = start.clone();
                                    e.end.col -= 1;

                                    Err(Err::Failure(e))
                                }
                                e => Err(e),
                            }
                        },
                    ),
                ),
                key_value,
            ),
            |tuple_vec| tuple_vec.into_iter().collect(),
        ),
        preceded(
            multispace0,
            or_else(char('}'), |e: Err<Error>, _| match e {
                Err::Error(mut e) => {
                    e.kind = Kind::MissingObjectBracket;
                    e.start = start.clone();
                    e.end.col -= 1;

                    Err(Err::Failure(e))
                }
                e => Err(e),
            }),
        ),
    )(i);

    #[allow(clippy::let_and_return)]
    result
}

fn json_value(i: Span) -> Result<SpannedValue> {
    let (i, _) = many0(multispace1)(i)?;

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

/// Use this function to parse your json into a [SpannedValue]
/// ```ignore
/// use spanned_json_parse::parse;
/// use std::fs;
///
/// fn main() {
///     let json = fs::read_to_string("path").unwrap();
///
///     let parsed = parse(&json);
///
///     println!("Parsed: {:#?}", parsed);
/// }
/// ```
pub fn parse(s: &str) -> ParseResult {
    let span = Span::new(s);

    let (i, value) = unwrap_nom_error(json_value(span))?;

    let _ = end_chars(i)?;

    Ok(value)
}
