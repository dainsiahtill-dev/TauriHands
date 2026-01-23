use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::{Context, Result};
use async_trait::async_trait;

use super::engine::{AutomationTask, TaskType, TaskPriority, TaskStatus, AutomationConfig};

#[async_trait]
pub trait TaskPlanner: Send + Sync {
    async fn plan(&self, description: &str) -> Result<Vec<AutomationTask>>;
    async fn refine_plan(&self, tasks: &[AutomationTask], feedback: &str) -> Result<Vec<AutomationTask>>;
}

pub struct LLMTaskPlanner {
    config: AutomationConfig,
    client: reqwest::Client,
}

impl LLMTaskPlanner {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: reqwest::Client::new(),
        })
    }

    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": self.config.llm_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are an expert software development planner. Break down complex development tasks into specific, actionable subtasks. Always respond with valid JSON."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 2000
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
            Ok(content.to_string())
        } else {
            Err(anyhow::anyhow!("Invalid LLM response format"))
        }
    }

    fn parse_task_plan(&self, response: &str) -> Result<Vec<AutomationTask>> {
        let plan_data: serde_json::Value = serde_json::from_str(response)
            .map_err(|_| anyhow::anyhow!("Failed to parse JSON response"))?;

        let tasks_array = plan_data
            .get("tasks")
            .and_then(|t| t.as_array())
            .ok_or_else(|| anyhow::anyhow!("No tasks array in response"))?;

        let mut tasks = Vec::new();
        let mut dependencies = HashMap::new();

        for (index, task_data) in tasks_array.iter().enumerate() {
            let title = task_data
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or(&format!("Task {}", index + 1))
                .to_string();

            let description = task_data
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let task_type_str = task_data
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("CodeGeneration");

            let task_type = match task_type_str {
                "CodeGeneration" => TaskType::CodeGeneration,
                "CodeModification" => TaskType::CodeModification,
                "Testing" => TaskType::Testing,
                "Documentation" => TaskType::Documentation,
                "Refactoring" => TaskType::Refactoring,
                "Debugging" => TaskType::Debugging,
                "Deployment" => TaskType::Deployment,
                "Analysis" => TaskType::Analysis,
                "Configuration" => TaskType::Configuration,
                _ => TaskType::Custom(task_type_str.to_string()),
            };

            let priority_str = task_data
                .get("priority")
                .and_then(|p| p.as_str())
                .unwrap_or("Medium");

            let priority = match priority_str {
                "Low" => TaskPriority::Low,
                "Medium" => TaskPriority::Medium,
                "High" => TaskPriority::High,
                "Critical" => TaskPriority::Critical,
                _ => TaskPriority::Medium,
            };

            let task_id = Uuid::new_v4();
            
            // Parse dependencies
            if let Some(deps) = task_data.get("dependencies").and_then(|d| d.as_array()) {
                let mut task_deps = Vec::new();
                for dep in deps {
                    if let Some(dep_title) = dep.as_str() {
                        // Find dependency task by title (simplified)
                        for (dep_index, dep_task) in tasks_array.iter().enumerate() {
                            if dep_index < index {
                                if let Some(dep_title_match) = dep_task.get("title").and_then(|t| t.as_str()) {
                                    if dep_title_match == dep_title {
                                        task_deps.push(task_id); // This should be the actual dependency ID
                                    }
                                }
                            }
                        }
                    }
                }
                dependencies.insert(task_id, task_deps);
            }

            let task = AutomationTask {
                id: task_id,
                title,
                description,
                task_type,
                priority,
                status: TaskStatus::Pending,
                dependencies: dependencies.get(&task_id).unwrap_or(&Vec::new()).clone(),
                subtasks: Vec::new(),
                metadata: HashMap::new(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
                updated_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            };

            tasks.push(task);
        }

        Ok(tasks)
    }

    fn create_planning_prompt(&self, description: &str) -> String {
        format!(
            r#"Break down the following development task into specific, actionable subtasks:

Task: {}

Please create a detailed plan with the following structure:
{{
  "tasks": [
    {{
      "title": "Clear, specific task title",
      "description": "Detailed description of what needs to be done",
      "type": "CodeGeneration|CodeModification|Testing|Documentation|Refactoring|Debugging|Deployment|Analysis|Configuration",
      "priority": "Low|Medium|High|Critical",
      "dependencies": ["List of task titles this depends on (empty if none)"]
    }}
  ]
}}

Guidelines:
1. Break down complex tasks into smaller, manageable pieces
2. Consider the logical order of operations
3. Include testing and validation steps
4. Add documentation tasks where appropriate
5. Consider deployment and configuration needs
6. Set appropriate priorities based on importance and dependencies"#,
            description
        )
    }
}

#[async_trait]
impl TaskPlanner for LLMTaskPlanner {
    async fn plan(&self, description: &str) -> Result<Vec<AutomationTask>> {
        log::info!("Planning task: {}", description);
        
        let prompt = self.create_planning_prompt(description);
        let response = self.call_llm(&prompt).await?;
        let tasks = self.parse_task_plan(&response)?;
        
        log::info!("Generated {} subtasks", tasks.len());
        Ok(tasks)
    }

    async fn refine_plan(&self, tasks: &[AutomationTask], feedback: &str) -> Result<Vec<AutomationTask>> {
        let current_plan: String = tasks
            .iter()
            .enumerate()
            .map(|(i, task)| format!(
                "{}. {} ({:?}) - {}",
                i + 1,
                task.title,
                task.task_type,
                task.description
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"Refine the following task plan based on feedback:

Current Plan:
{}

Feedback:
{}

Please provide an updated plan in the same JSON format as before."#,
            current_plan, feedback
        );

        let response = self.call_llm(&prompt).await?;
        let refined_tasks = self.parse_task_plan(&response)?;
        
        log::info!("Refined plan to {} subtasks", refined_tasks.len());
        Ok(refined_tasks)
    }
}

pub struct RuleBasedPlanner {
    config: AutomationConfig,
}

impl RuleBasedPlanner {
    pub fn new(config: AutomationConfig) -> Self {
        Self { config }
    }

    fn analyze_task_type(&self, description: &str) -> TaskType {
        let desc_lower = description.to_lowercase();
        
        if desc_lower.contains("test") || desc_lower.contains("spec") {
            TaskType::Testing
        } else if desc_lower.contains("document") || desc_lower.contains("readme") {
            TaskType::Documentation
        } else if desc_lower.contains("refactor") || desc_lower.contains("cleanup") {
            TaskType::Refactoring
        } else if desc_lower.contains("debug") || desc_lower.contains("fix") {
            TaskType::Debugging
        } else if desc_lower.contains("deploy") || desc_lower.contains("release") {
            TaskType::Deployment
        } else if desc_lower.contains("analyze") || desc_lower.contains("review") {
            TaskType::Analysis
        } else if desc_lower.contains("config") || desc_lower.contains("setup") {
            TaskType::Configuration
        } else if desc_lower.contains("modify") || desc_lower.contains("update") {
            TaskType::CodeModification
        } else {
            TaskType::CodeGeneration
        }
    }
}

#[async_trait]
impl TaskPlanner for RuleBasedPlanner {
    async fn plan(&self, description: &str) -> Result<Vec<AutomationTask>> {
        log::info!("Planning task with rules: {}", description);
        
        let task_type = self.analyze_task_type(description);
        let mut tasks = Vec::new();

        match task_type {
            TaskType::CodeGeneration => {
                tasks.push(AutomationTask {
                    id: Uuid::new_v4(),
                    title: "Analyze requirements".to_string(),
                    description: "Analyze the requirements and design the solution".to_string(),
                    task_type: TaskType::Analysis,
                    priority: TaskPriority::High,
                    status: TaskStatus::Pending,
                    dependencies: Vec::new(),
                    subtasks: Vec::new(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().to_string(),
                    updated_at: chrono::Utc::now().to_string(),
                });

                tasks.push(AutomationTask {
                    id: Uuid::new_v4(),
                    title: "Generate code".to_string(),
                    description: "Generate the main implementation".to_string(),
                    task_type: TaskType::CodeGeneration,
                    priority: TaskPriority::High,
                    status: TaskStatus::Pending,
                    dependencies: vec![tasks[0].id],
                    subtasks: Vec::new(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().to_string(),
                    updated_at: chrono::Utc::now().to_string(),
                });

                tasks.push(AutomationTask {
                    id: Uuid::new_v4(),
                    title: "Create tests".to_string(),
                    description: "Create comprehensive tests for the implementation".to_string(),
                    task_type: TaskType::Testing,
                    priority: TaskPriority::Medium,
                    status: TaskStatus::Pending,
                    dependencies: vec![tasks[1].id],
                    subtasks: Vec::new(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().to_string(),
                    updated_at: chrono::Utc::now().to_string(),
                });
            }
            TaskType::Testing => {
                tasks.push(AutomationTask {
                    id: Uuid::new_v4(),
                    title: "Create test cases".to_string(),
                    description: "Design and create test cases".to_string(),
                    task_type: TaskType::Testing,
                    priority: TaskPriority::High,
                    status: TaskStatus::Pending,
                    dependencies: Vec::new(),
                    subtasks: Vec::new(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().to_string(),
                    updated_at: chrono::Utc::now().to_string(),
                });

                tasks.push(AutomationTask {
                    id: Uuid::new_v4(),
                    title: "Run tests".to_string(),
                    description: "Execute all test cases".to_string(),
                    task_type: TaskType::Testing,
                    priority: TaskPriority::Medium,
                    status: TaskStatus::Pending,
                    dependencies: vec![tasks[0].id],
                    subtasks: Vec::new(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().to_string(),
                    updated_at: chrono::Utc::now().to_string(),
                });
            }
            _ => {
                // Default single task
                tasks.push(AutomationTask {
                    id: Uuid::new_v4(),
                    title: description.to_string(),
                    description: description.to_string(),
                    task_type,
                    priority: TaskPriority::Medium,
                    status: TaskStatus::Pending,
                    dependencies: Vec::new(),
                    subtasks: Vec::new(),
                    metadata: HashMap::new(),
                    created_at: chrono::Utc::now().to_string(),
                    updated_at: chrono::Utc::now().to_string(),
                });
            }
        }

        Ok(tasks)
    }

    async fn refine_plan(&self, tasks: &[AutomationTask], _feedback: &str) -> Result<Vec<AutomationTask>> {
        // For rule-based planner, just return the same tasks
        Ok(tasks.to_vec())
    }
}
