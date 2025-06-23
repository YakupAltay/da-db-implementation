use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during database operations
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Avail error: {0}")]
    AvailError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String)
}

/// Represents a record in the database
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub id: String,
}

impl Record {
    pub fn new(key: String, value: String) -> Self {
        Self {
            key,
            value,
            created_at: Utc::now(),
            updated_at: None,
            id: Uuid::new_v4().to_string(),
        }
    }
}

/// Metadata for the database, stored in the first blob
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseMetadata {
    pub record_count: u64,
    pub last_updated: DateTime<Utc>,
    pub start_height: u64,
}

impl Default for DatabaseMetadata {
    fn default() -> Self {
        Self {
            record_count: 0,
            last_updated: Utc::now(),
            start_height: 1,
        }
    }
}
