#![allow(unused)]
#![forbid(unsafe_code)]
#![warn(clippy::all)]

mod error;
mod parser;
mod ser;

pub mod value;
pub use parser::parse;
