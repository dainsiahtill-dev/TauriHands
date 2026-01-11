use serde::Serialize;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct AuditLog {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

#[derive(Serialize)]
pub struct AuditEntry {
    pub timestamp_ms: u128,
    pub action: String,
    pub session_id: Option<String>,
    pub command: Option<String>,
    pub payload: serde_json::Value,
}

impl AuditLog {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn write(&self, entry: AuditEntry) {
        let _guard = self.lock.lock().expect("audit log lock poisoned");
        if let Some(parent) = self.path.parent() {
            let _ = create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            if let Ok(line) = serde_json::to_string(&entry) {
                let _ = writeln!(file, "{}", line);
            }
        }
    }
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
