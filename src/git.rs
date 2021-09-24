use git2::Repository;

use crate::string_util;

pub fn git_repo_url() -> anyhow::Result<String, String> {
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

pub fn normalize_remote(remote: &str) -> String {
    if is_ssh_remote(remote) { normalize_ssh_remote(remote) } else {
        normalize_https_remote(remote)
    }.replace('/', "_").replace('.', "-")
}

fn is_ssh_remote(remote: &str) -> bool {
    remote.contains('@')
}

fn normalize_https_remote(remote: &str) -> String {
    let (_, server_and_path_part) = string_util::split_into_two(remote, "//");
    let (server, path) = string_util::split_into_two(&server_and_path_part, "/");
    format!(
        "{}{}",
        string_util::remove_trailing(&server, ':'),
        string_util::prepend_if_missing(&path, "/")
    )
}

fn normalize_ssh_remote(remote: &str) -> String {
    let (_, server_and_path_part) = string_util::split_into_two(remote, "@");
    let (server, path) = string_util::split_into_two(&server_and_path_part, ":");
    format!("{}{}", server, string_util::prepend_if_missing(&path, "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: How do you nest (aggregate) tests a la JS 'describe'?

    // normalize_remote.
    #[test]
    fn returns_server_slash_path_for_ssh_ref() {
        assert_eq!(
            normalize_remote("git@github.com:/openpubmobus/mobdtimer.git"),
            "github-com_openpubmobus_mobdtimer-git"
        )
    }

    #[test]
    fn returns_server_slash_path_for_ssh_ref_without_slash() {
        assert_eq!(
            normalize_remote("git@github.com:openpubmobus/mobdtimer.git"),
            "github-com_openpubmobus_mobdtimer-git"
        )
    }

    #[test]
    fn returns_server_slash_path_for_https_ref_without_colon() {
        assert_eq!(
            normalize_remote("https://github.com/openpubmobus/mobdtimer.git"),
            "github-com_openpubmobus_mobdtimer-git"
        )
    }

    #[test]
    fn returns_server_slash_path_for_https_ref_with_colon() {
        assert_eq!(
            normalize_remote("https://github.com:/openpubmobus/mobdtimer.git"),
            "github-com_openpubmobus_mobdtimer-git"
        )
    }
}
