use anyhow::Result;
use std::env;
//use async_std::task;
//use chrono::Utc;
use firebase_rs::*;
//use notify_rust::Notification;
//use std::sync::atomic::AtomicBool;
//use std::sync::Arc;
//use std::time::Duration;
//use serde_json::{json, Value};

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com";
static PROGNAME: &str = "mobdtimer";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Need one argument, amount of time");
        std::process::exit(1)
    }
    let duration = args[1].parse::<i32>();
    if duration.is_err() {
        eprintln!("argument needs to be numerical.");
        std::process::exit(1)
    }
    println!("{:?}", duration);

    let db: Firebase;
    match firebase() {
        Ok(f) => db = f,
        Err(e) => {
            eprintln!("Firebase connection error:");
            eprintln!("{}", e);
            std::process::exit(1)
        }
    }
}

fn firebase() -> Result<Firebase> {
    Firebase::new(FIREBASE_URL).map_err(|e| e.into())
}
