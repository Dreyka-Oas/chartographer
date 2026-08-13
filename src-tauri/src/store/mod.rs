pub mod followers;
pub mod metrics;
pub mod projects;
pub mod queries;
pub mod schema;

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        schema::migrate(&conn)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// Exécute une opération sous verrou. Le verrou n'est empoisonné que si une
    /// opération a paniqué, ce qui est un bug : on propage la panique.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let guard = self.conn.lock().expect("verrou de base empoisonné");
        f(&guard)
    }
}
