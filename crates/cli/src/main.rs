use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    let path = args.get(1).ok_or("Please provide a path").unwrap();

    let json = fs::read_to_string(path).unwrap();

    let parsed = spanned_json_parser::parse(&json);

    match parsed {
        Ok(_) => process::exit(0),
        Err(_) => process::exit(1),
    }
}
