use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use anyhow::{Context, Result};
use async_trait::async_trait;

use super::engine::{AutomationConfig, AutomationTask, AutomationResult, TaskStatus};

#[async_trait]
pub trait ProgressMonitor: Send + Sync {
    fn update_progress(&self, task_id: uuid::Uuid, progress: f64);
    fn get_progress(&self) -> Result<f64>;
    fn get_task_progress(&self, task_id: uuid::Uuid) -> Option<f64>;
    fn get_progress_report(&self) -> ProgressReport;
    fn start_monitoring(&self, task: &AutomationTask);
    fn complete_task(&self, task_id: uuid::Uuid, result: &AutomationResult);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    pub overall_progress: f64,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub active_tasks: usize,
    pub task_details: Vec<TaskProgress>,
    pub estimated_completion: Option<String>,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: uuid::Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub progress: f64,
    pub started_at: String,
    pub estimated_completion: Option<String>,
    pub current_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub average_task_duration: Duration,
    pub tasks_per_hour: f64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub total_execution_time: Duration,
}

pub struct RealTimeMonitor {
    config: AutomationConfig,
    task_progress: Arc<Mutex<HashMap<uuid::Uuid, TaskProgress>>>,
    completed_tasks: Arc<Mutex<Vec<AutomationResult>>>,
    start_times: Arc<Mutex<HashMap<uuid::Uuid, Instant>>>,
    overall_progress: Arc<Mutex<f64>>,
}

impl RealTimeMonitor {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self {
            config,
            task_progress: Arc::new(Mutex::new(HashMap::new())),
            completed_tasks: Arc::new(Mutex::new(Vec::new())),
            start_times: Arc::new(Mutex::new(HashMap::new())),
            overall_progress: Arc::new(Mutex::new(0.0)),
        })
    }

    fn calculate_overall_progress(&self) -> f64 {
        let progress_map = self.task_progress.lock().unwrap();
        let completed = self.completed_tasks.lock().unwrap();
        
        if progress_map.is_empty() && completed.is_empty() {
            return 0.0;
        }

        let total_tasks = progress_map.len() + completed.len();
        if total_tasks == 0 {
            return 0.0;
        }

        let active_progress: f64 = progress_map.values().map(|p| p.progress).sum();
        let completed_progress = completed.len() as f64 * 100.0;
        
        (active_progress + completed_progress) / total_tasks as f64
    }

    fn estimate_completion_time(&self) -> Option<String> {
        let progress_map = self.task_progress.lock().unwrap();
        let start_times = self.start_times.lock().unwrap();
        
        if progress_map.is_empty() {
            return None;
        }

        let mut total_remaining_time = Duration::from_secs(0);
        
        for (task_id, progress) in progress_map.iter() {
            if let Some(start_time) = start_times.get(task_id) {
                let elapsed = start_time.elapsed();
                if progress.progress > 0.0 {
                    let estimated_total = Duration::from_millis(
                        (elapsed.as_millis() as f64 / (progress.progress / 100.0)) as u64
                    );
                    let remaining = estimated_total.saturating_sub(elapsed);
                    total_remaining_time += remaining;
                }
            }
        }

        if total_remaining_time.as_secs() > 0 {
            Some(format!("Estimated completion in {} seconds", total_remaining_time.as_secs()))
        } else {
            None
        }
    }

    fn calculate_performance_metrics(&self) -> PerformanceMetrics {
        let completed = self.completed_tasks.lock().unwrap();
        
        if completed.is_empty() {
            return PerformanceMetrics {
                average_task_duration: Duration::from_secs(0),
                tasks_per_hour: 0.0,
                success_rate: 0.0,
                error_rate: 0.0,
                total_execution_time: Duration::from_secs(0),
            };
        }

        let total_duration: Duration = completed.iter().map(|r| r.execution_time).sum();
        let average_duration = total_duration / completed.len() as u32;
        
        let successful_tasks = completed.iter().filter(|r| r.success).count();
        let success_rate = successful_tasks as f64 / completed.len() as f64;
        let error_rate = 1.0 - success_rate;
        
        let tasks_per_hour = if average_duration.as_secs() > 0 {
            3600.0 / average_duration.as_secs() as f64
        } else {
            0.0
        };

        PerformanceMetrics {
            average_task_duration: average_duration,
            tasks_per_hour,
            success_rate,
            error_rate,
            total_execution_time: total_duration,
        }
    }
}

#[async_trait]
impl ProgressMonitor for RealTimeMonitor {
    fn update_progress(&self, task_id: uuid::Uuid, progress: f64) {
        let mut progress_map = self.task_progress.lock().unwrap();
        if let Some(task_progress) = progress_map.get_mut(&task_id) {
            task_progress.progress = progress.min(100.0).max(0.0);
        }
        
        // Update overall progress
        *self.overall_progress.lock().unwrap() = self.calculate_overall_progress();
    }

    fn get_progress(&self) -> Result<f64> {
        Ok(*self.overall_progress.lock().unwrap())
    }

    fn get_task_progress(&self, task_id: uuid::Uuid) -> Option<f64> {
        self.task_progress.lock().unwrap().get(&task_id).map(|p| p.progress)
    }

    fn get_progress_report(&self) -> ProgressReport {
        let progress_map = self.task_progress.lock().unwrap();
        let completed = self.completed_tasks.lock().unwrap();
        
        let total_tasks = progress_map.len() + completed.len();
        let completed_tasks = completed.len();
        let failed_tasks = completed.iter().filter(|r| !r.success).count();
        let active_tasks = progress_map.len();
        
        let task_details: Vec<TaskProgress> = progress_map.values().cloned().collect();
        let estimated_completion = self.estimate_completion_time();
        let performance_metrics = self.calculate_performance_metrics();

        ProgressReport {
            overall_progress: self.calculate_overall_progress(),
            total_tasks,
            completed_tasks,
            failed_tasks,
            active_tasks,
            task_details,
            estimated_completion,
            performance_metrics,
        }
    }

    fn start_monitoring(&self, task: &AutomationTask) {
        let mut progress_map = self.task_progress.lock().unwrap();
        let mut start_times = self.start_times.lock().unwrap();
        
        let task_progress = TaskProgress {
            task_id: task.id,
            title: task.title.clone(),
            status: task.status.clone(),
            progress: 0.0,
            started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            estimated_completion: None,
            current_step: "Starting".to_string(),
        };
        
        progress_map.insert(task.id, task_progress);
        start_times.insert(task.id, Instant::now());
        
        log::info!("Started monitoring task: {}", task.title);
    }

    fn complete_task(&self, task_id: uuid::Uuid, result: &AutomationResult) {
        let mut progress_map = self.task_progress.lock().unwrap();
        let mut completed = self.completed_tasks.lock().unwrap();
        let mut start_times = self.start_times.lock().unwrap();
        
        // Remove from active tasks
        progress_map.remove(&task_id);
        start_times.remove(&task_id);
        
        // Add to completed tasks
        completed.push(result.clone());
        
        // Update overall progress
        *self.overall_progress.lock().unwrap() = self.calculate_overall_progress();
        
        log::info!("Completed task: {:?}, success: {}", task_id, result.success);
    }
}

pub struct FileMonitor {
    config: AutomationConfig,
    log_file: std::path::PathBuf,
}

impl FileMonitor {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        let log_file = config.workspace.join("taurihands_progress.log");
        Ok(Self { config, log_file })
    }

    fn write_progress_to_file(&self, report: &ProgressReport) -> Result<()> {
        let log_entry = serde_json::to_string_pretty(report)?;
        std::fs::write(&self.log_file, log_entry)?;
        Ok(())
    }
}

#[async_trait]
impl ProgressMonitor for FileMonitor {
    fn update_progress(&self, _task_id: uuid::Uuid, _progress: f64) {
        let report = self.get_progress_report();
        if let Err(e) = self.write_progress_to_file(&report) {
            log::error!("Failed to write progress to file: {}", e);
        }
    }

    fn get_progress(&self) -> Result<f64> {
        if self.log_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.log_file) {
                if let Ok(report) = serde_json::from_str::<ProgressReport>(&content) {
                    return Ok(report.overall_progress);
                }
            }
        }
        Ok(0.0)
    }

    fn get_task_progress(&self, _task_id: uuid::Uuid) -> Option<f64> {
        // File monitor doesn't track individual task progress
        None
    }

    fn get_progress_report(&self) -> ProgressReport {
        if self.log_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.log_file) {
                if let Ok(report) = serde_json::from_str::<ProgressReport>(&content) {
                    return report;
                }
            }
        }
        
        // Return empty report if file doesn't exist or is invalid
        ProgressReport {
            overall_progress: 0.0,
            total_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            active_tasks: 0,
            task_details: Vec::new(),
            estimated_completion: None,
            performance_metrics: PerformanceMetrics {
                average_task_duration: Duration::from_secs(0),
                tasks_per_hour: 0.0,
                success_rate: 0.0,
                error_rate: 0.0,
                total_execution_time: Duration::from_secs(0),
            },
        }
    }

    fn start_monitoring(&self, _task: &AutomationTask) {
        let report = self.get_progress_report();
        if let Err(e) = self.write_progress_to_file(&report) {
            log::error!("Failed to write task start to file: {}", e);
        }
    }

    fn complete_task(&self, _task_id: uuid::Uuid, _result: &AutomationResult) {
        let report = self.get_progress_report();
        if let Err(e) = self.write_progress_to_file(&report) {
            log::error!("Failed to write task completion to file: {}", e);
        }
    }
}
