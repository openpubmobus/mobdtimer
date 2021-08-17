use anyhow::Result;
use std::env;
use std::thread;
// use firebase_rs::*;
use eventsource::reqwest::Client;
use reqwest::Url;
use std::io::{self, Write};

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com/someUID.json";
static PROMPT: &str = "mobdtimer> ";

fn main() {
    let result = process_args();
    match result {
        Ok(result) => {
            println!("starting timer for {:?}", result);
            thread::spawn(|| run_event_thread());
            run_command_thread()
        }
        Err(message) => {
            eprintln!("{}", message)
        }
    }
}

fn process_args() -> Result<i32, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err("Timer duration required".to_string());
    }
    let duration_result = args[1].parse::<i32>();
    if duration_result.is_err() {
        return Err("Timer duration must be numeric".to_string());
    }
    return Ok(duration_result.unwrap());
}

fn run_command_thread() {
    loop {
        let mut input = String::new();
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim();
                match &trimmed.split(" ").collect::<Vec<&str>>()[..] {
                    &[command] => match command {
                        "" => continue,
                        "q" => return,
                        _ => println!("invalid command"),
                    },

                    &[command, arg] => match command {
                        "a" => abort_timer(),
                        "s" => start_timer(arg.to_string()),
                        _ => println!("invalid command"),
                    },
                    _ => println!("invalid command X"),
                }
            }
            Err(error) => eprintln!("error: {:?}", error),
        }
    }
}

fn start_timer(length: String) {
    println!("starting timer for {} minutes", length)
}

fn abort_timer() {
    println!("aborting current timer, if any")
}

fn run_event_thread() {
    let client = Client::new(Url::parse(FIREBASE_URL).unwrap());
    for event in client {
        match event {
            Ok(good_event) => {
                println!("\n========{}========\n", good_event);
                print!("{}", PROMPT);
                io::stdout().flush().unwrap();
            }
            Err(error) => println!("{:?}", error),
        }
    }
}


/* // code for interacting with firebase:
let db: Firebase;
match firebase() {
    Ok(f) => db = f,
    Err(e) => {
        eprintln!("Firebase connection error:");
        eprintln!("{}", e);
        std::process::exit(1)
   }
}

fn firebase() -> Result<Firebase> {
    Firebase::new(FIREBASE_URL).map_err(|e| e.into())
}
*/
