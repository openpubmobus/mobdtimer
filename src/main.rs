use anyhow::Result;
use std::env;
use std::thread;
//use async_std::task;
//use chrono::Utc;
use firebase_rs::*;
//use notify_rust::Notification;
//use std::sync::atomic::AtomicBool;
//use std::sync::Arc;
//use std::time::Duration;
//use serde_json::{json, Value};
//use async_std::task;
use eventsource::reqwest::Client;
use reqwest::Url;
use std::io::{self, Write};
use std::process::Command;

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com/someUID.json";
static PROGNAME: &str = "mobdtimer";
static PROMPT: &str = "mobdtimer> ";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need one argument, amount of time");
        std::process::exit(1)
    }
    let duration = args[1].parse::<i32>();
    if duration.is_err() {
        eprintln!("Time argument needs to be numeric.");
        std::process::exit(1)
    }

    thread::spawn(|| runEventThread());

    runCommandThread()
}

fn runCommandThread() -> ! {
    loop {
        let mut input = String::new();
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.eq("q") {
                    std::process::exit(1)
                }
            }
            Err(error) => eprintln!("error: {:?}", error),
        }
    }
}

fn runEventThread() {
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
*/

fn run_mob_status() -> Result<bool, String> {
    let output = Command::new("mob")
        .arg("status")
        .output()
        .expect("failed to execute process");
    return if output.status.success() {
        let is_mob_programming =
            String::from_utf8_lossy(&output.stdout).contains("are mob programming");
        Ok(is_mob_programming)
    } else {
        Err("error getting mob status".to_string())
    };
}

fn firebase() -> Result<Firebase> {
    Firebase::new(FIREBASE_URL).map_err(|e| e.into())
}
