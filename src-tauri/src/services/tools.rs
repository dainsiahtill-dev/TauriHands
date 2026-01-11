use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

use crate::services::audit::{now_ms, AuditEntry, AuditLog};

const MAX_EXCERPT_BYTES: usize = 12_000;
const MAX_READ_BYTES: usize = 240_000;

#[derive(Serialize)]
pub struct ToolResult {
    pub ok: bool,
    pub stdout_excerpt: Option<String>,
    pub stderr_excerpt: Option<String>,
    pub exit_code: Option<i32>,
    pub artifacts: Option<serde_json::Value>,
    pub next_suggestion: Option<String>,
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub program: String,
    pub args: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
pub struct ReadFileRequest {
    pub path: String,
}

#[derive(Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct SearchRequest {
    pub pattern: String,
    pub paths: Option<Vec<String>>,
    pub glob: Option<String>,
    pub max_results: Option<usize>,
}

#[derive(Serialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: u64,
    pub column: u64,
    pub text: String,
}

pub fn run_command(
    request: CommandRequest,
    default_cwd: &str,
    audit: &AuditLog,
) -> Result<ToolResult, String> {
    let args = request.args.unwrap_or_default();
    if let Some(reason) = is_dangerous_command(&request.program, &args) {
        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "tool.run_command.blocked".to_string(),
            session_id: None,
            command: Some(format_command(&request.program, &args)),
            payload: serde_json::json!({ "reason": reason }),
        });
        return Err(reason);
    }

    let mut command = Command::new(&request.program);
    command.args(&args);
    command.current_dir(request.cwd.unwrap_or_else(|| default_cwd.to_string()));
    if let Some(env) = request.env {
        command.envs(env);
    }

    let output = command.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let (stdout_excerpt, stdout_truncated) = truncate_utf8(&stdout, MAX_EXCERPT_BYTES);
    let (stderr_excerpt, stderr_truncated) = truncate_utf8(&stderr, MAX_EXCERPT_BYTES);

    audit.write(AuditEntry {
        timestamp_ms: now_ms(),
        action: "tool.run_command".to_string(),
        session_id: None,
        command: Some(format_command(&request.program, &args)),
        payload: serde_json::json!({
            "cwd": default_cwd,
            "exit_code": output.status.code(),
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "stdout_truncated": stdout_truncated,
            "stderr_truncated": stderr_truncated,
            "timeout_ms": request.timeout_ms,
        }),
    });

    Ok(ToolResult {
        ok: output.status.success(),
        stdout_excerpt: Some(stdout_excerpt),
        stderr_excerpt: Some(stderr_excerpt),
        exit_code: output.status.code(),
        artifacts: None,
        next_suggestion: None,
    })
}

pub fn read_file(
    request: ReadFileRequest,
    content: String,
    truncated: bool,
    audit: &AuditLog,
) -> ToolResult {
    audit.write(AuditEntry {
        timestamp_ms: now_ms(),
        action: "fs.read_file".to_string(),
        session_id: None,
        command: None,
        payload: serde_json::json!({
            "path": request.path,
            "truncated": truncated,
        }),
    });

    ToolResult {
        ok: true,
        stdout_excerpt: None,
        stderr_excerpt: None,
        exit_code: Some(0),
        artifacts: Some(serde_json::json!({
            "path": request.path,
            "content": content,
            "truncated": truncated,
        })),
        next_suggestion: None,
    }
}

pub fn write_file(
    request: WriteFileRequest,
    bytes_written: usize,
    audit: &AuditLog,
) -> ToolResult {
    audit.write(AuditEntry {
        timestamp_ms: now_ms(),
        action: "fs.write_file".to_string(),
        session_id: None,
        command: None,
        payload: serde_json::json!({
            "path": request.path,
            "bytes_written": bytes_written,
        }),
    });

    ToolResult {
        ok: true,
        stdout_excerpt: None,
        stderr_excerpt: None,
        exit_code: Some(0),
        artifacts: Some(serde_json::json!({
            "path": request.path,
            "bytes_written": bytes_written,
        })),
        next_suggestion: None,
    }
}

pub fn search(
    request: SearchRequest,
    matches: Vec<SearchMatch>,
    audit: &AuditLog,
) -> ToolResult {
    audit.write(AuditEntry {
        timestamp_ms: now_ms(),
        action: "fs.search".to_string(),
        session_id: None,
        command: None,
        payload: serde_json::json!({
            "pattern": request.pattern,
            "paths": request.paths,
            "glob": request.glob,
            "matches": matches.len(),
            "max_results": request.max_results,
        }),
    });

    ToolResult {
        ok: true,
        stdout_excerpt: None,
        stderr_excerpt: None,
        exit_code: Some(0),
        artifacts: Some(serde_json::json!({
            "matches": matches,
        })),
        next_suggestion: None,
    }
}

pub fn max_read_bytes() -> usize {
    MAX_READ_BYTES
}

fn format_command(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    }
}

fn truncate_utf8(value: &str, max_len: usize) -> (String, bool) {
    if value.len() <= max_len {
        return (value.to_string(), false);
    }
    let mut end = max_len;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

fn is_dangerous_command(program: &str, args: &[String]) -> Option<String> {
    let mut joined = program.to_lowercase();
    if !args.is_empty() {
        joined.push(' ');
        joined.push_str(&args.join(" ").to_lowercase());
    }
    let patterns = [
        "rm -rf /",
        "rm -rf --no-preserve-root /",
        "del /s",
        "rd /s /q",
        "format",
        "shutdown",
        "reboot",
        "poweroff",
        "mkfs",
        "reg delete",
    ];
    for pattern in patterns {
        if joined.contains(pattern) {
            return Some(format!("Blocked dangerous command pattern: {}", pattern));
        }
    }
    None
}
