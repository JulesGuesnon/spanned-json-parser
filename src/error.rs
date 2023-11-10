use crate::{parser::Span, value::Position};
use nom::error::{ErrorKind, FromExternalError, ParseError};
use std::num::ParseFloatError;
use std::num::ParseIntError;

#[derive(Debug, PartialEq)]
pub enum Kind {
    MissingQuote,
    MissingArrayBracket,
    MissingComma,
    MissingObjectBracket,
    InvalidKey(String),
    MissingChar(char),
    MissingColon,
    CharsAfterRoot(String),
    NotAnHex(String),
    NotAString,
    NotABool,
    NotANull,
    NotAnObject,
    NotAnArray,
    NotANumber,
    InvalidValue(String),
    TrailingComma,
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
    fn from(_value: ParseIntError) -> Self {
        let position = Position::default();
        Self::new(position.clone(), position, Kind::NotANumber)
    }
}

impl From<ParseFloatError> for Error {
    fn from(_value: ParseFloatError) -> Self {
        let position = Position::default();

        Self::new(position.clone(), position, Kind::NotANumber)
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

    fn append(input: Span<'a>, kind: ErrorKind, _other: Self) -> Self {
        let pos = Position::from(input);

        Self {
            start: pos.clone(),
            end: pos,
            kind: Kind::NomError(kind),
        }
    }
}

impl<'a, T> FromExternalError<Span<'a>, T> for Error {
    fn from_external_error(input: Span<'a>, _kind: ErrorKind, _e: T) -> Self {
        let position = Position::from(input);

        Self::new(position.clone(), position, Kind::ToBeDefined)
    }
}
