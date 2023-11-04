use std::{env, fs, process, time::Instant};

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

    let start = Instant::now();
    println!("Starting parsing: {:?}", start);

    let parsed = serde_json::from_str::<serde_json::Value>(&json);
    // let parsed = spanned_json_parser::parse(&json);

    println!("Ended parsing: {:?}", start.elapsed());
    match parsed {
        Ok(_) => process::exit(0),
        Err(_) => process::exit(1),
    }
}
