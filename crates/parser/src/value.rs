use std::collections::HashMap;

use nom_locate::LocatedSpan;

#[derive(Debug, PartialEq)]
pub enum Number {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    Null,
    Number(Number),
    String(&'a str),
    Bool(bool),
    Array(Vec<SpannedValue<'a>>),
    Object(HashMap<&'a str, SpannedValue<'a>>),
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub col: usize,
    pub line: usize,
}

#[derive(Debug, PartialEq)]
pub struct SpannedValue<'a> {
    pub value: Value<'a>,
    pub start: Position,
    pub end: Position,
}

impl<T: nom::AsBytes> From<LocatedSpan<T>> for Position {
    fn from(val: LocatedSpan<T>) -> Self {
        Position {
            col: val.location_line() as usize,
            line: val.naive_get_utf8_column(),
        }
    }
}
