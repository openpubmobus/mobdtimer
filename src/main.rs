//use std::env;
use async_std::task;
use std::io::{self, BufRead, Write};
use std::thread;

use anyhow::Result;
use chrono::Utc;
use eventsource::reqwest::Client;
use firebase_rs::*;
use reqwest::Url;
use std::time::Duration;

mod git;
mod string_util;

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com";
static PROMPT: &str = "mobdtimer> ";

fn main() {
    //start_timer(Utc::now().timestamp() + 10);
    match process_args() {
        Ok(result) => {
            println!("starting timer for {:?}", result);

            let repo_url = git::normalize_remote(&git::git_repo_url().unwrap());
            let repo_url_clone = repo_url.clone();
            println!("repo key: {}", repo_url);

            thread::spawn(|| run_event_thread(repo_url_clone));

            run_command_thread(&repo_url, io::stdin().lock(), io::stdout())
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

fn run_command_thread<R, W>(repo_url: &str, mut reader: R, mut writer: W)
where
    R: BufRead,
    W: Write,
{
    loop {
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        match reader.read_line(&mut input) {
            Ok(_) => match handle_command(repo_url, &input.trim()) {
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

fn handle_command(repo_url: &str, command: &&str) -> Result<CommandResult, String> {
    match &command.split(' ').collect::<Vec<&str>>()[..] {
        [command] => match *command {
            "" => Ok(CommandResult::Continue),
            "q" => Ok(CommandResult::Exit),
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

fn run_event_thread(repo_url: String) {
    let url = format!("{}/{}.json", FIREBASE_URL, repo_url);
    let client = Client::new(Url::parse(&url).unwrap());
    for event in client {
        match event {
            Ok(good_event) => {
                if let Some(event_type) = good_event.event_type {
                    match event_type.as_str() {
                        "put" => println!("===> {:?} <===", good_event.data),
                        _ => (),
                    }
                }
                //print!("{}", PROMPT);
                io::stdout().flush().unwrap();
            }
            Err(error) => println!("{:?}", error),
        }
    }
}

fn start_timer(end_time: i64) {
    println!("I was here, start_timer");
    task::spawn(notify_at(end_time));
}

async fn notify_at(wakeup_time_epoch: i64) {
    println!("I was here, notify_at");
    let sleep_seconds = wakeup_time_epoch - Utc::now().timestamp();
    task::block_on(async move { task::sleep(Duration::from_secs(sleep_seconds as u64)).await });
    //callback();
    println!("TIMER FINISHED");
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
