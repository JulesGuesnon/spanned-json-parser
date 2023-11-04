#![allow(unused)]
#![forbid(unsafe_code)]
#![warn(clippy::all)]

mod input;
mod parser;
mod ser;

pub mod error;
pub mod value;

pub use parser::parse;
