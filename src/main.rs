use anyhow::Result;
use std::env;
use std::thread;
// use firebase_rs::*;
use eventsource::reqwest::Client;
use git2::Repository;
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
    if is_ssh_remote(remote) {
        normalize_ssh_remote(remote)
    } else {
        normalize_https_remote(remote)
    }
}

fn is_ssh_remote(remote: &str) -> bool {
    remote.contains('@')
}

fn normalize_https_remote(remote: &str) -> String {
    let (_, server_and_path_part) = split_into_two(remote, "//");
    let (server, path) = split_into_two(&server_and_path_part, "/");
    format!("{}{}", remove_trailing(&server, ':'), prepend_slash_if_missing(&path))
}

fn normalize_ssh_remote(remote: &str) -> String {
    let (_, server_and_path_part) = split_into_two(remote, "@");
    let (server, path) = split_into_two(&server_and_path_part, ":");
    format!("{}{}", server, prepend_slash_if_missing(&path))
}

fn split_into_two(s: &str, split_on: &str) -> (String, String) {
    match s.find(split_on) {
        Some(index) =>
            (s[0..index].to_string(), s[index+split_on.len()..].to_string()),
        None =>
            (s.to_string(), "".to_string())
    }
}

fn remove_trailing(s: &str, ch: char) -> String {
    s.split(ch).next().unwrap().to_string()
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

    // normalize_remote. How do you nest (aggregate) tests a la JS 'describe'?
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

    // split_into_two

    #[test]
    fn returns_tuple_for_two_elements() {
        let (first, second) = split_into_two("abc:def", ":");

        assert_eq!(first, "abc");
        assert_eq!(second, "def");
    }

    #[test]
    fn returns_tuple_with_null_2nd_when_split_string_not_found() {
        let (first, second) = split_into_two("abc", ":");

        assert_eq!(first, "abc");
        assert_eq!(second, "");
    }

    #[test]
    fn returns_empty_tuple_when_string_empty() {
        let (first, second) = split_into_two("", ":");

        assert_eq!(first, "");
        assert_eq!(second, "");

        // TODO: Why doesn't this work:
        // let tuple = split_into_two("", ":");
        // assert_eq!(tuple, ("", ""));
    }

    // remove_trailing
    #[test]
    fn returns_same_when_no_trailing_char() {
        assert_eq!(remove_trailing("abc", ':'), "abc");
    }

    #[test]
    fn returns_trailing_char_when_exists() {
        assert_eq!(remove_trailing("abc:", ':'), "abc");
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
