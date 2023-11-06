use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    let path = args.get(1).ok_or("Please provide a path").unwrap();

    let json = match fs::read_to_string(path) {
        Ok(str) => str,
        Err(e) => {
            println!("Failed to read string: {}", e);
            process::exit(1)
        }
    };

    let parsed = spanned_json_parser::parse(&json);

    match parsed {
        Ok(_) => process::exit(0),
        Err(_) => process::exit(1),
    }
}
