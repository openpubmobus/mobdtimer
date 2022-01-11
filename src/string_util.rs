pub fn split_into_two(s: &str, split_on: &str) -> (String, String) {
    match s.find(split_on) {
        Some(index) => (
            s[0..index].to_string(),
            s[index + split_on.len()..].to_string(),
        ),
        None => (s.to_string(), "".to_string()),
    }
}

pub fn remove_trailing(s: &str, ch: char) -> String {
    s.split(ch).next().unwrap().to_string()
}

pub fn prepend_if_missing(s: &str, prepend: &str) -> String {
    if s.starts_with(prepend) {
        s.to_string()
    } else {
        format!("{}{}", prepend, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod split_into_two_tests {
        use super::*;

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
    }

    mod remove_trailing_tests {
        use super::*;

        #[test]
        fn returns_same_when_no_trailing_char() {
            assert_eq!(remove_trailing("abc", ':'), "abc");
        }

        #[test]
        fn returns_trailing_char_when_exists() {
            assert_eq!(remove_trailing("abc:", ':'), "abc");
        }
    }

    mod prepend_if_missing_tests {
        use super::*;

        #[test]
        fn returns_same_when_char_in_first_position() {
            assert_eq!(prepend_if_missing("/abc", "/"), "/abc");
        }

        #[test]
        fn prepends_char_when_not_exists_in_first_position() {
            assert_eq!(prepend_if_missing("abc", "/"), "/abc");
        }
    }
}