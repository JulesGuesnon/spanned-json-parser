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
    NomError(nom::error::ErrorKind),
    // Used when an error will be remaped
    ToBeDefined,
}

// impl Kind {
//     pub fn same(&self, other: &Self) -> bool {
//         #[allow(clippy::match_like_matches_macro)]
//         match (self, other) {
//             (Kind::InvalidString, Kind::InvalidString)
//             | (Kind::MissingQuote, Kind::MissingQuote)
//             | (Kind::InvalidHex(_), Kind::InvalidHex(_))
//             | (Kind::InvalidNumber(_), Kind::InvalidNumber(_))
//             | (Kind::CharsAfterRoot(_), Kind::CharsAfterRoot(_))
//             | (Kind::ToBeDefined, Kind::ToBeDefined)
//             | (Kind::MissingArrayBracket, Kind::MissingArrayBracket)
//             | (Kind::MissingObjectBracket, Kind::MissingObjectBracket)
//             | (Kind::InvalidKey, Kind::InvalidKey)
//             | (Kind::MissingColon, Kind::MissingColon)
//             | (Kind::NomError(_), Kind::NomError(_)) => true,
//             _ => false,
//         }
//     }
// }

#[derive(Debug)]
pub struct Error {
    pub start: Position,
    pub end: Position,
    pub value: Kind,
}

impl Error {
    pub fn new(start: Position, end: Position, value: Kind) -> Self {
        Self { start, end, value }
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
            value: Kind::NomError(kind),
        }
    }

    fn append(input: Span<'a>, kind: ErrorKind, other: Self) -> Self {
        other
    }
}

impl<'a, T> FromExternalError<Span<'a>, T> for Error {
    fn from_external_error(input: Span<'a>, kind: ErrorKind, e: T) -> Self {
        let position = Position::from(input);

        Self::new(position.clone(), position, Kind::ToBeDefined)
    }
}
