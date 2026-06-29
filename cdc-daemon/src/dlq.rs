use crate::constants::DEFAULT_DLQ_DIR;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::{OpenOptions, create_dir_all};
use tokio::io::AsyncWriteExt;
use tracing::error;

pub fn resolve_dlq_directory() -> String {
    match std::env::var("LOCAL_DLQ_DIR") {
        Ok(path) => path,
        Err(_) => DEFAULT_DLQ_DIR.to_string(),
    }
}

pub async fn write_record(collection: &str, record: Value, reason: &str) {
    let dlq_dir = resolve_dlq_directory();

    if let Err(e) = create_dir_all(&dlq_dir).await {
        error!("Failed to create DLQ directory: {:?}", e);
        return;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let payload = json!({
        "collection": collection,
        "timestamp_ms": timestamp,
        "reason": reason,
        "record": record
    });

    let serialized = match serde_json::to_string(&payload) {
        Ok(s) => s + "\n",
        Err(e) => {
            error!("Failed to serialize DLQ record: {:?}", e);
            return;
        }
    };

    let dlq_path = format!("{}/dlq.log", dlq_dir);

    match OpenOptions::new().create(true).append(true).open(&dlq_path).await {
        Ok(mut file) => {
            if let Err(e) = file.write_all(serialized.as_bytes()).await {
                error!("Failed to write DLQ record: {:?}", e);
            }
        }
        Err(e) => error!("Failed to open DLQ file: {:?}", e),
    }
}