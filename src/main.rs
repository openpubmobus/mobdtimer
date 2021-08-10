use anyhow::Result;
use std::env;
use std::thread;
// use firebase_rs::*;
use eventsource::reqwest::Client;
use reqwest::Url;
use std::io::{self, Write};

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com/someUID.json";
// static PROGNAME: &str = "mobdtimer";
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
                match trimmed {
                    "" => continue,
                    "q" => return,
                    _ => {
                        let command_tokens = trimmed.splitn(2, " ").collect::<Vec<&str>>();
                        //println!("{}, {}", command_tokens[0], command_tokens[1]);
                        let (command, arg) = (command_tokens[0], command_tokens[1]);
                        match command {
                            "s" => start_timer(arg.to_string()),
                            _ => println!("invalid command"),
                        }
                    }
                }
            }
            Err(error) => eprintln!("error: {:?}", error),
        }
    }
}

fn start_timer(length: String) {
    println!("starting timer for {} minutes", length)
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

// When we need to update firebase:
/*
let db: Firebase;
match firebase() {
    Ok(f) => db = f,
    Err(e) => {
        eprintln!("Firebase connection error:");
        eprintln!("{}", e);
        std::process::exit(1)
   }
}

// fn firebase() -> Result<Firebase> {
//     Firebase::new(FIREBASE_URL).map_err(|e| e.into())
// }
*/

// fn run_mob_status() -> Result<bool, String> {
//     let output = Command::new("mob")
//         .arg("status")
//         .output()
//         .expect("failed to execute process");
//     return if output.status.success() {
//         let is_mob_programming =
//             String::from_utf8_lossy(&output.stdout).contains("are mob programming");
//         Ok(is_mob_programming)
//     } else {
//         Err("error getting mob status".to_string())
//     }
// }
