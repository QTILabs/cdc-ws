use crate::constants::DEFAULT_DLQ_DIR;

pub fn resolve_dlq_directory() -> String {
    match std::env::var("LOCAL_DLQ_DIR") {
        Ok(path) => path,
        Err(_) => DEFAULT_DLQ_DIR.to_string(),
    }
}
