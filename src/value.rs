use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum Number {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Null,
    Number(Number),
    String(String),
    Bool(bool),
    Array(Vec<SpannedValue>),
    Object(HashMap<String, SpannedValue>),
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub col: usize,
    pub line: usize,
}

#[derive(Debug, PartialEq)]
pub struct SpannedValue {
    pub value: Value,
    pub start: Position,
    pub end: Position,
}
