//use std::env;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use anyhow::Result;
use chrono::Utc;
use eventsource::event::Event;
use eventsource::reqwest::Client;
use firebase_rs::*;
use reqwest::Url;
//use serde_json::{json, Value};
use serde_json::Value;
use std::time::Duration;

mod git;
mod string_util;

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com";
static PROMPT: &str = "mobdtimer> ";

struct TimerControl {
    last_end_time: Mutex<i64>,
    mutex: Mutex<bool>,
    condvar: Condvar,
}

#[derive(Clone)]
struct Db {
    uid: String,
    connection: Firebase,
}

fn flushed_print(line: &str) {
    println!("{}", line);
    io::stdout().flush().unwrap();
}

fn main() {
    match process_args() {
        Ok(_) => {
            let timer_control = Arc::new(TimerControl {
                last_end_time: Mutex::new(0),
                mutex: Mutex::new(false),
                condvar: Condvar::new(),
            });

            let db_control = Db {
                connection: firebase().unwrap(),
                uid: git::normalize_remote(&git::git_repo_url().unwrap()),
            };
            let db_control_clone = db_control.clone();

            println!("repo key: {}", db_control.uid);

            thread::spawn(move || run_event_thread(&timer_control, &db_control));
            run_command_thread(&db_control_clone, io::stdin().lock(), io::stdout())
        }
        Err(message) => {
            eprintln!("{}", message)
        }
    }
}

// TODO make args optional?
fn process_args() -> Result<i32, String> {
    // let args: Vec<String> = env::args().collect();
    // if args.len() < 2 {
    //     return Err("Timer duration required".to_string());
    // }
    // let duration_result = args[1].parse::<i32>();
    // if duration_result.is_err() {
    //     return Err("Timer duration must be numeric".to_string());
    // }
    // Ok(duration_result.unwrap())
    Ok(0)
}

enum CommandResult {
    Continue,
    Exit,
}

fn run_command_thread<R, W>(db_control: &Db, mut reader: R, mut writer: W)
where
    R: BufRead,
    W: Write,
{
    loop {
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(_) => match handle_command(db_control, &input.trim()) {
                Ok(CommandResult::Exit) => return,
                Ok(_) => continue,
                Err(error) => writeln!(&mut writer, "{}", error).expect("Unable to write"),
            },
            Err(error) => {
                writeln!(&mut writer, "input error: {:?}", error).expect("Unable to write")
            }
        }
    }
}

fn handle_command(db_control: &Db, command: &&str) -> Result<CommandResult, String> {
    match &command.split(' ').collect::<Vec<&str>>()[..] {
        [command] => match *command {
            "" => Ok(CommandResult::Continue),
            "q" => Ok(CommandResult::Exit),
            "k" => {
                db_stop_timer(db_control);
                Ok(CommandResult::Continue)
            }
            _ => Err("invalid command".to_string()),
        },
        [command, arg] => match *command {
            "s" => {
                db_create_timer(arg.to_string(), db_control);
                Ok(CommandResult::Continue)
            }
            _ => Err("invalid command".to_string()),
        },
        _ => Err("too many arguments".to_string()),
    }
}

fn db_create_timer(duration_in_minutes: String, db_control: &Db) {
    // TODO: blow up if not numeric
    let duration = duration_in_minutes.parse::<u64>().unwrap();
    println!(
        "starting timer for {} minutes using repo key {}",
        duration, db_control.uid
    );

    let end_time = store_future_time(db_control, None, duration);
    println!(
        "Timer started, id: {} end_time: {:?}",
        db_control.uid, end_time
    );
}

fn db_stop_timer(db_control: &Db) {
    store_end_time(db_control, &(Utc::now().timestamp() - 1));
    println!("End time in past stored");
}

fn store_future_time(db_control: &Db, given_time: Option<i64>, wait_minutes: u64) -> Result<i64> {
    let start_time_epoch = match given_time {
        Some(time) => time,
        None => Utc::now().timestamp(),
    };

    let end_time_epoch = start_time_epoch + (wait_minutes as i64) * 60;
    store_end_time(db_control, &end_time_epoch);
    Ok(end_time_epoch)
}

fn store_end_time(db_control: &Db, end_time_epoch: &i64) {
    let timer = db_control.connection.at(&db_control.uid).unwrap();
    timer
        .set(&format!("{{\"endTime\":{}}}", end_time_epoch))
        .unwrap();
}

fn firebase() -> Result<Firebase> {
    Firebase::new(FIREBASE_URL).map_err(|e| e.into())
}

fn run_event_thread(timer_control: &Arc<TimerControl>, db_control: &Db) {
    let url = format!("{}/{}.json", FIREBASE_URL, db_control.uid);
    let client = Client::new(Url::parse(&url).unwrap());
    for event in client {
        match event {
            Ok(good_event) => {
                handle_event(good_event, timer_control);
            }
            Err(error) => println!("{:?}", error),
        }
    }
}

fn handle_event(event: Event, timer_control: &Arc<TimerControl>) {
    if let Some(event_type) = event.event_type {
        if event_type.as_str() == "put" {
            let x = format!("put; event id: {:?} >>> {:?}", event.id, event.data);
            flushed_print(&x);
            on_new_event(event.data, timer_control)
        }
        /* else {
            println!("not put; event id {:?} >>> {:?}", event.id, event.data)
        }
        */
    }
}

fn on_new_event(json_payload: String, timer_control: &Arc<TimerControl>) {
    let node: Value = serde_json::from_str(&json_payload).unwrap();
    if let Some(end_time) = node["data"]["endTime"].as_i64() {
        let mut mut_last_end_time = timer_control.last_end_time.lock().unwrap();
        *mut_last_end_time = end_time;
        if end_time > Utc::now().timestamp() {
            flushed_print("starting a timer");
            let timer_control_clone = timer_control.clone();
            thread::spawn(move || start_timer(&timer_control_clone));
        } else {
            flushed_print("end time passed -- killing a timer");
            kill_timer_thread(timer_control);
        }
    }
}

fn kill_timer_thread(timer_control: &Arc<TimerControl>) -> CommandResult {
    let TimerControl {
        last_end_time: _,
        mutex,
        condvar,
    } = &**timer_control;
    let mut kill_timer_flag = mutex.lock().unwrap();
    *kill_timer_flag = true;
    condvar.notify_one();
    CommandResult::Continue
}

fn start_timer(timer_control: &Arc<TimerControl>) {
    let TimerControl {
        last_end_time,
        mutex,
        condvar,
    } = &**timer_control;
    let mut kill_timer_flag = mutex.lock().unwrap();

    let duration_in_seconds =
        (*last_end_time.lock().unwrap() - Utc::now().timestamp()).unsigned_abs();

    loop {
        let result = condvar
            .wait_timeout(kill_timer_flag, Duration::from_secs(duration_in_seconds))
            .unwrap();
        kill_timer_flag = result.0;
        if *kill_timer_flag {
            flushed_print("timer completed");
            break;
        } else if result.1.timed_out() {
            flushed_print("timer completed (timed out)");
            break;
        } else {
            flushed_print("what happened here??");
        }
        // else: spurious wakeup; restart loop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: How do you nest (aggregate) tests a la JS 'describe'?

    // run_command_thread
    #[test]
    fn returns_on_exit_command() {
        let commands = b"q";
        let mut output = Vec::new();

        // TODO what does the input[..] mean
        run_command_thread("", &commands[..], &mut output);

        assert_output(output, "");
    }

    #[test]
    fn prints_error_on_errored_command() {
        let commands = b"somebadcommand\nq";
        let mut output = Vec::new();

        run_command_thread("", &commands[..], &mut output);

        assert_output(output, "invalid command\n");
    }

    fn assert_output(output: Vec<u8>, expected_output: &str) {
        assert_eq!(
            String::from_utf8(output).expect("Not UTF-8"),
            expected_output
        );
    }
}
