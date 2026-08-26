mod history;
mod migrations;
mod prompts;
mod search;
mod tags;

use std::{fs, path::Path, time::Duration};

use rusqlite::{Connection, OpenFlags};
use time::OffsetDateTime;

use crate::error::Result;

pub use history::{PromptVersion, PromptVersionSummary};
pub use prompts::{Prompt, PromptInput, PromptListEntry};
pub use search::SearchResult;

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open_read_only(path: &Path) -> Result<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(Duration::from_millis(250))?;
        Ok(Self { connection })
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        let mut database = Self { connection };
        database.migrate()?;
        Ok(database)
    }
}

pub(crate) fn now_timestamp() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
