use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;
use anyhow::{Context, Result};
use async_trait::async_trait;

use super::engine::{AutomationTask, AutomationResult, TaskType, TaskStatus, AutomationConfig};

#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, task: &AutomationTask) -> Result<AutomationResult>;
    fn supports_task_type(&self, task_type: &TaskType) -> bool;
}

pub struct CodeExecutor {
    config: AutomationConfig,
    client: reqwest::Client,
}

impl CodeExecutor {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: reqwest::Client::new(),
        })
    }

    async fn execute_code_generation(&self, task: &AutomationTask) -> Result<AutomationResult> {
        log::info!("Executing code generation task: {}", task.title);
        
        // Read current workspace context
        let workspace_context = self.analyze_workspace().await?;
        
        // Generate code using LLM
        let generated_code = self.generate_code(&task.description, &workspace_context).await?;
        
        // Write generated code to appropriate files
        let artifacts = self.save_generated_code(&generated_code).await?;
        
        // Run validation
        let validation_result = self.validate_generated_code(&artifacts).await?;

        Ok(AutomationResult {
            task_id: task.id,
            status: TaskStatus::Completed,
            success: validation_result,
            output: format!("Generated {} files", artifacts.len()),
            error: None,
            execution_time: std::time::Duration::from_secs(0),
            artifacts,
            metrics: HashMap::from([
                ("files_generated".to_string(), artifacts.len() as f64),
                ("lines_of_code".to_string(), self.count_lines_in_artifacts(&artifacts) as f64),
            ]),
        })
    }

    async fn execute_code_modification(&self, task: &AutomationTask) -> Result<AutomationResult> {
        log::info!("Executing code modification task: {}", task.title);

        // Find files to modify
        let target_files = self.find_target_files(&task.description).await?;
        
        let mut modified_files = Vec::new();
        
        for file_path in &target_files {
            let modification_result = self.modify_file(file_path, &task.description).await?;
            if modification_result {
                modified_files.push(file_path.clone());
            }
        }

        Ok(AutomationResult {
            task_id: task.id,
            status: TaskStatus::Completed,
            success: true,
            output: format!("Modified {} files", modified_files.len()),
            error: None,
            execution_time: std::time::Duration::from_secs(0),
            artifacts: modified_files,
            metrics: HashMap::from([
                ("files_modified".to_string(), modified_files.len() as f64),
            ]),
        })
    }

    async fn execute_testing(&self, task: &AutomationTask) -> Result<AutomationResult> {
        log::info!("Executing testing task: {}", task.title);

        // Run existing tests
        let test_results = self.run_tests().await?;
        
        // Generate additional tests if needed
        let generated_tests = self.generate_tests(&task.description).await?;
        
        // Run new tests
        let new_test_results = self.run_generated_tests(&generated_tests).await?;

        Ok(AutomationResult {
            task_id: task.id,
            status: TaskStatus::Completed,
            success: test_results.passed + new_test_results.passed > 0,
            output: format!(
                "Tests: {} passed, {} failed, {} generated",
                test_results.passed + new_test_results.passed,
                test_results.failed + new_test_results.failed,
                generated_tests.len()
            ),
            error: None,
            execution_time: std::time::Duration::from_secs(0),
            artifacts: generated_tests,
            metrics: HashMap::from([
                ("tests_passed".to_string(), (test_results.passed + new_test_results.passed) as f64),
                ("tests_failed".to_string(), (test_results.failed + new_test_results.failed) as f64),
                ("tests_generated".to_string(), generated_tests.len() as f64),
            ]),
        })
    }

    async fn execute_documentation(&self, task: &AutomationTask) -> Result<AutomationResult> {
        log::info!("Executing documentation task: {}", task.title);

        // Analyze codebase for documentation
        let code_analysis = self.analyze_codebase_for_docs().await?;
        
        // Generate documentation
        let documentation = self.generate_documentation(&task.description, &code_analysis).await?;
        
        // Save documentation files
        let doc_files = self.save_documentation(&documentation).await?;

        Ok(AutomationResult {
            task_id: task.id,
            status: TaskStatus::Completed,
            success: true,
            output: format!("Generated {} documentation files", doc_files.len()),
            error: None,
            execution_time: std::time::Duration::from_secs(0),
            artifacts: doc_files,
            metrics: HashMap::from([
                ("docs_generated".to_string(), doc_files.len() as f64),
                ("pages_written".to_string(), self.count_doc_pages(&doc_files) as f64),
            ]),
        })
    }

    async fn analyze_workspace(&self) -> Result<String> {
        let mut context = String::new();
        
        // Get project structure
        if let Ok(output) = Command::new("find")
            .arg(&self.config.workspace)
            .arg("-type")
            .arg("f")
            .arg("-name")
            .arg("*.rs")
            .output()
        {
            let files = String::from_utf8_lossy(&output.stdout);
            context.push_str(&format!("Rust files:\n{}\n", files));
        }

        // Get package.json if exists
        let package_json = self.config.workspace.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                context.push_str(&format!("package.json:\n{}\n", content));
            }
        }

        // Get Cargo.toml if exists
        let cargo_toml = self.config.workspace.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                context.push_str(&format!("Cargo.toml:\n{}\n", content));
            }
        }

        Ok(context)
    }

    async fn generate_code(&self, description: &str, context: &str) -> Result<String> {
        let prompt = format!(
            r#"Generate code based on the following requirements and context:

Requirements:
{}

Workspace Context:
{}

Please provide complete, production-ready code with proper error handling, documentation, and best practices."#,
            description, context
        );

        let request_body = serde_json::json!({
            "model": self.config.llm_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are an expert software developer. Generate clean, efficient, and well-documented code."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 4000
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

    async fn save_generated_code(&self, code: &str) -> Result<Vec<PathBuf>> {
        let mut artifacts = Vec::new();
        
        // Parse code blocks from response
        let code_blocks: Vec<&str> = code.lines()
            .filter(|line| line.trim().starts_with("```"))
            .collect();

        if code_blocks.is_empty() {
            // No code blocks, save as single file
            let file_path = self.config.workspace.join("generated_code.rs");
            std::fs::write(&file_path, code)?;
            artifacts.push(file_path);
        } else {
            // Extract and save code blocks
            let mut in_code_block = false;
            let mut current_code = String::new();
            let mut file_counter = 1;

            for line in code.lines() {
                if line.trim().starts_with("```") {
                    if in_code_block {
                        // End of code block
                        let file_path = self.config.workspace.join(format!("generated_{}.rs", file_counter));
                        std::fs::write(&file_path, &current_code)?;
                        artifacts.push(file_path);
                        
                        current_code.clear();
                        file_counter += 1;
                        in_code_block = false;
                    } else {
                        // Start of code block
                        in_code_block = true;
                    }
                } else if in_code_block {
                    current_code.push_str(line);
                    current_code.push('\n');
                }
            }
        }

        Ok(artifacts)
    }

    async fn validate_generated_code(&self, artifacts: &[PathBuf]) -> Result<bool> {
        for artifact in artifacts {
            if artifact.extension().and_then(|s| s.to_str()) == Some("rs") {
                // Try to compile Rust code
                if let Ok(output) = Command::new("rustc")
                    .arg(artifact)
                    .arg("--emit")
                    .arg("metadata")
                    .output()
                {
                    if !output.status.success() {
                        log::warn!("Code validation failed for {:?}", artifact);
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    async fn find_target_files(&self, description: &str) -> Result<Vec<PathBuf>> {
        let mut target_files = Vec::new();
        
        // Simple heuristic: look for files mentioned in description
        for word in description.split_whitespace() {
            if word.ends_with(".rs") || word.ends_with(".js") || word.ends_with(".ts") {
                let file_path = self.config.workspace.join(word);
                if file_path.exists() {
                    target_files.push(file_path);
                }
            }
        }

        // If no specific files found, search for common patterns
        if target_files.is_empty() {
            for entry in walkdir::WalkDir::new(&self.config.workspace)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "rs" || ext == "js" || ext == "ts" {
                        target_files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(target_files)
    }

    async fn modify_file(&self, file_path: &PathBuf, description: &str) -> Result<bool> {
        let current_content = std::fs::read_to_string(file_path)?;
        
        let prompt = format!(
            r#"Modify the following code based on the requirements:

Current Code:
{}

Requirements:
{}

Please provide the modified code only, without explanations."#,
            current_content, description
        );

        // Use LLM to modify the code
        let modified_code = self.generate_code(&prompt, "").await?;
        
        // Save modified code
        std::fs::write(file_path, modified_code)?;
        
        Ok(true)
    }

    async fn run_tests(&self) -> Result<TestResults> {
        let mut results = TestResults::default();

        // Try different test runners
        if self.config.workspace.join("Cargo.toml").exists() {
            if let Ok(output) = Command::new("cargo")
                .arg("test")
                .current_dir(&self.config.workspace)
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let _error_str = String::from_utf8_lossy(&output.stderr);
                
                // Parse test results
                for line in output_str.lines() {
                    if line.contains("test result:") {
                        if line.contains("ok") {
                            results.passed += line.matches("passed").count();
                        } else {
                            results.failed += line.matches("failed").count();
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn generate_tests(&self, description: &str) -> Result<Vec<PathBuf>> {
        let prompt = format!(
            r#"Generate comprehensive tests for the following requirements:

Requirements:
{}

Please provide complete test cases with proper setup, teardown, and assertions."#,
            description
        );

        let test_code = self.generate_code(&prompt, "").await?;
        let test_files = self.save_generated_code(&test_code).await?;
        
        Ok(test_files)
    }

    async fn run_generated_tests(&self, test_files: &[PathBuf]) -> Result<TestResults> {
        let mut results = TestResults::default();

        for test_file in test_files {
            if let Ok(output) = Command::new("cargo")
                .arg("test")
                .arg("--test")
                .arg(test_file)
                .current_dir(&self.config.workspace)
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                
                // Parse test results
                for line in output_str.lines() {
                    if line.contains("test result:") {
                        if line.contains("ok") {
                            results.passed += line.matches("passed").count();
                        } else {
                            results.failed += line.matches("failed").count();
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn analyze_codebase_for_docs(&self) -> Result<String> {
        let mut analysis = String::new();
        
        // Get all source files
        for entry in walkdir::WalkDir::new(&self.config.workspace)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "rs" || ext == "js" || ext == "ts" {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        analysis.push_str(&format!("\nFile: {}\n", path.display()));
                        analysis.push_str(&content);
                        analysis.push('\n');
                    }
                }
            }
        }

        Ok(analysis)
    }

    async fn generate_documentation(&self, description: &str, code_analysis: &str) -> Result<String> {
        let prompt = format!(
            r#"Generate comprehensive documentation based on the following requirements and code analysis:

Requirements:
{}

Code Analysis:
{}

Please generate well-structured documentation including:
1. Overview
2. Installation/Setup
3. Usage examples
4. API reference
5. Contributing guidelines"#,
            description, code_analysis
        );

        self.generate_code(&prompt, "").await
    }

    async fn save_documentation(&self, documentation: &str) -> Result<Vec<PathBuf>> {
        let mut doc_files = Vec::new();
        
        // Save main README
        let readme_path = self.config.workspace.join("README.md");
        std::fs::write(&readme_path, documentation)?;
        doc_files.push(readme_path);
        
        // Save additional documentation files
        let sections: Vec<&str> = documentation.split("# ").collect();
        for (_i, section) in sections.iter().enumerate().skip(1) {
            if let Some(title) = section.lines().next() {
                let filename = format!("{}.md", title.to_lowercase().replace(&[' ', '(', ')'][..], "_"));
                let file_path = self.config.workspace.join(filename);
                std::fs::write(&file_path, section)?;
                doc_files.push(file_path);
            }
        }

        Ok(doc_files)
    }

    fn count_lines_in_artifacts(&self, artifacts: &[PathBuf]) -> usize {
        artifacts
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .map(|content| content.lines().count())
            .sum()
    }

    fn count_doc_pages(&self, doc_files: &[PathBuf]) -> usize {
        doc_files.len()
    }
}

#[derive(Debug, Default)]
struct TestResults {
    passed: usize,
    failed: usize,
}

#[async_trait]
impl TaskExecutor for CodeExecutor {
    async fn execute(&self, task: &AutomationTask) -> Result<AutomationResult> {
        match task.task_type {
            TaskType::CodeGeneration => self.execute_code_generation(task).await,
            TaskType::CodeModification => self.execute_code_modification(task).await,
            TaskType::Testing => self.execute_testing(task).await,
            TaskType::Documentation => self.execute_documentation(task).await,
            _ => Ok(AutomationResult {
                task_id: task.id,
                status: TaskStatus::Failed,
                success: false,
                output: String::new(),
                error: Some(format!("Unsupported task type: {:?}", task.task_type)),
                execution_time: std::time::Duration::from_secs(0),
                artifacts: Vec::new(),
                metrics: HashMap::new(),
            }),
        }
    }

    fn supports_task_type(&self, task_type: &TaskType) -> bool {
        matches!(
            task_type,
            TaskType::CodeGeneration | TaskType::CodeModification | TaskType::Testing | TaskType::Documentation
        )
    }
}
