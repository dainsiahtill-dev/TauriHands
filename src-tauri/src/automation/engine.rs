use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use anyhow::{Context, Result};
use async_trait::async_trait;

use super::planner::TaskPlanner;
use super::executor::TaskExecutor;
use super::validator::TaskValidator;
use super::recovery::ErrorRecovery;
use super::monitor::ProgressMonitor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationTask {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub dependencies: Vec<Uuid>,
    pub subtasks: Vec<AutomationTask>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    CodeGeneration,
    CodeModification,
    Testing,
    Documentation,
    Refactoring,
    Debugging,
    Deployment,
    Analysis,
    Configuration,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Planning,
    Executing,
    Validating,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationConfig {
    pub workspace: PathBuf,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub parallel_execution: bool,
    pub auto_recovery: bool,
    pub validation_enabled: bool,
    pub progress_reporting: bool,
    pub llm_model: String,
    pub api_key: Option<String>,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_default(),
            max_retries: 3,
            timeout_seconds: 300,
            parallel_execution: true,
            auto_recovery: true,
            validation_enabled: true,
            progress_reporting: true,
            llm_model: "gpt-4".to_string(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationResult {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time: std::time::Duration,
    pub artifacts: Vec<PathBuf>,
    pub metrics: HashMap<String, f64>,
}

#[async_trait]
pub trait AutomationEngine: Send + Sync {
    async fn execute_task(&self, task: AutomationTask) -> Result<AutomationResult>;
    async fn plan_task(&self, description: &str) -> Result<Vec<AutomationTask>>;
    async fn validate_result(&self, result: &AutomationResult) -> Result<bool>;
    async fn recover_from_error(&self, error: &str, task: &AutomationTask) -> Result<Option<AutomationTask>>;
    fn get_progress(&self) -> Result<f64>;
}

pub struct TauriHandsEngine {
    config: AutomationConfig,
    planner: Arc<dyn TaskPlanner>,
    executor: Arc<dyn TaskExecutor>,
    validator: Arc<dyn TaskValidator>,
    recovery: Arc<dyn ErrorRecovery>,
    monitor: Arc<dyn ProgressMonitor>,
    task_history: Arc<Mutex<Vec<AutomationResult>>>,
    active_tasks: Arc<Mutex<HashMap<Uuid, AutomationTask>>>,
}

impl TauriHandsEngine {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        let planner = Arc::new(super::planner::LLMTaskPlanner::new(config.clone())?);
        let executor = Arc::new(super::executor::CodeExecutor::new(config.clone())?);
        let validator = Arc::new(super::validator::DefaultValidator::new(config.clone())?);
        let recovery = Arc::new(super::recovery::SmartRecovery::new(config.clone())?);
        let monitor = Arc::new(super::monitor::RealTimeMonitor::new(config.clone())?);

        Ok(Self {
            config,
            planner,
            executor,
            validator,
            recovery,
            monitor,
            task_history: Arc::new(Mutex::new(Vec::new())),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn execute_automation(&self, description: &str) -> Result<Vec<AutomationResult>> {
        log::info!("Starting automation: {}", description);
        
        // Step 1: Plan the task
        let tasks = self.plan_task(description).await?;
        log::info!("Planned {} subtasks", tasks.len());

        // Step 2: Execute tasks with dependencies
        let mut results = Vec::new();
        let mut executed_tasks = std::collections::HashSet::new();

        for task in &tasks {
            if let Some(result) = self.execute_task_with_dependencies(task, &mut executed_tasks).await? {
                results.push(result);
            }
        }

        // Step 3: Store results
        self.task_history.lock().unwrap().extend(results.clone());

        log::info!("Automation completed with {} results", results.len());
        Ok(results)
    }

    async fn execute_task_with_dependencies(
        &self,
        task: &AutomationTask,
        executed_tasks: &mut std::collections::HashSet<Uuid>,
    ) -> Result<Option<AutomationResult>> {
        // Check if already executed
        if executed_tasks.contains(&task.id) {
            return Ok(None);
        }

        // Execute dependencies first
        for dep_id in &task.dependencies {
            if let Some(dep_task) = self.active_tasks.lock().unwrap().get(dep_id) {
                if let Some(_) = self.execute_task_with_dependencies(dep_task, executed_tasks).await? {
                    // Dependency executed successfully
                }
            }
        }

        // Execute this task
        let result = self.execute_task(task.clone()).await?;
        executed_tasks.insert(task.id);

        // Validate result if enabled
        if self.config.validation_enabled {
            let is_valid = self.validate_result(&result).await?;
            if !is_valid && self.config.auto_recovery {
                log::warn!("Task validation failed, attempting recovery");
                if let Some(recovery_task) = self.recover_from_error(&result.error.unwrap_or_default(), task).await? {
                    let recovery_result = self.execute_task(recovery_task).await?;
                    return Ok(Some(recovery_result));
                }
            }
        }

        Ok(Some(result))
    }

    pub fn get_task_history(&self) -> Vec<AutomationResult> {
        self.task_history.lock().unwrap().clone()
    }

    pub fn get_active_tasks(&self) -> HashMap<Uuid, AutomationTask> {
        self.active_tasks.lock().unwrap().clone()
    }

    pub fn cancel_task(&self, task_id: Uuid) -> Result<bool> {
        let mut active_tasks = self.active_tasks.lock().unwrap();
        if let Some(task) = active_tasks.get_mut(&task_id) {
            task.status = TaskStatus::Cancelled;
            return Ok(true);
        }
        Ok(false)
    }
}

#[async_trait]
impl AutomationEngine for TauriHandsEngine {
    async fn execute_task(&self, task: AutomationTask) -> Result<AutomationResult> {
        let start_time = std::time::Instant::now();
        
        // Update task status
        {
            let mut active_tasks = self.active_tasks.lock().unwrap();
            active_tasks.insert(task.id, task.clone());
        }

        log::info!("Executing task: {}", task.title);

        // Execute the task
        let result = match self.executor.execute(&task).await {
            Ok(mut result) => {
                result.execution_time = start_time.elapsed();
                result
            }
            Err(e) => AutomationResult {
                task_id: task.id,
                status: TaskStatus::Failed,
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                execution_time: start_time.elapsed(),
                artifacts: Vec::new(),
                metrics: HashMap::new(),
            },
        };

        // Remove from active tasks
        {
            let mut active_tasks = self.active_tasks.lock().unwrap();
            active_tasks.remove(&task.id);
        }

        Ok(result)
    }

    async fn plan_task(&self, description: &str) -> Result<Vec<AutomationTask>> {
        self.planner.plan(description).await
    }

    async fn validate_result(&self, result: &AutomationResult) -> Result<bool> {
        self.validator.validate(result).await
    }

    async fn recover_from_error(&self, error: &str, task: &AutomationTask) -> Result<Option<AutomationTask>> {
        self.recovery.recover(error, task).await
    }

    fn get_progress(&self) -> Result<f64> {
        self.monitor.get_progress()
    }
}
