use crate::input::Input;
use std::{collections::HashMap, fmt::Display};
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

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

#[derive(Debug, PartialEq, Clone)]
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

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Position {
    pub col: usize,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone)]
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

impl Position {
    pub fn from_ahead(val: Input<'_>) -> Self {
        Self {
            line: val.location_line(),
            // Often times, we retrieve the position after the start or end char
            // has already been eaten, so we need to go back by 1
            col: val.get_utf8_column() - 1,
        }
    }
}

impl<'a> From<Input<'a>> for Position {
    fn from(val: Input<'a>) -> Self {
        Self {
            line: val.location_line(),
            col: val.get_utf8_column(),
        }
    }
}
