use std::{fs};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use anyhow::anyhow;
use tokio::sync::Mutex;
use chrono::{DateTime, NaiveDateTime, Timelike, Utc};
use spdlog::error;
use crate::util::config::ServerConfig;
use rand::prelude::*;
use rusqlite::{params, Connection};
use crate::util::graceful_shutdown::{instant_kill_program, kill_signal_received};

static DAILY_SEED_CACHE: OnceLock<Mutex<DailySeedCache>> = OnceLock::new();

/// Cache for daily seeds, not internally sync'd since its externally sync'd with a mutex
struct DailySeedCache {
    /// Current daily seed, checked each call if its still valid
    current_daily_seed: i64,

    /// Actual day current_daily_seed belongs to (to know if we need to rotate it)
    current_day: NaiveDateTime,

    /// SQLite database for the cache
    connection: Connection,
}

impl DailySeedCache {
    /// Standardized way to get date
    fn get_date() -> NaiveDateTime {
        Utc::now().date_naive().into()
    }

    /// Standardized way to get the date as a string
    fn get_date_string() -> String {
        Self::get_date().to_string()
    }

    /// Tries to find a daily seed for a given day in the daily seed cache dir
    fn try_to_get_cached_seed(&self) -> Option<i64> {
        
    }

    /// Writes a new daily seed to the cache dir and sets the atomic daily seed
    fn flush_new_seed(&mut self) -> Result<i64, anyhow::Error> {
        let seed: i64 = rand::rng().random();
        self.connection.execute("INSERT INTO seed_cache (date, seed) VALUES (?1, ?2)", params![Self::get_date_string(), seed])?;
        Ok(seed)
    }

    /// Init empty new daily seed cache
    pub fn new(seed_cache_db_fname: &str) -> Self {
        let connection = match Connection::open(Path::new(seed_cache_db_fname)) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to open daily seed cache db connection, {}", e);
                instant_kill_program();
            }
        };
        if let Err(e) = connection.execute("CREATE TABLE IF NOT EXISTS seed_cache (date TEXT PRIMARY KEY, seed INTEGER);", params![]) {
            error!("Failed to create seed cache table, {}", e);
            instant_kill_program();
        }
        DailySeedCache {
            current_daily_seed: 0,
            current_day: DateTime::UNIX_EPOCH.date_naive().into(),
            connection,
        }
    }

    /// Top-level get the current seed, dw about the details
    pub fn get_daily_seed(&mut self) -> Result<i64, anyhow::Error> {
        if Self::get_date() == self.current_day {
            return Ok(self.current_daily_seed)
        }

        match self.try_to_get_cached_seed() {
            Some(seed) => {
                self.current_day = Self::get_date();
                self.current_daily_seed = seed;
                Ok(seed)
            },
            None => {
                let seed = self.flush_new_seed()?;
                self.current_day = Self::get_date();
                self.current_daily_seed = seed;
                Ok(seed)
            }
        }
    }
}

/// Top-level get daily seed function. Can fail if the filesystem can't get written to
pub async fn get_current_seed() -> Result<i64, anyhow::Error> {
    let mut daily_cache = match DAILY_SEED_CACHE.get() {
        Some(lock) => lock.lock().await,
        None => return Err(anyhow!("Failed to get once lock, programming error."))
    };
    daily_cache.get_daily_seed()
}

/// Initialize the daily seed thread
pub async fn init_daily_seed_task(config: &ServerConfig) -> Result<(), anyhow::Error> {
    DAILY_SEED_CACHE.get_or_init(|| Mutex::new(DailySeedCache::new(config.daily_seed_cache_db.as_str())));

    // Tries to make a new seed every hour to force cache to flush
    tokio::spawn(async {
        let mut last_yap_time = Utc::now().hour();
        loop {
            if Utc::now().hour() != last_yap_time {
                last_yap_time = Utc::now().hour();
                let _ = get_current_seed().await;
            }

            if kill_signal_received() {
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    Ok(())
}
/*
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cache_at(dir: &Path) -> DailySeedCache {
        DailySeedCache::new(dir.to_str().unwrap())
    }

    #[test]
    fn test_new_starts_with_epoch_day_and_zero_seed() {
        let dir = tempdir().unwrap();
        let cache = cache_at(dir.path());
        assert_eq!(cache.current_daily_seed, 0);
        assert_eq!(cache.current_day, DateTime::UNIX_EPOCH.date_naive().into());
    }

    #[test]
    fn test_try_to_get_cached_seed_returns_none_when_missing() {
        let dir = tempdir().unwrap();
        let cache = cache_at(dir.path());
        assert_eq!(cache.try_to_get_cached_seed(), None);
    }

    #[test]
    fn test_try_to_get_cached_seed_returns_value_when_present() {
        let dir = tempdir().unwrap();
        let cache = cache_at(dir.path());
        let file_path = dir.path().join(DailySeedCache::get_date_string());
        fs::write(&file_path, "12345").unwrap();
        assert_eq!(cache.try_to_get_cached_seed(), Some(12345));
    }

    #[test]
    fn test_try_to_get_cached_seed_returns_none_on_corrupt_data() {
        let dir = tempdir().unwrap();
        let cache = cache_at(dir.path());
        let file_path = dir.path().join(DailySeedCache::get_date_string());
        fs::write(&file_path, "not_a_number").unwrap();
        assert_eq!(cache.try_to_get_cached_seed(), None);
    }

    #[test]
    fn test_flush_new_seed_writes_file_matching_returned_seed() {
        let dir = tempdir().unwrap();
        let mut cache = cache_at(dir.path());
        let seed = cache.flush_new_seed().unwrap();

        let file_path = dir.path().join(DailySeedCache::get_date_string());
        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents.parse::<i64>().unwrap(), seed);
    }

    #[test]
    fn test_flush_new_seed_errors_if_dir_missing() {
        let mut cache = DailySeedCache::new("/nonexistent/path/that/should/not/exist");
        assert!(cache.flush_new_seed().is_err());
    }

    #[test]
    fn test_get_daily_seed_uses_in_memory_value_for_current_day() {
        let dir = tempdir().unwrap();
        let mut cache = cache_at(dir.path());
        cache.current_day = DailySeedCache::get_date();
        cache.current_daily_seed = 42;
        assert_eq!(cache.get_daily_seed().unwrap(), 42);
    }

    #[test]
    fn test_get_daily_seed_reads_existing_cache_file() {
        let dir = tempdir().unwrap();
        let mut cache = cache_at(dir.path());
        let file_path = dir.path().join(DailySeedCache::get_date_string());
        fs::write(&file_path, "777").unwrap();

        let seed = cache.get_daily_seed().unwrap();
        assert_eq!(seed, 777);
        assert_eq!(cache.current_day, DailySeedCache::get_date());
        assert_eq!(cache.current_daily_seed, 777);
    }

    #[test]
    fn test_get_daily_seed_generates_new_seed_if_absent() {
        let dir = tempdir().unwrap();
        let mut cache = cache_at(dir.path());

        let seed = cache.get_daily_seed().unwrap();
        assert_eq!(cache.current_daily_seed, seed);

        let file_path = dir.path().join(DailySeedCache::get_date_string());
        assert!(file_path.exists());
        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents.parse::<i64>().unwrap(), seed);
    }

    #[test]
    fn test_get_daily_seed_is_stable_across_repeated_calls_same_day() {
        let dir = tempdir().unwrap();
        let mut cache = cache_at(dir.path());

        let first = cache.get_daily_seed().unwrap();
        let second = cache.get_daily_seed().unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn test_init_daily_seed_task_and_get_current_seed() {
        let base = tempdir().unwrap();
        let cache_dir = base.path().join("seed_cache"); // must not exist yet

        let config = ServerConfig {
            port: 0,
            log_filename: String::new(),
            ignore_filename: String::new(),
            daily_seed_cache: cache_dir.to_str().unwrap().to_string(),
            forbidden_filename: String::new(),
            not_found_filename: String::new(),
            unauthorized_filename: String::new(),
        };

        init_daily_seed_task(&config).await.unwrap();
        assert!(cache_dir.exists());

        let seed_a = get_current_seed().await.unwrap();
        let seed_b = get_current_seed().await.unwrap();
        assert_eq!(seed_a, seed_b, "seed should be stable within the same day");

        let file_path = cache_dir.join(DailySeedCache::get_date_string());
        assert!(file_path.exists());
        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents.parse::<i64>().unwrap(), seed_a);
    }
}*/