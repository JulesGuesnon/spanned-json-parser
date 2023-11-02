use crate::{parser::Span, value::Position};
use nom::error::{ErrorKind, FromExternalError, ParseError};
use std::num::IntErrorKind;
use std::num::ParseFloatError;
use std::num::ParseIntError;

#[derive(Debug, PartialEq)]
pub enum Kind {
    InvalidString,
    MissingQuote,
    MissingArrayBracket,
    MissingObjectBracket,
    InvalidKey(String),
    MissingColon,
    InvalidHex(String),
    InvalidNumber(String),
    CharsAfterRoot(String),
    InvalidBool,
    InvalidNull,
    NotAnObject,
    NotAnArray,
    NotANumber,
    InvalidValue(String),
    NomError(nom::error::ErrorKind),
    // Used when an error will be remaped
    ToBeDefined,
}

#[derive(Debug)]
pub struct Error {
    pub start: Position,
    pub end: Position,
    pub kind: Kind,
}

impl Error {
    pub fn new(start: Position, end: Position, value: Kind) -> Self {
        Self {
            start,
            end,
            kind: value,
        }
    }
}

impl Default for Error {
    fn default() -> Self {
        Self {
            start: Position::default(),
            end: Position::default(),
            kind: Kind::ToBeDefined,
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        let position = Position::default();
        match value.kind() {
            IntErrorKind::Empty => Self::new(
                position.clone(),
                position,
                Kind::InvalidNumber("Failed to parsed number. Reason: empty".into()),
            ),
            IntErrorKind::InvalidDigit => Self::new(
                position.clone(),
                position,
                Kind::InvalidNumber("Failed to parsed number. Reason: not a valid number".into()),
            ),
            IntErrorKind::PosOverflow => Self::new(
                position.clone(),
                position,
                Kind::InvalidNumber("Failed to parsed number. Reason: number too large".into()),
            ),
            IntErrorKind::NegOverflow => Self::new(
                position.clone(),
                position,
                Kind::InvalidNumber("Failed to parsed number. Reason: number too small".into()),
            ),
            IntErrorKind::Zero => Self::new(
                position.clone(),
                position,
                Kind::InvalidNumber("Failed to parsed number. Reason: zero".into()),
            ),
            _ => Self::new(
                position.clone(),
                position,
                Kind::InvalidNumber("Failed to parsed number. Reason: unknown".into()),
            ),
        }
    }
}

impl From<ParseFloatError> for Error {
    fn from(value: ParseFloatError) -> Self {
        let position = Position::default();

        Self::new(
            position.clone(),
            position,
            Kind::InvalidNumber("Failed to parsed number. Reason: not a valid float".into()),
        )
    }
}

impl<'a> ParseError<Span<'a>> for Error {
    fn from_error_kind(input: Span<'a>, kind: ErrorKind) -> Self {
        let position = Position::from(input);

        Self {
            start: position.clone(),
            end: position,
            kind: Kind::NomError(kind),
        }
    }

    fn append(input: Span<'a>, kind: ErrorKind, other: Self) -> Self {
        let pos = Position::from(input);

        Self {
            start: pos.clone(),
            end: pos,
            kind: Kind::NomError(kind),
        }
    }
}

impl<'a, T> FromExternalError<Span<'a>, T> for Error {
    fn from_external_error(input: Span<'a>, kind: ErrorKind, e: T) -> Self {
        let position = Position::from(input);

        Self::new(position.clone(), position, Kind::ToBeDefined)
    }
}
