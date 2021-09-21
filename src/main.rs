use anyhow::Result;
use chrono::Utc;
use eventsource::reqwest::Client;
use firebase_rs::*;
use git2::Repository;
use reqwest::Url;
use std::env;
use std::io::{self, Write};
use std::thread;

static FIREBASE_URL: &str = "https://rust-timer-default-rtdb.firebaseio.com";
static PROMPT: &str = "mobdtimer> ";

fn main() {
    let repo_url = normalize_remote(&git_repo_url().unwrap());
    match process_args() {
        Ok(result) => {
            println!("starting timer for {:?}", result);
            let repo_url_clone = repo_url.clone();
            thread::spawn(|| run_event_thread(repo_url_clone));
            run_command_thread(&repo_url)
        }
        Err(message) => {
            eprintln!("{}", message)
        }
    }
}

// TODO make args optional?
fn process_args() -> Result<i32, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err("Timer duration required".to_string());
    }
    let duration_result = args[1].parse::<i32>();
    if duration_result.is_err() {
        return Err("Timer duration must be numeric".to_string());
    }
    Ok(duration_result.unwrap())
}

enum CommandResult { Continue, Exit }

fn run_command_thread(repo_url: &str) {
    loop {
        let mut input = String::new();
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        match io::stdin().read_line(&mut input) {
            Ok(_) =>
                match handle_command(repo_url, &input.trim()) {
                    Ok(CommandResult::Exit) => return,
                    Ok(_) => continue,
                    Err(error) => eprintln!("command error: {:?}", error)
                },
            Err(error) => eprintln!("input error: {:?}", error),
        }
    }
}

fn handle_command(repo_url: &str, command: &&str) -> Result<CommandResult, String> {
    match &command.split(' ').collect::<Vec<&str>>()[..] {
        [command] => match *command {
            ""  => Ok(CommandResult::Continue),
            "q" => Ok(CommandResult::Exit),
            _   => Err("invalid command".to_string())
        },
        [command, arg] => match *command {
            "s" => { start_timer(arg.to_string(), repo_url);
                     Ok(CommandResult::Continue) },
            _ =>   Err("invalid command".to_string()),
        },
        _ => Err("too many arguments".to_string()),
    }
}

fn start_timer(duration_in_minutes: String, repo_url: &str) {
    // TODO: blow up if not numeric
    let duration = duration_in_minutes.parse::<u64>().unwrap();
    println!("starting timer for {} minutes using repo key {}", duration, repo_url);

    let firebase_conn = firebase().unwrap();

    let end_time = store_future_time(&firebase_conn, None, duration, &repo_url);
    println!("Timer started, id: {}", &repo_url);
    // task::block_on(task::spawn(notify_at(
    //     end_time.unwrap(),
    //     notification,
    //     Arc::new(AtomicBool::new(false)),
    // )));
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
                println!("\n========{}========\n", good_event);
                print!("{}", PROMPT);
                io::stdout().flush().unwrap();
            }
            Err(error) => println!("{:?}", error),
        }
    }
}

// TODO: move these to a git module
fn git_repo_url() -> Result<String, String> {
    return match Repository::open(".") {
        Ok(repo) => Ok(repo
            .find_remote("origin")
            .unwrap()
            .url()
            .unwrap()
            .to_string()),
        Err(error) => {
            eprintln!("error: {:?}", error);
            Err("wtf".to_string())
        }
    };
}

fn normalize_remote(remote: &str) -> String {
    if is_ssh_remote(remote) {
        normalize_ssh_remote(remote)
    } else {
        normalize_https_remote(remote)
    };
    let remote = remote.replace('/', "_");
    remote.replace('.', "-")
}

fn is_ssh_remote(remote: &str) -> bool {
    remote.contains('@')
}

fn normalize_https_remote(remote: &str) -> String {
    let (_, server_and_path_part) = split_into_two(remote, "//");
    let (server, path) = split_into_two(&server_and_path_part, "/");
    format!(
        "{}{}",
        remove_trailing(&server, ':'),
        prepend_if_missing(&path, "/")
    )
}

fn normalize_ssh_remote(remote: &str) -> String {
    let (_, server_and_path_part) = split_into_two(remote, "@");
    let (server, path) = split_into_two(&server_and_path_part, ":");
    format!("{}{}", server, prepend_if_missing(&path, "/"))
}

// TODO: move these to a string utility module
fn split_into_two(s: &str, split_on: &str) -> (String, String) {
    match s.find(split_on) {
        Some(index) => (
            s[0..index].to_string(),
            s[index + split_on.len()..].to_string(),
        ),
        None => (s.to_string(), "".to_string()),
    }
}

fn remove_trailing(s: &str, ch: char) -> String {
    s.split(ch).next().unwrap().to_string()
}

fn prepend_if_missing(s: &str, prepend: &str) -> String {
    if s.starts_with(prepend) {
        s.to_string()
    } else {
        format!("{}{}", prepend, s)
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
        assert_eq!(
            split_into_two("tuv:wxy", ":"),
            ("tuv".to_string(), "wxy".to_string())
        );
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

    // prepend_if_missing
    #[test]
    fn returns_same_when_char_in_first_position() {
        assert_eq!(prepend_if_missing("/abc", "/"), "/abc");
    }

    #[test]
    fn prepends_char_when_not_exists_in_first_position() {
        assert_eq!(prepend_if_missing("abc", "/"), "/abc");
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


*/
