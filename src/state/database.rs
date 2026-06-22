use serde_json::error::Category::Data;
use spdlog::prelude::*;
use tokio::sync::mpsc;
use rusqlite::{params, Connection, Result};
use tokio::task::JoinHandle;
use tokio::time;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::time::Duration;
use anyhow::anyhow;

/// Row in the database
pub struct DatabaseRow {
    pub name: String,
    pub score: i32,
}

impl DatabaseRow {
    /// Creates an empty row for testing purposes
    pub fn empty() -> Self {
        DatabaseRow { 
            name: String::new(), 
            score: 0
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
}

/// Thread-safe database that can only be written to via a ringbuffer
pub struct Database {
    /// Provided to accessors to send writes to the DB
    write_sender: mpsc::Sender<DatabaseRow>,

    /// 
    read_request_sender: mpsc::Sender<DatabaseReadRequest>,
    kill_thread: Arc<AtomicBool>,
    writer_thread: JoinHandle<()>,
}

impl Database {
    fn handle_row_write(database: &mut Connection, row: DatabaseRow) {
        // TODO: This
    }

    fn handle_row_read(database: &mut Connection, read: DatabaseReadRequest) {
        // TODO: This
        if let Err(e) = read.return_to.try_send(Vec::new()) {
            error!("Failed to send read request back to sender for query '{}'", read.query);
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
            loop {
                // TODO: Dump everything to file before kill thread signal received
                if let Ok(row) = write_receiver.try_recv() {
                    Database::handle_row_write(&mut connection, row);
                }

                // TODO: Handle all read requests before quitting
                if let Ok(read) = read_request_receiver.try_recv() {
                    Database::handle_row_read(&mut connection, read);
                }

                if spawned_thread_kill_switch.load(Ordering::Relaxed) {
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
        })
    }

    pub async fn get_accessor(&self) -> DatabaseAccessor {
        let (read_sender, read_receiver) = mpsc::channel(10);
        DatabaseAccessor {
            write_request: self.write_sender.clone(),
            read_requester: self.read_request_sender.clone(),
            read_receiver,
            read_sender
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
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_database() -> Database {
        Database::open("filename").await.unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_open_database() {
        let db = get_database().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_read_write_accessor() {
        let db = get_database().await;
        let mut accessor = db.get_accessor().await;
        accessor.read(String::new(), Duration::from_secs(1)).await.unwrap();
        accessor.write(DatabaseRow::empty()).await.unwrap();
    }
}