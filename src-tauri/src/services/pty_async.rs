use async_trait::async_trait;
use portable_pty::{CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command as TokioCommand};
use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncTerminalExecRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub timeout_secs: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncTerminalExecResponse {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsyncTerminalStreamChunk {
    pub data: String,
    pub stream_type: String, // "stdout" or "stderr"
    pub timestamp: u128,
}

pub struct AsyncTerminalManager {
    active_sessions: Arc<Mutex<HashMap<String, AsyncTerminalSession>>>,
}

struct AsyncTerminalSession {
    child: Child,
    start_time: Instant,
}

impl AsyncTerminalManager {
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute_command(&self, request: AsyncTerminalExecRequest) -> Result<AsyncTerminalExecResponse, String> {
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();

        let mut cmd = TokioCommand::new(&request.command);
        cmd.args(&request.args);

        if let Some(cwd) = &request.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(env) = &request.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let timeout_duration = request.timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));

        let execution = timeout(
            timeout_duration,
            cmd.output()
        );

        let output = execution.await
            .map_err(|_| "Command execution timed out".to_string())?
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();

        Ok(AsyncTerminalExecResponse {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms: end_time - start_time,
        })
    }

    pub async fn execute_interactive(
        &self,
        request: AsyncTerminalExecRequest,
    ) -> Result<mpsc::UnboundedReceiver<AsyncTerminalStreamChunk>, String> {
        let (tx, rx) = mpsc::unbounded_channel();
        let session_id = uuid::Uuid::new_v4().to_string();

        let mut cmd = TokioCommand::new(&request.command);
        cmd.args(&request.args);

        if let Some(cwd) = &request.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(env) = &request.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Spawn the process with piped stdout and stderr
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        let stdout = child.stdout.take()
            .ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take()
            .ok_or("Failed to capture stderr")?;

        let tx_clone = tx.clone();
        let tx_stderr = tx.clone();

        // Spawn tasks to handle stdout and stderr
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut line = String::new();
            
            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }
                
                let chunk = AsyncTerminalStreamChunk {
                    data: line.clone(),
                    stream_type: "stdout".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis(),
                };
                
                let _ = tx_clone.send(chunk);
                line.clear();
            }
        });

        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            
            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break;
                }
                
                let chunk = AsyncTerminalStreamChunk {
                    data: line.clone(),
                    stream_type: "stderr".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis(),
                };
                
                let _ = tx_stderr.send(chunk);
                line.clear();
            }
        });

        // Store session for potential management
        {
            let mut sessions = self.active_sessions.lock().unwrap();
            sessions.insert(session_id, AsyncTerminalSession {
                child,
                start_time: Instant::now(),
            });
        }

        Ok(rx)
    }

    pub async fn kill_session(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.active_sessions.lock().unwrap();
        if let Some(session) = sessions.remove(session_id) {
            session.child.kill().await
                .map_err(|e| format!("Failed to kill process: {}", e))?;
        }
        Ok(())
    }

    pub async fn wait_for_session(&self, session_id: &str) -> Result<Option<i32>, String> {
        let exit_code = {
            let mut sessions = self.active_sessions.lock().unwrap();
            if let Some(session) = sessions.get(session_id) {
                match timeout(Duration::from_secs(120), session.child.wait()).await {
                    Ok(Ok(status)) => Some(status.code()),
                    Ok(Err(e)) => return Err(format!("Process wait error: {}", e)),
                    Err(_) => return Err("Process wait timeout".to_string()),
                }
            } else {
                return Err("Session not found".to_string());
            }
        };

        // Clean up session
        {
            let mut sessions = self.active_sessions.lock().unwrap();
            sessions.remove(session_id);
        }

        Ok(exit_code)
    }
}

#[async_trait]
pub trait AsyncTerminalProvider {
    async fn execute(&self, request: AsyncTerminalExecRequest) -> Result<AsyncTerminalExecResponse, String>;
    async fn execute_interactive(
        &self,
        request: AsyncTerminalExecRequest,
    ) -> Result<mpsc::UnboundedReceiver<AsyncTerminalStreamChunk>, String>;
    async fn kill_session(&self, session_id: &str) -> Result<(), String>;
    async fn wait_for_session(&self, session_id: &str) -> Result<Option<i32>, String>;
}

#[async_trait]
impl AsyncTerminalProvider for AsyncTerminalManager {
    async fn execute(&self, request: AsyncTerminalExecRequest) -> Result<AsyncTerminalExecResponse, String> {
        self.execute_command(request).await
    }

    async fn execute_interactive(
        &self,
        request: AsyncTerminalExecRequest,
    ) -> Result<mpsc::UnboundedReceiver<AsyncTerminalStreamChunk>, String> {
        self.execute_interactive(request).await
    }

    async fn kill_session(&self, session_id: &str) -> Result<(), String> {
        self.kill_session(session_id).await
    }

    async fn wait_for_session(&self, session_id: &str) -> Result<Option<i32>, String> {
        self.wait_for_session(session_id).await
    }
}
