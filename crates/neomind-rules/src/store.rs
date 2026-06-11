//! Rule persistence using redb database.
//!
//! Provides persistent storage for rule definitions and execution history.

use crate::engine::{CompiledRule, RuleId};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;

// Table definitions
const RULES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("rules");

/// Error type for rule storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Implement From for all redb error types
impl From<redb::Error> for StoreError {
    fn from(e: redb::Error) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<redb::StorageError> for StoreError {
    fn from(e: redb::StorageError) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<redb::DatabaseError> for StoreError {
    fn from(e: redb::DatabaseError) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<redb::TableError> for StoreError {
    fn from(e: redb::TableError) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<redb::TransactionError> for StoreError {
    fn from(e: redb::TransactionError) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<redb::CommitError> for StoreError {
    fn from(e: redb::CommitError) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<redb::CompactionError> for StoreError {
    fn from(e: redb::CompactionError) -> Self {
        StoreError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Serialization(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Persistent storage for rules.
pub struct RuleStore {
    db: Arc<Database>,
    /// Storage path for singleton tracking.
    path: String,
    /// Temp file path for cleanup (if using memory mode).
    temp_path: Option<PathBuf>,
}

/// Global rule store singleton to prevent multiple opens.
static RULE_STORE_SINGLETON: Mutex<Option<Arc<RuleStore>>> = Mutex::new(None);

impl RuleStore {
    /// Open or create a rule store at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Arc<Self>> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Check singleton
        {
            let singleton = RULE_STORE_SINGLETON.lock();
            if let Some(store) = singleton.as_ref() {
                if store.path == path_str {
                    return Ok(store.clone());
                }
            }
        }

        // Open database
        let (db, temp_path) = Self::open_db(&path_str)?;

        let store = Arc::new(RuleStore {
            db: Arc::new(db),
            path: path_str,
            temp_path,
        });

        *RULE_STORE_SINGLETON.lock() = Some(store.clone());
        Ok(store)
    }

    /// Create an in-memory store.
    pub fn memory() -> Result<Arc<Self>> {
        let temp_path =
            std::env::temp_dir().join(format!("rules_store_{}.redb", uuid::Uuid::new_v4()));
        Self::open(temp_path)
    }

    fn open_db(path_str: &str) -> Result<(Database, Option<PathBuf>)> {
        let (db, temp_path) = if path_str == ":memory:" {
            // Use temp file for in-memory mode
            let temp_path =
                std::env::temp_dir().join(format!("rules_store_{}.redb", uuid::Uuid::new_v4()));
            let db = Database::create(&temp_path)?;
            (db, Some(temp_path))
        } else {
            let path_ref = Path::new(path_str);
            if let Some(parent) = path_ref.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let db = if path_ref.exists() {
                Database::open(path_ref)?
            } else {
                Database::create(path_ref)?
            };
            (db, None)
        };

        Ok((db, temp_path))
    }

    /// Save a rule.
    pub fn save(&self, rule: &CompiledRule) -> Result<()> {
        let key = format!("rule:{}", rule.id);
        let value = serde_json::to_vec(rule)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(RULES_TABLE)?;
            table.insert(key.as_str(), value.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Load a rule by ID.
    pub fn load(&self, id: &RuleId) -> Result<Option<CompiledRule>> {
        let key = format!("rule:{}", id);

        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(RULES_TABLE)?;

        match table.get(key.as_str())? {
            Some(value) => {
                let rule = serde_json::from_slice(value.value())?;
                Ok(Some(rule))
            }
            None => Ok(None),
        }
    }

    /// Delete a rule by ID.
    pub fn delete(&self, id: &RuleId) -> Result<bool> {
        let key = format!("rule:{}", id);

        let write_txn = self.db.begin_write()?;
        let existed = {
            let mut table = write_txn.open_table(RULES_TABLE)?;
            let result = table.remove(key.as_str())?.is_some();
            result
        };
        write_txn.commit()?;
        Ok(existed)
    }

    /// List all rules.
    pub fn list_all(&self) -> Result<Vec<CompiledRule>> {
        let mut rules = Vec::new();

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(RULES_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(rules), // Table doesn't exist yet
        };

        let iter = table.iter()?;
        for result in iter {
            let (_, value) = result?;
            let rule: CompiledRule = serde_json::from_slice(value.value())?;
            rules.push(rule);
        }

        Ok(rules)
    }
}

impl Drop for RuleStore {
    fn drop(&mut self) {
        // Clean up temp file if using memory mode
        if let Some(ref temp_path) = self.temp_path {
            let _ = std::fs::remove_file(temp_path);
        }
    }
}
