use crate::{parser::Span, value::Position};
use nom::error::{ErrorKind, FromExternalError, ParseError};
use std::num::IntErrorKind;
use std::num::ParseIntError;

pub enum Kind {
    Invalid(String),
    Missing(String),
    InvalidHex(String),
    NomError(String),
    InvalidNumber(String),
}

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

impl<'a> ParseError<Span<'a>> for Error {
    fn from_error_kind(input: Span<'a>, kind: ErrorKind) -> Self {
        let position = Position::from(input);
        Self {
            start: position.clone(),
            end: position,
            value: Kind::NomError(kind.description().to_owned()),
        }
    }

    fn append(input: Span<'a>, kind: ErrorKind, other: Self) -> Self {
        other
    }
}

impl<'a> FromExternalError<Span<'a>, ParseIntError> for Error {
    fn from_external_error(input: Span<'a>, kind: ErrorKind, e: ParseIntError) -> Self {
        let position = Position::from(input);

        match e.kind() {
            IntErrorKind::Empty => Self::new(
                position,
                position,
                Kind::InvalidNumber("Failed to parse the number. Reason: empty".to_owned()),
            ),
            IntErrorKind::InvalidDigit => todo!(),
            IntErrorKind::PosOverflow => todo!(),
            IntErrorKind::NegOverflow => todo!(),
            IntErrorKind::Zero => todo!(),
            _ => todo!(),
        }
    }
}
