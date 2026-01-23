use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    pub timestamp: u128,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_total: u64,
    pub disk_usage: u64,
    pub network_io: NetworkIO,
    pub application_metrics: ApplicationMetrics,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkIO {
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationMetrics {
    pub active_connections: u32,
    pub request_count: u64,
    pub error_count: u64,
    pub response_time_avg: f64,
    pub response_time_p95: f64,
    pub llm_calls: u64,
    pub tool_calls: u64,
    pub terminal_sessions: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSnapshot {
    pub id: String,
    pub timestamp: u128,
    pub duration_ms: u128,
    pub operation_type: String,
    pub success: bool,
    pub details: HashMap<String, serde_json::Value>,
}

pub struct PerformanceMonitor {
    metrics: Arc<RwLock<PerformanceMetrics>>,
    snapshots: Arc<Mutex<Vec<PerformanceSnapshot>>>,
    response_times: Arc<Mutex<Vec<f64>>>,
    start_time: Instant,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics {
                timestamp: current_timestamp(),
                cpu_usage: 0.0,
                memory_usage: 0,
                memory_total: 0,
                disk_usage: 0,
                network_io: NetworkIO {
                    bytes_sent: 0,
                    bytes_received: 0,
                },
                application_metrics: ApplicationMetrics {
                    active_connections: 0,
                    request_count: 0,
                    error_count: 0,
                    response_time_avg: 0.0,
                    response_time_p95: 0.0,
                    llm_calls: 0,
                    tool_calls: 0,
                    terminal_sessions: 0,
                },
            })),
            snapshots: Arc::new(Mutex::new(Vec::new())),
            response_times: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    pub async fn record_operation_start(&self, operation_type: &str) -> String {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let snapshot = PerformanceSnapshot {
            id: snapshot_id.clone(),
            timestamp: current_timestamp(),
            duration_ms: 0,
            operation_type: operation_type.to_string(),
            success: false,
            details: HashMap::new(),
        };

        {
            let mut snapshots = self.snapshots.lock().unwrap();
            snapshots.push(snapshot);
        }

        snapshot_id
    }

    pub async fn record_operation_end(
        &self,
        snapshot_id: &str,
        success: bool,
        details: HashMap<String, serde_json::Value>,
    ) {
        let start_time = {
            let snapshots = self.snapshots.lock().unwrap();
            snapshots
                .iter()
                .find(|s| s.id == snapshot_id)
                .map(|s| s.timestamp)
                .unwrap_or_else(|| current_timestamp())
        };

        let duration = current_timestamp() - start_time;

        {
            let mut snapshots = self.snapshots.lock().unwrap();
            if let Some(snapshot) = snapshots.iter_mut().find(|s| s.id == snapshot_id) {
                snapshot.duration_ms = duration;
                snapshot.success = success;
                snapshot.details = details;
            }
        }

        // Update response time statistics
        if success {
            let mut response_times = self.response_times.lock().unwrap();
            response_times.push(duration as f64);
            
            // Keep only last 1000 response times
            if response_times.len() > 1000 {
                response_times.remove(0);
            }
        }

        self.update_application_metrics(operation_type_from_snapshot_id(snapshot_id), success).await;
    }

    pub async fn increment_llm_calls(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.application_metrics.llm_calls += 1;
    }

    pub async fn increment_tool_calls(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.application_metrics.tool_calls += 1;
    }

    pub async fn increment_terminal_sessions(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.application_metrics.terminal_sessions += 1;
    }

    pub async fn decrement_terminal_sessions(&self) {
        let mut metrics = self.metrics.write().await;
        if metrics.application_metrics.terminal_sessions > 0 {
            metrics.application_metrics.terminal_sessions -= 1;
        }
    }

    pub async fn increment_active_connections(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.application_metrics.active_connections += 1;
    }

    pub async fn decrement_active_connections(&self) {
        let mut metrics = self.metrics.write().await;
        if metrics.application_metrics.active_connections > 0 {
            metrics.application_metrics.active_connections -= 1;
        }
    }

    pub async fn increment_request_count(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.application_metrics.request_count += 1;
    }

    pub async fn increment_error_count(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.application_metrics.error_count += 1;
    }

    pub async fn update_system_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.timestamp = current_timestamp();
        
        // Update system metrics (simplified versions)
        metrics.cpu_usage = self.get_cpu_usage().await;
        metrics.memory_usage = self.get_memory_usage().await;
        metrics.memory_total = self.get_memory_total().await;
        metrics.disk_usage = self.get_disk_usage().await;
        metrics.network_io = self.get_network_io().await;
        
        // Update application metrics
        self.update_response_time_stats(&mut metrics).await;
    }

    pub async fn get_current_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn get_recent_snapshots(&self, limit: usize) -> Vec<PerformanceSnapshot> {
        let snapshots = self.snapshots.lock().unwrap();
        snapshots.iter().rev().take(limit).cloned().collect()
    }

    pub async fn clear_old_snapshots(&self, older_than_ms: u128) {
        let mut snapshots = self.snapshots.lock().unwrap();
        let cutoff = current_timestamp() - older_than_ms;
        snapshots.retain(|s| s.timestamp > cutoff);
    }

    pub fn get_uptime(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }

    async fn update_application_metrics(&self, operation_type: &str, success: bool) {
        let mut metrics = self.metrics.write().await;
        
        if !success {
            metrics.application_metrics.error_count += 1;
        }
        
        // Update specific counters based on operation type
        match operation_type {
            "llm_request" => metrics.application_metrics.llm_calls += 1,
            "tool_call" => metrics.application_metrics.tool_calls += 1,
            "terminal_session" => {
                if success {
                    metrics.application_metrics.terminal_sessions += 1;
                }
            }
            _ => {}
        }
    }

    async fn update_response_time_stats(&self, metrics: &mut PerformanceMetrics) {
        let response_times = self.response_times.lock().unwrap();
        
        if response_times.is_empty() {
            metrics.application_metrics.response_time_avg = 0.0;
            metrics.application_metrics.response_time_p95 = 0.0;
            return;
        }

        let sum: f64 = response_times.iter().sum();
        let count = response_times.len() as f64;
        metrics.application_metrics.response_time_avg = sum / count;

        // Calculate p95
        let mut sorted_times = response_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_index = ((sorted_times.len() as f64 * 0.95) as usize).min(sorted_times.len() - 1);
        metrics.application_metrics.response_time_p95 = sorted_times[p95_index];
    }

    async fn get_cpu_usage(&self) -> f64 {
        // Simplified CPU usage calculation
        // In a real implementation, you would use platform-specific APIs
        0.0 // Placeholder
    }

    async fn get_memory_usage(&self) -> u64 {
        // Simplified memory usage calculation
        // In a real implementation, you would use platform-specific APIs
        0 // Placeholder
    }

    async fn get_memory_total(&self) -> u64 {
        // Simplified total memory
        // In a real implementation, you would use platform-specific APIs
        8589934592 // 8GB placeholder
    }

    async fn get_disk_usage(&self) -> u64 {
        // Simplified disk usage
        // In a real implementation, you would use platform-specific APIs
        0 // Placeholder
    }

    async fn get_network_io(&self) -> NetworkIO {
        // Simplified network I/O
        // In a real implementation, you would use platform-specific APIs
        NetworkIO {
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

fn current_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn operation_type_from_snapshot_id(snapshot_id: &str) -> &str {
    // In a real implementation, you would store operation type with the snapshot
    // For now, return a default
    "unknown"
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
