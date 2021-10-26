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
use serde_json::{json, Value};
use std::time::Duration;

mod git;
mod string_util;

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com";
static PROMPT: &str = "mobdtimer> ";

fn main() {
    let timer_control = Arc::new((Mutex::new(false), Condvar::new()));

    match process_args() {
        Ok(result) => {
            // println!("specified timer for {:?}", result);

            let repo_url = git::normalize_remote(&git::git_repo_url().unwrap());
            let repo_url_clone = repo_url.clone();
            println!("repo key: {}", repo_url);

            let timer_control_clone = Arc::clone(&timer_control);
            thread::spawn(move || run_event_thread(repo_url_clone, &timer_control));
            run_command_thread(
                &repo_url,
                &timer_control_clone,
                io::stdin().lock(),
                io::stdout(),
            )
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

fn run_command_thread<R, W>(
    repo_url: &str,
    timer_control: &Arc<(Mutex<bool>, Condvar)>,
    mut reader: R,
    mut writer: W,
) where
    R: BufRead,
    W: Write,
{
    loop {
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(_) => match handle_command(repo_url, timer_control, &input.trim()) {
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

fn handle_command(
    repo_url: &str,
    timer_control: &Arc<(Mutex<bool>, Condvar)>,
    command: &&str,
) -> Result<CommandResult, String> {
    match &command.split(' ').collect::<Vec<&str>>()[..] {
        [command] => match *command {
            "" => Ok(CommandResult::Continue),
            "q" => Ok(CommandResult::Exit),
            "k" => Ok(kill_timer(timer_control)),
            _ => Err("invalid command".to_string()),
        },
        [command, arg] => match *command {
            "s" => {
                create_timer(arg.to_string(), repo_url);
                Ok(CommandResult::Continue)
            }
            _ => Err("invalid command".to_string()),
        },
        _ => Err("too many arguments".to_string()),
    }
}

fn create_timer(duration_in_minutes: String, repo_url: &str) {
    // TODO: blow up if not numeric
    let duration = duration_in_minutes.parse::<u64>().unwrap();
    println!(
        "starting timer for {} minutes using repo key {}",
        duration, repo_url
    );

    let firebase_conn = firebase().unwrap();

    let end_time = store_future_time(&firebase_conn, None, duration, repo_url);
    println!("Timer started, id: {} end_time: {:?}", &repo_url, end_time);
}

fn store_future_time(
    firebase: &Firebase,
    given_time: Option<i64>,
    wait_minutes: u64,
    uid: &str,
) -> Result<i64> {
    let start_time_epoch = match given_time {
        Some(time) => time,
        None => Utc::now().timestamp(),
    };

    let end_time = start_time_epoch + (wait_minutes as i64) * 60;
    let timer = firebase.at(uid)?;
    timer.set(&format!("{{\"endTime\":{}}}", end_time))?;

    Ok(end_time)
}

fn firebase() -> Result<Firebase> {
    Firebase::new(FIREBASE_URL).map_err(|e| e.into())
}

fn run_event_thread(repo_url: String, timer_control: &Arc<(Mutex<bool>, Condvar)>) {
    let url = format!("{}/{}.json", FIREBASE_URL, repo_url);
    let client = Client::new(Url::parse(&url).unwrap());
    for event in client {
        match event {
            Ok(good_event) => {
                handle_event(good_event, timer_control);
                //print!("{}", PROMPT);
                io::stdout().flush().unwrap();
            }
            Err(error) => println!("{:?}", error),
        }
    }
}

fn handle_event(event: Event, timer_control: &Arc<(Mutex<bool>, Condvar)>) {
    if let Some(event_type) = event.event_type {
        if event_type.as_str() == "put" {
            handle_put(event.data, timer_control)
        }
    }
}

fn handle_put(json_payload: String, timer_control: &Arc<(Mutex<bool>, Condvar)>) {
    let node: Value = serde_json::from_str(&json_payload).unwrap();
    if let Some(end_time) = node["data"]["endTime"].as_i64() {
        if end_time > Utc::now().timestamp() {
            start_timer(end_time, timer_control);
        }
    }
}

fn kill_timer(timer_control: &Arc<(Mutex<bool>, Condvar)>) -> CommandResult {
    let (lock, cvar) = &**timer_control;
    let mut started = lock.lock().unwrap();
    *started = true;
    cvar.notify_one();
    CommandResult::Continue
}

fn start_timer(end_time: i64, timer_control: &Arc<(Mutex<bool>, Condvar)>) {
    // wait for the thread to start up
    let (lock, cvar) = &**timer_control;
    let mut started = lock.lock().unwrap();
    // as long as the value inside the `Mutex<bool>` is `false`, we wait
    let duration = (end_time - Utc::now().timestamp()).unsigned_abs();
    loop {
        let result = cvar
            .wait_timeout(started, Duration::from_secs(duration))
            .unwrap();
        started = result.0;
        if *started {
            // We received the notification and the value has been updated, we can leave.
            println!("Got Interrupted");
            break;
        } else if result.1.timed_out() {
            println!("WE'RE DONE");
            break;
        }
    }
}

/*
async fn sleep_until_end_time(wakeup_time_epoch: i64) {
    let sleep_seconds = wakeup_time_epoch - Utc::now().timestamp();
    task::block_on(async move { task::sleep(Duration::from_secs(sleep_seconds as u64)).await });
    println!("timer elapsed");
}
*/

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
