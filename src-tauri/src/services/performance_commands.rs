use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use super::performance::{PerformanceMetrics, PerformanceMonitor, PerformanceSnapshot};

pub struct PerformanceCommands {
    monitor: Arc<PerformanceMonitor>,
}

impl PerformanceCommands {
    pub fn new(monitor: Arc<PerformanceMonitor>) -> Self {
        Self { monitor }
    }

    pub async fn get_metrics(&self) -> Result<PerformanceMetrics, String> {
        self.monitor.get_current_metrics().await
    }

    pub async fn get_snapshots(&self, limit: Option<usize>) -> Result<Vec<PerformanceSnapshot>, String> {
        let limit = limit.unwrap_or(100);
        self.monitor.get_recent_snapshots(limit).await
    }

    pub async fn record_operation_start(&self, operation_type: String) -> Result<String, String> {
        Ok(self.monitor.record_operation_start(&operation_type).await)
    }

    pub async fn record_operation_end(
        &self,
        snapshot_id: String,
        success: bool,
        details: Option<Value>,
    ) -> Result<(), String> {
        let details = details.unwrap_or_else(|| Value::Object(Default::default()));
        let details_map = serde_json::from_value(details)
            .unwrap_or_else(|_| std::collections::HashMap::new());
        
        self.monitor.record_operation_end(&snapshot_id, success, details_map).await;
        Ok(())
    }

    pub async fn increment_llm_calls(&self) -> Result<(), String> {
        self.monitor.increment_llm_calls().await;
        Ok(())
    }

    pub async fn increment_tool_calls(&self) -> Result<(), String> {
        self.monitor.increment_tool_calls().await;
        Ok(())
    }

    pub async fn increment_terminal_sessions(&self) -> Result<(), String> {
        self.monitor.increment_terminal_sessions().await;
        Ok(())
    }

    pub async fn decrement_terminal_sessions(&self) -> Result<(), String> {
        self.monitor.decrement_terminal_sessions().await;
        Ok(())
    }

    pub async fn increment_active_connections(&self) -> Result<(), String> {
        self.monitor.increment_active_connections().await;
        Ok(())
    }

    pub async fn decrement_active_connections(&self) -> Result<(), String> {
        self.monitor.decrement_active_connections().await;
        Ok(())
    }

    pub async fn increment_request_count(&self) -> Result<(), String> {
        self.monitor.increment_request_count().await;
        Ok(())
    }

    pub async fn increment_error_count(&self) -> Result<(), String> {
        self.monitor.increment_error_count().await;
        Ok(())
    }

    pub async fn update_system_metrics(&self) -> Result<(), String> {
        self.monitor.update_system_metrics().await;
        Ok(())
    }

    pub async fn clear_old_snapshots(&self, older_than_ms: Option<u128>) -> Result<(), String> {
        let older_than = older_than_ms.unwrap_or(3600000); // Default 1 hour
        self.monitor.clear_old_snapshots(older_than).await;
        Ok(())
    }

    pub async fn get_uptime(&self) -> Result<u128, String> {
        Ok(self.monitor.get_uptime())
    }
}

// Register performance commands with Tauri
pub fn register_performance_commands(app: &mut tauri::App, commands: Arc<PerformanceCommands>) {
    let commands_clone = Arc::clone(&commands);
    
    app.manage(commands_clone);
    
    #[tauri::command]
    pub async fn performance_get_metrics(
        app: AppHandle,
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<PerformanceMetrics, String> {
        commands.get_metrics().await
    }

    #[tauri::command]
    pub async fn performance_get_snapshots(
        limit: Option<usize>,
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<Vec<PerformanceSnapshot>, String> {
        commands.get_snapshots(limit).await
    }

    #[tauri::command]
    pub async fn performance_record_operation_start(
        operation_type: String,
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<String, String> {
        commands.record_operation_start(operation_type).await
    }

    #[tauri::command]
    pub async fn performance_record_operation_end(
        snapshot_id: String,
        success: bool,
        details: Option<Value>,
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.record_operation_end(snapshot_id, success, details).await
    }

    #[tauri::command]
    pub async fn performance_increment_llm_calls(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.increment_llm_calls().await
    }

    #[tauri::command]
    pub async fn performance_increment_tool_calls(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.increment_tool_calls().await
    }

    #[tauri::command]
    pub async fn performance_increment_terminal_sessions(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.increment_terminal_sessions().await
    }

    #[tauri::command]
    pub async fn performance_decrement_terminal_sessions(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.decrement_terminal_sessions().await
    }

    #[tauri::command]
    pub async fn performance_increment_active_connections(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.increment_active_connections().await
    }

    #[tauri::command]
    pub async fn performance_decrement_active_connections(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.decrement_active_connections().await
    }

    #[tauri::command]
    pub async fn performance_increment_request_count(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.increment_request_count().await
    }

    #[tauri::command]
    pub async fn performance_increment_error_count(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.increment_error_count().await
    }

    #[tauri::command]
    pub async fn performance_update_system_metrics(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.update_system_metrics().await
    }

    #[tauri::command]
    pub async fn performance_clear_old_snapshots(
        older_than_ms: Option<u128>,
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<(), String> {
        commands.clear_old_snapshots(older_than_ms).await
    }

    #[tauri::command]
    pub async fn performance_get_uptime(
        commands: tauri::State<'_, Arc<PerformanceCommands>>,
    ) -> Result<u128, String> {
        commands.get_uptime().await
    }
}
