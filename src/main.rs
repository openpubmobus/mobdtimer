use anyhow::Result;
use std::env;
use std::thread;
// use firebase_rs::*;
use eventsource::reqwest::Client;
use git2::{Repository, StatusOptions};
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
                    // TODO: require arg for s somehow
                    &[command, arg] => match command {
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
    let timer_key = git_repo_url().unwrap();
    println!(
        "starting timer for {} minutes using repo key {}",
        length, timer_key
    );
}

fn git_repo_url() -> Result<String, String> {
    return match Repository::open(".") {
        Ok(repo) => {
            /*
            repo.remotes().iter().for_each(| remote |
                remote.iter().for_each(| r | println!("{}", r.unwrap())
            ));
            */
            println!(
                "REMOTE: {}",
                repo.find_remote("origin").unwrap().url().unwrap()
            );
            Ok("xxx".to_string())
        }
        Err(error) => {
            eprintln!("error: {:?}", error);
            Err("wtf".to_string())
        }
    };
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

fn normalize_remote(remote: &str) -> String {
    let mut remote_parts: Vec<&str> = remote.split('@').collect();
    if remote_parts.len() == 2 {
        let server_and_path_part = remote_parts[1].to_string();
        let server_and_path: Vec<&str> = server_and_path_part.split(':').collect();
        let server = server_and_path[0];
        let path = server_and_path[1];
        format!("{}{}", server, prepend_slash_if_missing(path))
    } else {
        remote_parts = remote.split("//").collect();
        let server_and_path_part = remote_parts[1];
        let server_and_path: Vec<&str> = server_and_path_part.split('/').collect();
        let server = server_and_path[0];
        let path = server_and_path[1];
        format!("{}{}", remove_trailing_colon_if_exists(server), prepend_slash_if_missing(path))
    }
}

fn remove_trailing_colon_if_exists(server: &str) -> String {
    let server_parts: Vec<&str> = server.split(':').collect();
    server_parts[0].to_string()
}

fn prepend_slash_if_missing(path: &str) -> String {
    if !path.starts_with("/") {
        format!("/{}", path)
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_server_slash_path_for_ssh_ref() {
        assert_eq!(
            normalize_remote("git@github.com:/openpubmobus/mobdtimer.git"),
            "github.com/openpubmobus/mobdtimer.git"
        )
    }

    #[test]
    fn returns_server_slash_path_for_ssh_ref_without_slash() {
        assert_eq!(
            normalize_remote("git@github.com:openpubmobus/mobdtimer.git"),
            "github.com/openpubmobus/mobdtimer.git"
        )
    }

    #[test]
    fn returns_server_slash_path_for_https_ref_without_colon() {
        assert_eq!(
            normalize_remote("https://github.com/openpubmobus/mobdtimer.git"),
            "github.com/openpubmobus/mobdtimer.git"
        )
    }

    #[test]
    fn returns_server_slash_path_for_https_ref_with_colon() {
        assert_eq!(
            normalize_remote("https://github.com:/openpubmobus/mobdtimer.git"),
            "github.com/openpubmobus/mobdtimer.git"
        )
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
