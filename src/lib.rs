#![forbid(unsafe_code)]
#![warn(clippy::all)]
#![allow(clippy::needless_doctest_main)]
//! This crate is a json parser that will return span information for values, which mean lines and column number. It is also compatible with [serde](https://serde.rs/) so you can serialize it to any other struct that implements [Deserialize](https://docs.rs/serde/latest/serde/de/trait.Deserialize.html)
//!
//! ## Why use it ?
//!
//! One of the main use case is to do validation after parsing. By having the line and col number, you can tell really precisely to user where a value is invalid
//!
//! ## How to use it ?
//!
//! The crate expose a `Value` that is similar to [serde](https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html), and wraps everything into this struct:
//!
//! ```rust
//! pub struct Position {
//!     pub col: usize,
//!     pub line: usize,
//! }
//!
//! pub struct SpannedValue {
//!     pub value: Value,
//!     pub start: Position,
//!     pub end: Position,
//! }
//! ```
//!
//! ### Parsing
//!
//! ```rust
//! use spanned_json_parse::parse;
//! use std::fs;
//!
//! fn main() {
//!     let json = fs::read_to_string("path").unwrap();
//!
//!     let parsed = parse(&json);
//!
//!     println!("Parsed: {:#?}", parsed);
//! }
//! ```
//!
//! ### Serializing in a struct
//!
//! ```rust
//! use serde::Deserialize;
//! use spanned_json_parser::parse;
//!
//! #[derive(Deserialize)]
//! struct Test {
//!     pub hello: String,
//! }
//!
//! fn main() {
//!     let json = r#"{"hello": "world"}"#;
//!
//!     let parsed = parse(json).unwrap();
//!
//!     let test: Test = serde_json::from_value(serde_json::to_value(parsed).unwrap()).unwrap();
//!
//!     println!("Test hello: {}", test.hello);
//! }
//! ```
//!
//! ## Performance
//!
//! Here are the outputs of the benchmark. Everything was tested on a Macbook Pro M1, so keep in mind that this numbers are here to give you an idea of the performance, but might not be representative of the reality:
//!
//! ```
//! Parser ./benches/data/twitter.json
//! time:   [10.220 ms 10.279 ms 10.334 ms]
//! thrpt:  [58.280 MiB/s 58.589 MiB/s 58.932 MiB/s]
//!
//! Parser ./benches/data/citm_catalog.json
//! time:   [18.204 ms 18.281 ms 18.353 ms]
//! thrpt:  [89.752 MiB/s 90.102 MiB/s 90.486 MiB/s]
//!
//! Parser ./benches/data/canada.json
//! time:   [42.026 ms 42.188 ms 42.341 ms]
//! thrpt:  [50.702 MiB/s 50.886 MiB/s 51.082 MiB/s]
//! ```

extern crate bytecount;
extern crate memchr;
extern crate nom;
extern crate serde;

mod input;
mod parser;
mod ser;

pub mod error;
pub mod value;

pub use parser::parse;
