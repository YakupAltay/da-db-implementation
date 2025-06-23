use serde_json;
use std::collections::HashMap;

use crate::avail;
use crate::schema::{DatabaseError, DatabaseMetadata, Record};

pub struct DatabaseClient {
    app_id: u32,
    metadata: Option<DatabaseMetadata>,
    block_range: Option<u32>,
}

// Helper function to get current timestamp for logging
fn log_with_timestamp(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    println!("[{}] {}", timestamp, message);
}

impl DatabaseClient {
    pub async fn new(
        app_id: u32,
        block_range: Option<u32>,
    ) -> Result<Self, DatabaseError> {
        let mut db_client = Self {
            app_id,
            metadata: None,
            block_range,
        };

        if let Some(metadata) = db_client.discover_database().await? {
            log_with_timestamp(&format!("Found existing database starting at block: {:?}", metadata.start_height));
            db_client.metadata = Some(metadata);
        } else {
            let latest_block_height = avail::get_latest_block_height_on_avail()
                .await
                .map_err(|e| DatabaseError::AvailError(e.to_string()))?;

            let metadata = DatabaseMetadata {
                start_height: latest_block_height as u64,
                record_count: 0,
                last_updated: chrono::Utc::now()
            };

            db_client.save_metadata(&metadata).await?;
            db_client.metadata = Some(metadata);

            log_with_timestamp(&format!("Created new database starting at block: {:?}", latest_block_height));
        }

        Ok(db_client)
    }

    async fn discover_database(&self) -> Result<Option<DatabaseMetadata>, DatabaseError> {
        let latest_block_height = avail::get_latest_block_height_on_avail()
            .await
            .map_err(|e| DatabaseError::AvailError(e.to_string()))?;

        let start_height = if let Some(block_range) = self.block_range {
            latest_block_height.saturating_sub(block_range)
        } else {
            latest_block_height.saturating_sub(10)
        };

        log_with_timestamp(&format!(
            "Searching for existing database (blocks {}..{})",
            start_height,
            latest_block_height
        ));

        let data = avail::get_data_from_avail_by_app_id(
                self.app_id,
                latest_block_height - start_height
            ).await
            .map_err(|e| DatabaseError::AvailError(e.to_string()))?;

        for line in data.iter().rev() {
            if let Ok(metadata) = serde_json::from_str::<DatabaseMetadata>(line) {
                log_with_timestamp(&format!("Found existing database at height {}", metadata.start_height));
                return Ok(Some(metadata));
            }
        }

        log_with_timestamp(&format!("No existing database found, creating new one at height {}", latest_block_height));
        Ok(None)
    }

    async fn save_metadata(&self, metadata: &DatabaseMetadata) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(metadata)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        avail::submit_data_to_avail_by_app_id(self.app_id, json)
            .await
            .map_err(|e| DatabaseError::AvailError(e.to_string()))?;

        Ok(())
    }

    pub async fn add_record(&mut self, record: Record) -> Result<(), DatabaseError> {
        let json = serde_json::to_string(&record)
            .map_err(|e| DatabaseError::SerializationError(e.to_string()))?;

        avail::submit_data_to_avail_by_app_id(self.app_id, json)
            .await
            .map_err(|e| DatabaseError::AvailError(e.to_string()))?;

        if let Some(mut metadata) = self.metadata.clone() {
            metadata.record_count += 1;
            metadata.last_updated = chrono::Utc::now();
            self.save_metadata(&metadata).await?;
            self.metadata = Some(metadata);
        }

        Ok(())
    }

    pub async fn get_record(&self, key: &str) -> Result<Option<Record>, DatabaseError> {
        let latest_block_height = avail::get_latest_block_height_on_avail()
            .await
            .map_err(|e| DatabaseError::AvailError(e.to_string()))?;
        let db_start = self.metadata.as_ref().map(|m| m.start_height).unwrap_or(0);
        log_with_timestamp(&format!(
            "Searching for record with key '{}' (database start: {}, current height: {})",
            key, db_start, latest_block_height
        ));
        let block_range_to_search = if latest_block_height as u64 >= db_start {
            (latest_block_height as u64 - db_start) as u32
        } else {
            0
        };
        let blobs = avail::get_data_from_avail_by_app_id(
            self.app_id,
            block_range_to_search
        ).await
        .map_err(|e| DatabaseError::AvailError(e.to_string()))?;

        for blob in blobs.iter().rev() {
            if serde_json::from_str::<crate::schema::DatabaseMetadata>(blob).is_ok() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<Record>(blob) {
                if record.key == key {
                    log_with_timestamp(&format!("Found record with key '{}' at height {}", key, latest_block_height));
                    return Ok(Some(record));
                }
            }
        }

        Ok(None)
    }

    pub async fn list_records(&self) -> Result<Vec<Record>, DatabaseError> {
        let latest_block_height = avail::get_latest_block_height_on_avail()
            .await
            .map_err(|e| DatabaseError::AvailError(e.to_string()))?;
        let db_start = self.metadata.as_ref().map(|m| m.start_height).unwrap_or(0);
        log_with_timestamp(&format!(
            "Listing all records (database start: {}, current height: {})",
            db_start, latest_block_height
        ));

        let block_range_to_search = if latest_block_height as u64 >= db_start {
            (latest_block_height as u64 - db_start) as u32
        } else {
            0
        };
        let blobs = avail::get_data_from_avail_by_app_id(
            self.app_id,
            block_range_to_search
        ).await
        .map_err(|e| DatabaseError::AvailError(e.to_string()))?;
        
        let mut map: HashMap<String, Record> = HashMap::new();

        for blob in blobs.iter().rev() {
            if serde_json::from_str::<crate::schema::DatabaseMetadata>(blob).is_ok() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<Record>(blob) {
                map.entry(record.key.clone()).or_insert(record);
            }
        }
        log_with_timestamp(&format!("Found {} records", map.len()));

        Ok(map.into_values().collect())
    }
}
