use crate::error::{AppError, AppResult};
use redb::{Database, TableDefinition};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("translations");

pub struct Cache {
    db: Database,
}

impl Cache {
    pub fn open(app: &AppHandle) -> AppResult<Self> {
        let path: PathBuf = app
            .path()
            .resolve("cache.redb", tauri::path::BaseDirectory::AppLocalData)
            .map_err(|e| AppError::Cache(e.to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::Cache(e.to_string()))?;
        }
        let db = Database::create(path).map_err(|e| AppError::Cache(e.to_string()))?;
        Ok(Self { db })
    }

    pub fn get(&self, key: &str) -> AppResult<Option<String>> {
        let txn = self.db.begin_read().map_err(|e| AppError::Cache(e.to_string()))?;
        let table = txn.open_table(TABLE).map_err(|e| AppError::Cache(e.to_string()))?;
        let guard = table.get(key).map_err(|e| AppError::Cache(e.to_string()))?;
        Ok(guard.map(|value| value.value().to_string()))
    }

    pub fn insert(&self, key: &str, value: &str) -> AppResult<()> {
        let txn = self.db.begin_write().map_err(|e| AppError::Cache(e.to_string()))?;
        {
            let mut table = txn.open_table(TABLE).map_err(|e| AppError::Cache(e.to_string()))?;
            table
                .insert(key, value)
                .map_err(|e| AppError::Cache(e.to_string()))?;
        }
        txn.commit().map_err(|e| AppError::Cache(e.to_string()))
    }

    pub fn clear(&self) -> AppResult<()> {
        let txn = self.db.begin_write().map_err(|e| AppError::Cache(e.to_string()))?;
        {
            let mut table = txn.open_table(TABLE).map_err(|e| AppError::Cache(e.to_string()))?;
            let keys: Vec<String> = table
                .iter()
                .map_err(|e| AppError::Cache(e.to_string()))?
                .filter_map(Result::ok)
                .map(|entry| entry.0.value().to_string())
                .collect();
            for key in keys {
                table
                    .remove(key.as_str())
                    .map_err(|e| AppError::Cache(e.to_string()))?;
            }
        }
        txn.commit().map_err(|e| AppError::Cache(e.to_string()))
    }
}

pub fn make_key(from: &str, to: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(from.as_bytes());
    hasher.update(b"->");
    hasher.update(to.as_bytes());
    hasher.update(b":");
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}
