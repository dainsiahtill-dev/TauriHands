use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Context, Result};
use async_trait::async_trait;

use super::engine::{AutomationTask, AutomationConfig, TaskType};

#[async_trait]
pub trait ErrorRecovery: Send + Sync {
    async fn recover(&self, error: &str, task: &AutomationTask) -> Result<Option<AutomationTask>>;
    fn get_recovery_strategy(&self, error: &str, task: &AutomationTask) -> RecoveryStrategy;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry,
    ModifyApproach,
    BreakDownTask,
    RequestHelp,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub strategy: RecoveryStrategy,
    pub description: String,
    pub modified_task: Option<AutomationTask>,
    pub retry_count: u32,
}

pub struct SmartRecovery {
    config: AutomationConfig,
    client: reqwest::Client,
}

impl SmartRecovery {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: reqwest::Client::new(),
        })
    }

    async fn analyze_error(&self, error: &str) -> ErrorAnalysis {
        let error_lower = error.to_lowercase();
        
        if error_lower.contains("compilation") || error_lower.contains("syntax") {
            ErrorAnalysis {
                error_type: ErrorType::Compilation,
                severity: ErrorSeverity::High,
                suggested_fix: "Fix syntax errors and compilation issues".to_string(),
            }
        } else if error_lower.contains("permission") || error_lower.contains("access denied") {
            ErrorAnalysis {
                error_type: ErrorType::Permission,
                severity: ErrorSeverity::High,
                suggested_fix: "Check file permissions and access rights".to_string(),
            }
        } else if error_lower.contains("network") || error_lower.contains("connection") {
            ErrorAnalysis {
                error_type: ErrorType::Network,
                severity: ErrorSeverity::Medium,
                suggested_fix: "Check network connectivity and retry".to_string(),
            }
        } else if error_lower.contains("timeout") || error_lower.contains("time out") {
            ErrorAnalysis {
                error_type: ErrorType::Timeout,
                severity: ErrorSeverity::Medium,
                suggested_fix: "Increase timeout or optimize performance".to_string(),
            }
        } else if error_lower.contains("memory") || error_lower.contains("out of memory") {
            ErrorAnalysis {
                error_type: ErrorType::Memory,
                severity: ErrorSeverity::High,
                suggested_fix: "Optimize memory usage or break into smaller tasks".to_string(),
            }
        } else if error_lower.contains("api") || error_lower.contains("rate limit") {
            ErrorAnalysis {
                error_type: ErrorType::API,
                severity: ErrorSeverity::Medium,
                suggested_fix: "Check API configuration and rate limits".to_string(),
            }
        } else {
            ErrorAnalysis {
                error_type: ErrorType::Unknown,
                severity: ErrorSeverity::Medium,
                suggested_fix: "Investigate the error and try a different approach".to_string(),
            }
        }
    }

    async fn create_recovery_task(&self, original_task: &AutomationTask, analysis: &ErrorAnalysis) -> Result<Option<AutomationTask>> {
        match analysis.error_type {
            ErrorType::Compilation => {
                // Create a task to fix compilation errors
                let mut recovery_task = original_task.clone();
                recovery_task.id = uuid::Uuid::new_v4();
                recovery_task.title = format!("Fix compilation errors in {}", original_task.title);
                recovery_task.description = format!(
                    "Fix compilation errors in the original task. Original error: {}. Suggested fix: {}",
                    analysis.suggested_fix, analysis.suggested_fix
                );
                recovery_task.dependencies = Vec::new();
                recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                
                Ok(Some(recovery_task))
            }
            ErrorType::Permission => {
                // Create a task to check and fix permissions
                let mut recovery_task = original_task.clone();
                recovery_task.id = uuid::Uuid::new_v4();
                recovery_task.title = format!("Fix permissions for {}", original_task.title);
                recovery_task.description = format!(
                    "Check and fix file permissions. Original error: {}. Suggested fix: {}",
                    analysis.suggested_fix, analysis.suggested_fix
                );
                recovery_task.task_type = TaskType::Configuration;
                recovery_task.dependencies = Vec::new();
                recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                
                Ok(Some(recovery_task))
            }
            ErrorType::Network => {
                // Retry with network considerations
                let mut recovery_task = original_task.clone();
                recovery_task.id = uuid::Uuid::new_v4();
                recovery_task.title = format!("Retry {} with network fixes", original_task.title);
                recovery_task.description = format!(
                    "Retry the original task with network error handling. Original error: {}. Suggested fix: {}",
                    analysis.suggested_fix, analysis.suggested_fix
                );
                recovery_task.dependencies = Vec::new();
                recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                
                Ok(Some(recovery_task))
            }
            ErrorType::Timeout => {
                // Break down the task into smaller pieces
                let mut recovery_task = original_task.clone();
                recovery_task.id = uuid::Uuid::new_v4();
                recovery_task.title = format!("Break down {} into smaller tasks", original_task.title);
                recovery_task.description = format!(
                    "Break down the original task to avoid timeouts. Original error: {}. Suggested fix: {}",
                    analysis.suggested_fix, analysis.suggested_fix
                );
                recovery_task.task_type = TaskType::Analysis;
                recovery_task.dependencies = Vec::new();
                recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                
                Ok(Some(recovery_task))
            }
            ErrorType::Memory => {
                // Optimize for memory usage
                let mut recovery_task = original_task.clone();
                recovery_task.id = uuid::Uuid::new_v4();
                recovery_task.title = format!("Optimize {} for memory usage", original_task.title);
                recovery_task.description = format!(
                    "Optimize the task for better memory usage. Original error: {}. Suggested fix: {}",
                    analysis.suggested_fix, analysis.suggested_fix
                );
                recovery_task.dependencies = Vec::new();
                recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                
                Ok(Some(recovery_task))
            }
            ErrorType::API => {
                // Fix API configuration
                let mut recovery_task = original_task.clone();
                recovery_task.id = uuid::Uuid::new_v4();
                recovery_task.title = format!("Fix API configuration for {}", original_task.title);
                recovery_task.description = format!(
                    "Fix API configuration issues. Original error: {}. Suggested fix: {}",
                    analysis.suggested_fix, analysis.suggested_fix
                );
                recovery_task.task_type = TaskType::Configuration;
                recovery_task.dependencies = Vec::new();
                recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
                
                Ok(Some(recovery_task))
            }
            ErrorType::Unknown => {
                // Use LLM to suggest recovery
                let error_msg = format!("Unknown error occurred in task: {}", original_task.title);
                self.llm_recovery_suggestion(original_task, &error_msg).await
            }
        }
    }

    async fn llm_recovery_suggestion(&self, task: &AutomationTask, error: &str) -> Result<Option<AutomationTask>> {
        let prompt = format!(
            r#"Analyze the following error and suggest a recovery approach:

Task: {}
Description: {}
Type: {:?}

Error: {}

Please suggest a specific recovery task that would fix this error. Respond with a JSON object containing:
{{
  "should_retry": boolean,
  "modified_description": "string",
  "recovery_approach": "string"
}}"#,
            task.title, task.description, task.task_type, error
        );

        let request_body = serde_json::json!({
            "model": self.config.llm_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are an expert error recovery specialist. Analyze errors and suggest specific recovery approaches."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 1000
        });

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_ref().ok_or("API key required")?))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_text = response.text().await?;
        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        
        if let Some(content) = response_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        {
            if let Ok(suggestion) = serde_json::from_str::<serde_json::Value>(content) {
                if let Some(should_retry) = suggestion.get("should_retry").and_then(|v| v.as_bool()) {
                    if should_retry {
                        if let Some(modified_desc) = suggestion.get("modified_description").and_then(|v| v.as_str()) {
                            let mut recovery_task = task.clone();
                            recovery_task.id = uuid::Uuid::new_v4();
                            recovery_task.title = format!("Retry {} with modifications", task.title);
                            recovery_task.description = modified_desc.to_string();
                            recovery_task.dependencies = Vec::new();
                            recovery_task.created_at = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .to_string();
                            recovery_task.updated_at = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs()
                                .to_string();
                            
                            return Ok(Some(recovery_task));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone)]
struct ErrorAnalysis {
    error_type: ErrorType,
    severity: ErrorSeverity,
    suggested_fix: String,
}

#[derive(Debug, Clone)]
enum ErrorType {
    Compilation,
    Permission,
    Network,
    Timeout,
    Memory,
    API,
    Unknown,
}

#[derive(Debug, Clone)]
enum ErrorSeverity {
    Low,
    Medium,
    High,
}

#[async_trait]
impl ErrorRecovery for SmartRecovery {
    async fn recover(&self, error: &str, task: &AutomationTask) -> Result<Option<AutomationTask>> {
        log::info!("Attempting recovery for task: {}, error: {}", task.title, error);
        
        let analysis = self.analyze_error(error).await;
        
        match analysis.severity {
            ErrorSeverity::High => {
                log::warn!("High severity error detected: {}", analysis.suggested_fix);
                self.create_recovery_task(task, &analysis).await
            }
            ErrorSeverity::Medium => {
                log::info!("Medium severity error: {}", analysis.suggested_fix);
                self.create_recovery_task(task, &analysis).await
            }
            ErrorSeverity::Low => {
                log::debug!("Low severity error: {}", analysis.suggested_fix);
                // For low severity errors, we might just retry
                Ok(None)
            }
        }
    }

    fn get_recovery_strategy(&self, error: &str, _task: &AutomationTask) -> RecoveryStrategy {
        let error_lower = error.to_lowercase();
        
        if error_lower.contains("compilation") || error_lower.contains("syntax") {
            RecoveryStrategy::ModifyApproach
        } else if error_lower.contains("timeout") {
            RecoveryStrategy::BreakDownTask
        } else if error_lower.contains("network") || error_lower.contains("connection") {
            RecoveryStrategy::Retry
        } else if error_lower.contains("permission") {
            RecoveryStrategy::ModifyApproach
        } else {
            RecoveryStrategy::ModifyApproach
        }
    }
}

pub struct SimpleRecovery {
    config: AutomationConfig,
}

impl SimpleRecovery {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl ErrorRecovery for SimpleRecovery {
    async fn recover(&self, error: &str, task: &AutomationTask) -> Result<Option<AutomationTask>> {
        log::info!("Simple recovery for task: {}, error: {}", task.title, error);
        
        // Simple recovery: just retry with a modified description
        if error.contains("failed") || error.contains("error") {
            let mut recovery_task = task.clone();
            recovery_task.id = uuid::Uuid::new_v4();
            recovery_task.title = format!("Retry: {}", task.title);
            recovery_task.description = format!(
                "Retry the original task with error handling. Original error: {}",
                error
            );
            recovery_task.dependencies = Vec::new();
            recovery_task.created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
            recovery_task.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string();
            
            Ok(Some(recovery_task))
        } else {
            Ok(None)
        }
    }

    fn get_recovery_strategy(&self, _error: &str, _task: &AutomationTask) -> RecoveryStrategy {
        RecoveryStrategy::Retry
    }
}
