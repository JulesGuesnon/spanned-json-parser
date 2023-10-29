#![allow(unused)]
#![forbid(unsafe_code)]
#![warn(clippy::all)]

mod parser;

pub mod ser;
pub mod value;

pub use parser::parse;
