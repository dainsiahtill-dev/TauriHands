use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::collections::HashMap;
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::process::Command as TokioCommand;

use super::llm::LlmProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    pub model: String,
    pub reasoning_level: u8,
    pub approval_mode: CodexApprovalMode,
    pub workspace: PathBuf,
    pub enable_local_search: bool,
    pub max_tokens: Option<u32>,
    pub client_type: CodexClientType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodexApprovalMode {
    Always,
    Edit,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodexClientType {
    Local,
    Cloud,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4-codex".to_string(),
            reasoning_level: 1,
            approval_mode: CodexApprovalMode::Ask,
            workspace: std::env::current_dir().unwrap_or_default(),
            enable_local_search: true,
            max_tokens: None,
            client_type: CodexClientType::Local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexRequest {
    pub prompt: String,
    pub files: Vec<String>,
    pub context: Option<String>,
    pub model: Option<String>,
    pub reasoning_level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub files_modified: Vec<String>,
    pub tokens_used: u32,
    pub model_used: String,
    pub execution_time: std::time::Duration,
}

#[async_trait]
pub trait CodexClient: Send + Sync {
    async fn execute(&self, request: CodexRequest) -> Result<CodexResponse>;
    async fn interactive_session(&self) -> Result<()>;
    async fn code_review(&self, file_path: &PathBuf) -> Result<CodexResponse>;
    async fn search_web(&self, query: &str) -> Result<CodexResponse>;
    fn is_available(&self) -> bool;
    fn configure(&mut self, config: CodexConfig);
}

pub struct LocalCodexClient {
    config: CodexConfig,
}

impl LocalCodexClient {
    pub fn new(config: CodexConfig) -> Self {
        Self { config }
    }

    fn is_codex_available() -> bool {
        // Check if codex CLI is available
        Command::new("codex")
            .arg("--version")
            .output()
            .map(|_| true)
            .unwrap_or(false)
    }

    async fn execute_codex_command(&self, args: Vec<String>) -> Result<String> {
        let mut cmd = TokioCommand::new("codex");
        
        // Set model
        cmd.arg("--model").arg(&self.config.model);
        
        // Set reasoning level
        cmd.arg("--reasoning").arg(self.config.reasoning_level.to_string());
        
        // Set approval mode
        let approval_arg = match self.config.approval_mode {
            CodexApprovalMode::Always => "always",
            CodexApprovalMode::Edit => "edit",
            CodexApprovalMode::Ask => "ask",
        };
        cmd.arg("--approval").arg(approval_arg);
        
        // Set workspace
        cmd.arg("--path").arg(self.config.workspace.to_string_lossy().as_ref());
        
        // Add custom args
        for arg in &args {
            cmd.arg(arg);
        }
        
        // Set max tokens if specified
        if let Some(max_tokens) = self.config.max_tokens {
            cmd.arg("--max-tokens").arg(max_tokens.to_string());
        }
        
        log::info!("Executing codex with args: {:?}", args);
        
        let output = cmd
            .output()
            .await
            .context("Failed to execute codex command")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Codex command failed: {}", stderr));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    fn parse_codex_response(&self, output: &str) -> Result<CodexResponse> {
        // Parse codex output to extract structured information
        // This is a simplified parser - in production, you'd want more robust parsing
        
        let lines: Vec<&str> = output.lines().collect();
        let mut content = String::new();
        let mut reasoning = None;
        let mut files_modified = Vec::new();
        let mut tokens_used = 0;
        let model_used = self.config.model.clone();
        
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            
            // Look for reasoning indicators
            if line.contains("Reasoning:") || line.contains("🤔") {
                reasoning = Some(line.to_string());
                continue;
            }
            
            // Look for file modification indicators
            if line.contains("Modified:") || line.contains("Created:") || line.contains("Updated:") {
                files_modified.push(line.to_string());
                continue;
            }
            
            // Look for token usage
            if line.contains("Tokens used:") {
                if let Some(tokens_str) = line.split(':').nth(1) {
                    if let Ok(tokens) = tokens_str.trim().parse::<u32>() {
                        tokens_used = tokens;
                    }
                }
                continue;
            }
            
            // Main content
            if !line.starts_with("Codex") && 
               !line.contains("Reasoning:") && 
               !line.contains("Modified:") && 
               !line.contains("Created:") && 
               !line.contains("Updated:") && 
               !line.contains("Tokens used:") {
                content.push_str(line);
                content.push('\n');
            }
        }
        
        Ok(CodexResponse {
            content: content.trim().to_string(),
            reasoning,
            files_modified,
            tokens_used,
            model_used,
            execution_time: std::time::Duration::from_millis(1000), // Placeholder
        })
    }
}

#[async_trait]
impl CodexClient for LocalCodexClient {
    async fn execute(&self, request: CodexRequest) -> Result<CodexResponse> {
        log::info!("Executing Codex with prompt: {}", request.prompt);
        
        let mut args = vec![];
        
        // Add prompt
        args.push(request.prompt.clone());
        
        // Add files if any
        for file in &request.files {
            args.push("--file".to_string());
            args.push(file.clone());
        }
        
        // Add context if any
        if let Some(context) = &request.context {
            args.push("--context".to_string());
            args.push(context.clone());
        }
        
        // Override model if specified
        if let Some(model) = &request.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
        
        // Override reasoning level if specified
        if let Some(reasoning) = request.reasoning_level {
            args.push("--reasoning".to_string());
            args.push(reasoning.to_string());
        }
        
        let output = self.execute_codex_command(args).await?;
        self.parse_codex_response(&output)
    }

    async fn interactive_session(&self) -> Result<()> {
        log::info!("Starting Codex interactive session");
        
        let args = vec![];
        let output = self.execute_codex_command(args).await?;
        
        println!("Codex Interactive Session Started");
        println!("Workspace: {:?}", self.config.workspace);
        println!("Model: {}", self.config.model);
        println!("Reasoning Level: {}", self.config.reasoning_level);
        println!("Approval Mode: {:?}", self.config.approval_mode);
        println!();
        println!("Codex Output:");
        println!("{}", output);
        
        Ok(())
    }

    async fn code_review(&self, file_path: &PathBuf) -> Result<CodexResponse> {
        log::info!("Starting code review for {:?}", file_path);
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let args = vec![
            "codex".to_string(),
            "review".to_string(),
            file_path_str
        ];
        
        let output = self.execute_codex_command(args).await?;
        self.parse_codex_response(&output)
    }

    async fn search_web(&self, query: &str) -> Result<CodexResponse> {
        log::info!("Starting web search for: {}", query);
        
        let args = vec![
            "--search".to_string(),
            query.to_string()
        ];
        
        let output = self.execute_codex_command(args).await?;
        self.parse_codex_response(&output)
    }

    fn is_available(&self) -> bool {
        Self::is_codex_available()
    }

    fn configure(&mut self, config: CodexConfig) {
        self.config = config;
    }
}

pub struct CloudCodexClient {
    config: CodexConfig,
    api_key: Option<String>,
}

impl CloudCodexClient {
    pub fn new(config: CodexConfig, api_key: Option<String>) -> Self {
        Self { config, api_key }
    }
}

#[async_trait]
impl CodexClient for CloudCodexClient {

    async fn execute(&self, request: CodexRequest) -> Result<CodexResponse> {
        // For now, fall back to local codex
        // In a real implementation, you'd use OpenAI's API directly
        log::warn!("Cloud Codex not implemented, falling back to local");
        
        let local_client = LocalCodexClient::new(self.config.clone());
        local_client.execute(request).await
    }

    async fn interactive_session(&self) -> Result<()> {
        let local_client = LocalCodexClient::new(self.config.clone());
        local_client.interactive_session().await
    }

    async fn code_review(&self, file_path: &PathBuf) -> Result<CodexResponse> {
        let local_client = LocalCodexClient::new(self.config.clone());
        local_client.code_review(file_path).await
    }

    async fn search_web(&self, query: &str) -> Result<CodexResponse> {
        let local_client = LocalCodexClient::new(self.config.clone());
        local_client.search_web(query).await
    }

    fn is_available(&self) -> bool {
        // Check both local codex and API key
        LocalCodexClient::is_codex_available() || self.api_key.is_some()
    }

    fn configure(&mut self, config: CodexConfig) {
        self.config = config;
    }
}

pub struct CodexManager {
    client: Box<dyn CodexClient>,
    config: CodexConfig,
}

impl CodexManager {
    pub fn new(config: CodexConfig) -> Result<Self> {
        let client: Box<dyn CodexClient> = match config.client_type {
            CodexClientType::Local => {
                Box::new(LocalCodexClient::new(config.clone()))
            }
            CodexClientType::Cloud => {
                let api_key = std::env::var("OPENAI_API_KEY").ok();
                Box::new(CloudCodexClient::new(config.clone(), api_key))
            }
        };
        
        Ok(Self { client, config })
    }

    pub async fn execute_task(&self, prompt: &str, files: Vec<String>) -> Result<CodexResponse> {
        let request = CodexRequest {
            prompt: prompt.to_string(),
            files,
            context: None,
            model: Some(self.config.model.clone()),
            reasoning_level: Some(self.config.reasoning_level),
        };
        
        self.client.execute(request).await
    }

    pub async fn start_interactive(&self) -> Result<()> {
        self.client.interactive_session().await
    }

    pub async fn review_code(&self, file_path: &PathBuf) -> Result<CodexResponse> {
        self.client.code_review(file_path).await
    }

    pub async fn search(&self, query: &str) -> Result<CodexResponse> {
        self.client.search_web(query).await
    }

    pub fn is_available(&self) -> bool {
        self.client.is_available()
    }

    pub fn configure(&mut self, config: CodexConfig) {
        self.config = config.clone();
        self.client.configure(config);
    }
}
