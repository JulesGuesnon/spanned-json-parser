use std::{collections::HashMap, fmt::Display};

use nom_locate::LocatedSpan;

#[derive(Debug, PartialEq, Clone)]
pub enum Number {
    PosInt(u64),
    NegInt(i64),
    Float(f64),
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PosInt(num) => write!(f, "{}", num),
            Self::NegInt(num) => write!(f, "{}", num),
            Self::Float(num) => write!(f, "{}", num),
        }
    }
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

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "()"),
            Self::Number(num) => write!(f, "{}", num),
            Self::String(str) => write!(f, "{}", str),
            Self::Bool(bool) => write!(f, "{}", bool),
            Self::Array(array) => write!(f, "{:?}", array),
            Self::Object(object) => write!(f, "{:?}", object),
        }
    }
}

impl Value {
    pub fn unwrap_null(&self) {
        match self {
            Self::Null => (),
            _ => panic!("Try to get null, but value is not null: {}", self),
        }
    }

    pub fn unwrap_string(&self) -> &str {
        match self {
            Self::String(str) => str,
            _ => panic!("Try to get string, but value is not a string: {}", self),
        }
    }

    pub fn unwrap_number(&self) -> &Number {
        match self {
            Self::Number(num) => num,
            _ => panic!("Try to get number, but value is not a number: {}", self),
        }
    }

    pub fn unwrap_bool(&self) -> bool {
        match self {
            Self::Bool(bool) => *bool,
            _ => panic!("Try to get bool, but value is not a bool: {}", self),
        }
    }

    pub fn unwrap_array(&self) -> &Vec<SpannedValue> {
        match self {
            Self::Array(array) => array,
            _ => panic!("Try to get array, but value is not a array: {}", self),
        }
    }

    pub fn unwrap_object(&self) -> &HashMap<String, SpannedValue> {
        match self {
            Self::Object(obj) => obj,
            _ => panic!("Try to get object, but value is not a object: {}", self),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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

impl Display for SpannedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: nom::AsBytes> From<LocatedSpan<T>> for Position {
    fn from(val: LocatedSpan<T>) -> Self {
        Position {
            line: val.location_line() as usize,
            col: val.naive_get_utf8_column() - 1,
        }
    }
}
