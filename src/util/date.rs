use chrono::{DateTime, Utc};

pub fn get_date() -> String {
    Utc::now().format("%F").to_string()
}

pub fn get_date_epoch() -> String {
    DateTime::UNIX_EPOCH.naive_utc().format("%F").to_string()
}


pub fn is_date_iso8601(date: String) -> bool {
    let vec_date = date.into_bytes();
    vec_date.len() == 10 &&
    vec_date[0].is_ascii_digit() &&
    vec_date[1].is_ascii_digit() &&
    vec_date[2].is_ascii_digit() &&
    vec_date[3].is_ascii_digit() &&
    vec_date[4] == 45 &&
    vec_date[5].is_ascii_digit() &&
    vec_date[6].is_ascii_digit() &&
    vec_date[4] == 45 &&
    vec_date[8].is_ascii_digit() &&
    vec_date[9].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_function_is_valid() {
        assert!(is_date_iso8601("2026-08-12".to_string()));
        assert!(!is_date_iso8601("2026-0a-12".to_string()));
        assert!(!is_date_iso8601("2026-08--12".to_string()));
        assert!(!is_date_iso8601("2026/08/12".to_string()));
        assert!(!is_date_iso8601("2026-8-12".to_string()));
        assert!(!is_date_iso8601("26-08-12".to_string()));
    }

    #[test]
    fn make_sure_valid() {
        assert!(is_date_iso8601(get_date()));
    }

    #[test]
    fn make_sure_epoch_valid() {
        assert!(is_date_iso8601(get_date_epoch()));
    }
}