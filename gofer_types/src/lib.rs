mod api;
mod api_service;
mod storage;

use std::time::{SystemTime, UNIX_EPOCH};

/// Return the current epoch time in milliseconds.
pub fn epoch_milli() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
