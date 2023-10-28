use std::collections::HashMap;

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
