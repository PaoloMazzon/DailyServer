use spdlog::prelude::*;
use tokio::sync::mpsc;
use rusqlite::{params, Connection, Result};
use tokio::task::JoinHandle;
use tokio::time;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool, atomic::AtomicI64, atomic::Ordering};
use std::time::Duration;
use anyhow::anyhow;
use crate::util::graceful_shutdown::{kill_program, kill_signal_received};

static TABLE_CREATION_SQL: &str = "
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    score INTEGER
);";

/// Row in the database
#[derive(Debug)]
pub struct DatabaseRow {
    pub id: i64,
    pub name: String,
    pub score: i64,
}

impl DatabaseRow {
    /// Creates an empty database row for testing purposes
    pub fn empty() -> Self {
        DatabaseRow { 
            id: 0, 
            name: String::new(),
            score: 0,
        }
    }

    /// Creates a new database row with an assigned ID
    pub fn new(accessor: &mut DatabaseAccessor) -> Self {
        DatabaseRow { 
            id: accessor.get_id(),
            name: String::new(),
            score: 0,
        }
    }
}

/// A read-request is a query into the db and a sender so the db thread can reply
pub struct DatabaseReadRequest {
    pub query: String,
    pub return_to: mpsc::Sender<Vec<DatabaseRow>>,
}

/// A way to read and write to a database in a thread-safe manner
pub struct DatabaseAccessor {
    /// Used to send write requests to the database
    write_request: mpsc::Sender<DatabaseRow>,

    /// Used to send read requests to the database thread
    read_requester: mpsc::Sender<DatabaseReadRequest>,

    /// Used by this struct to receive requests back from the database thread
    read_receiver: mpsc::Receiver<Vec<DatabaseRow>>,
    
    /// Provided to the database thread to send the result back
    read_sender: mpsc::Sender<Vec<DatabaseRow>>,

    /// Global index counter copy copy from the database
    index_counter: Arc<AtomicI64>,
}

/// Thread-safe database that can only be written to via a ringbuffer
pub struct Database {
    /// Provided to accessors to send writes to the DB
    write_sender: mpsc::Sender<DatabaseRow>,

    /// Copied for database accessors
    read_request_sender: mpsc::Sender<DatabaseReadRequest>,

    /// Signals the database thread to die
    kill_thread: Arc<AtomicBool>,

    /// Join handle for the database thread (used in the drop function)
    writer_thread: JoinHandle<()>,

    /// Global index counter
    index_counter: Arc<AtomicI64>,
}

impl Database {
    fn handle_row_write(database: &mut Connection, row: DatabaseRow) {
        if let Err(e) = database.execute("INSERT INTO users (id, name, score) VALUES (?1, ?2, ?3)",
                                         params![row.id, row.name, row.score]) {
            error!("Failed to write row {:?} to database, {}", row, e);
        }
    }

    fn handle_row_read(database: &mut Connection, read: DatabaseReadRequest) {
        let mut query = match database.prepare(read.query.as_str()) {
            Ok(q) => q,
            Err(e) => {
                error!("Invalid database read requested with query {}", read.query);
                return
            }
        };
        let result_list: Vec<DatabaseRow> = match query.query_map([], |row| {
            Ok(DatabaseRow {
                id: row.get(0)?,
                name: row.get(1)?,
                score: row.get(2)?,
            })
        }) {
            Ok(iter) => iter.map(|x| {x.unwrap_or(DatabaseRow::empty())}).collect(),
            Err(e) => {
                error!("Failed to get query results from query {}", read.query);
                return
            }
        };
        if let Err(e) = read.return_to.try_send(result_list) {
            error!("Failed to send read request back to sender for query '{}'", read.query);
        }
    }

    fn handle_row_writes(connection: &mut Connection, write_receiver: &mut mpsc::Receiver<DatabaseRow>) {
        while let Ok(row) = write_receiver.try_recv() {
            Database::handle_row_write(connection, row);
        }
    }

    fn handle_row_reads(connection: &mut Connection, read_request_receiver: &mut mpsc::Receiver<DatabaseReadRequest>) {
        while let Ok(read) = read_request_receiver.try_recv() {
            Database::handle_row_read(connection, read);
        }
    }

    /// Opens a new database and spawns a task to watch the writes to it
    pub async fn open(filename: &str) -> Result<Self, rusqlite::Error> {
        let (write_sender, mut write_receiver) = mpsc::channel(10);
        let (read_request_sender, mut read_request_receiver) = mpsc::channel(10);        
        let mut connection = Connection::open(Path::new(filename))?;
        let kill_thread = Arc::new(AtomicBool::new(false));
        let spawned_thread_kill_switch = kill_thread.clone();

        let join_handle = tokio::spawn(async move {
            if let Err(e) = connection.execute(TABLE_CREATION_SQL, []) {
                error!("Failed to create dailies table, {}", e);
                kill_program();
                panic!();
            }

            loop {
                Self::handle_row_writes(&mut connection, &mut write_receiver);
                Self::handle_row_reads(&mut connection, &mut read_request_receiver);

                if spawned_thread_kill_switch.load(Ordering::Relaxed) || kill_signal_received() {
                    Self::handle_row_writes(&mut connection, &mut write_receiver);
                    Self::handle_row_reads(&mut connection, &mut read_request_receiver);
                    info!("Database writer thread received kill signal.");
                    break;
                }
            }
        });

        Ok(Database {
            write_sender,
            kill_thread,
            read_request_sender,
            writer_thread: join_handle,
            index_counter: Arc::new(AtomicI64::new(0)),
        })
    }

    pub async fn get_accessor(&self) -> DatabaseAccessor {
        let (read_sender, read_receiver) = mpsc::channel(10);
        DatabaseAccessor {
            write_request: self.write_sender.clone(),
            read_requester: self.read_request_sender.clone(),
            read_receiver,
            read_sender,
            index_counter: self.index_counter.clone(),
        }
    }
}

/// Drop closes the database writer thread
impl Drop for Database {
    fn drop(&mut self) {
        self.kill_thread.store(true, Ordering::Relaxed);
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                match time::timeout(Duration::from_secs(1), &mut self.writer_thread).await {
                    Ok(res) => match res {
                        Ok(()) => info!("Closed database writer gracefully."),
                        Err(e) => error!("Failed to close the database writer thread, {}", e)
                    },
                    Err(_) => error!("Database writer thread timed out.")
                }
            });
        });
    }
}

impl DatabaseAccessor {
    /// Tries to write a row, can fail
    pub async fn write(&self, row: DatabaseRow) -> Result<(), mpsc::error::SendError<DatabaseRow>> {
        self.write_request.send(row).await
    }

    /// Tries to read from a query, can fail
    pub async fn read(&mut self, query: String, timeout: Duration) -> Result<Vec<DatabaseRow>, anyhow::Error> {
        if let Err(e) = self.read_requester.send(DatabaseReadRequest { query: query.clone(), return_to: self.read_sender.clone() }).await {
            error!("Failed to send read request (query = '{}') to database ({}).", query, e);
            return Err(anyhow!("{}", e));
        }

        match time::timeout(timeout, self.read_receiver.recv()).await {
            Ok(res) => match res {
                Some(x) => Ok(x),
                None => {
                    spdlog::error!("Channel closed mid read request on query '{}'", query);
                    Err(anyhow!("Channel closed"))
                }
            },
            Err(_) => Err(anyhow!("Timed out"))
        }
    }

    /// Gets a new ID for use in a database row
    pub fn get_id(&mut self) -> i64 {
        self.index_counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_database() -> Database {
        Database::open(":memory:").await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_database() {
        let db = get_database().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_write_accessor() {
        let db = get_database().await;
        let mut accessor = db.get_accessor().await;
        let row1 = DatabaseRow::new(&mut accessor);
        let row2 = DatabaseRow::new(&mut accessor);

        accessor.write(row1).await.unwrap();
        let rows = accessor.read(String::from("SELECT id, name, score FROM users;"), Duration::from_secs(1)).await.unwrap();
        assert_eq!(rows.len(), 1);
        accessor.write(row2).await.unwrap();
    }
}