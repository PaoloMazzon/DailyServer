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

static CREATE_SEED_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS seed_cache (
    date TEXT PRIMARY KEY,
    seed INTEGER);
";

/// Cache for daily seeds, not internally sync'd since its externally sync'd with a mutex
struct DailySeedCache {
    /// Daily seed
    current_seed: DailySeed,

    /// SQLite database for the cache
    connection: Connection,
}

/// Just a wrapper for a seed and day, to prevent race conditions
struct DailySeed {
    /// Day associated with this seed
    day: NaiveDateTime,

    /// Actual seed
    seed: i64
}

impl DailySeedCache {
    /// Standardized way to get date
    fn get_date() -> NaiveDateTime {
        Utc::now().date_naive().into()
    }

    /// Tries to get the current daily seed from the SQLite database, can fail for a few reasons
    fn try_to_get_cached_seed(&self, date: &NaiveDateTime) -> Option<DailySeed> {
        self.connection.query_row("SELECT * FROM seed_cache WHERE date = ?1",
        params![date.to_string()],
            |row| {
                let string: String = row.get(0)?;
                let datetime = DateTime::parse_from_rfc3339(string.as_str())
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(DailySeed {
                    day: datetime.naive_utc(),
                    seed: row.get(1)?,
                })
            }).ok()
    }

    /// Writes a random daily seed to the SQLite database
    fn flush_new_seed(&mut self, date: &NaiveDateTime) -> Result<i64, anyhow::Error> {
        let seed: i64 = rand::rng().random();
        self.connection.execute("INSERT INTO seed_cache (date, seed) VALUES (?1, ?2)",
                                params![date.to_string(), seed])?;
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
        if let Err(e) = connection.execute(CREATE_SEED_TABLE_SQL, params![]) {
            error!("Failed to create seed cache table, {}", e);
            instant_kill_program();
        }
        DailySeedCache {
            current_seed: DailySeed {
                day: DateTime::UNIX_EPOCH.date_naive().into(),
                seed: 0,
            },
            connection,
        }
    }

    /// Top-level get the current seed
    /// 1. Check if the current seed matches the current day, return it if it does
    /// 2. Check if we have the current seed saved (in the event of a crash or something)
    ///   a) if so, load that seed and set it for the current instance
    ///   b) if not, create a new daily seed
    pub fn get_daily_seed(&mut self) -> Result<i64, anyhow::Error> {
        let date = Self::get_date();
        if date == self.current_seed.day {
            return Ok(self.current_seed.seed)
        }

        match self.try_to_get_cached_seed(&date) {
            Some(seed) => {
                self.current_seed = seed;
                Ok(self.current_seed.seed)
            },
            None => {
                let seed = self.flush_new_seed(&date)?;
                self.current_seed.day = date;
                self.current_seed.seed = seed;
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

#[cfg(test)]
mod tests {
    use crate::util::graceful_shutdown::kill_program;
    use tokio::time::sleep;
    use super::*;

    // Because its a singleton
    #[tokio::test]
    async fn full_integration() {
        let mut server_config = ServerConfig::load(Path::new("/not-real-path"));
        server_config.daily_seed_cache_db = "/tmp/testing.db".to_string();

        // This bit should trigger the creating a new seed and returning the one stored in a variable
        {
            init_daily_seed_task(&server_config).await.unwrap();
            let _ = get_current_seed();
            let _ = get_current_seed();
            kill_program();
            sleep(Duration::from_millis(100)).await;
        }
        // And this should force it to load the variable from cache (db)
        {
            init_daily_seed_task(&server_config).await.unwrap();
            let _ = get_current_seed();
        }
    }
}
