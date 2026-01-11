use base64::{engine::general_purpose, Engine as _};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::services::audit::{now_ms, AuditEntry, AuditLog};
use crate::services::tools::ToolResult;

const TERMINAL_OUTPUT_EVENT: &str = "terminal-output";

#[derive(Clone)]
pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
    order: Arc<Mutex<Vec<String>>>,
    logs_dir: PathBuf,
}

#[derive(Clone, Serialize)]
pub struct TerminalSessionInfo {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub log_path: String,
    pub created_at_ms: u128,
    pub is_alive: bool,
}

#[derive(Deserialize)]
pub struct TerminalCreateRequest {
    pub shell: Option<String>,
    pub shell_args: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct TerminalWriteRequest {
    pub session_id: String,
    pub data_base64: String,
}

#[derive(Deserialize)]
pub struct TerminalResizeRequest {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Deserialize)]
pub struct TerminalKillRequest {
    pub session_id: String,
}

#[derive(Deserialize)]
pub struct TerminalReplayRequest {
    pub session_id: String,
    pub max_bytes: usize,
}

#[derive(Deserialize)]
pub struct TerminalExecRequest {
    pub command: String,
    pub session_id: Option<String>,
    pub shell: Option<String>,
    pub cwd: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub timeout_ms: Option<u64>,
    pub max_bytes: Option<usize>,
}

#[derive(Deserialize)]
pub struct TerminalSetTitleRequest {
    pub session_id: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct TerminalSetOrderRequest {
    pub session_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct TerminalReplayResponse {
    pub session_id: String,
    pub data_base64: String,
    pub bytes: usize,
    pub truncated: bool,
}

struct PtySession {
    info: TerminalSessionInfo,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send>,
}

#[derive(Clone, Serialize)]
struct TerminalOutputEvent {
    session_id: String,
    data_base64: String,
}

impl TerminalManager {
    pub fn new(logs_dir: PathBuf) -> Self {
        let _ = create_dir_all(&logs_dir);
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            order: Arc::new(Mutex::new(Vec::new())),
            logs_dir,
        }
    }

    pub fn create_session(
        &self,
        app_handle: AppHandle,
        request: TerminalCreateRequest,
        cwd: PathBuf,
        audit: &AuditLog,
    ) -> Result<TerminalSessionInfo, String> {
        let shell = request
            .shell
            .unwrap_or_else(|| default_shell().to_string());
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: request.rows,
                cols: request.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let mut cmd = CommandBuilder::new(shell.clone());
        if let Some(args) = request.shell_args.clone() {
            cmd.args(args);
        }
        cmd.cwd(cwd.clone());

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| e.to_string())?;

        let master = pair.master;
        let reader = master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = master.take_writer().map_err(|e| e.to_string())?;

        let id = Uuid::new_v4().to_string();
        let log_path = self.log_path_for(&id);

        if let Some(parent) = log_path.parent() {
            let _ = create_dir_all(parent);
        }
        let _ = OpenOptions::new().create(true).append(true).open(&log_path);

        spawn_reader_thread(app_handle, id.clone(), log_path.clone(), reader);

        let title = request
            .title
            .clone()
            .unwrap_or_else(|| "Session".to_string());

        let info = TerminalSessionInfo {
            id: id.clone(),
            title,
            cwd: cwd.to_string_lossy().to_string(),
            shell: shell.clone(),
            cols: request.cols,
            rows: request.rows,
            log_path: log_path.to_string_lossy().to_string(),
            created_at_ms: now_ms(),
            is_alive: true,
        };

        let session = PtySession {
            info: info.clone(),
            master,
            writer,
            child,
        };

        self.sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?
            .insert(id.clone(), session);
        if let Ok(mut order) = self.order.lock() {
            order.push(id.clone());
        }

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.create_session".to_string(),
            session_id: Some(id),
            command: Some(shell),
            payload: serde_json::json!({
                "cwd": info.cwd,
                "cols": info.cols,
                "rows": info.rows,
                "log_path": info.log_path,
                "title": info.title,
            }),
        });

        Ok(info)
    }

    pub fn write(&self, request: TerminalWriteRequest, audit: &AuditLog) -> Result<(), String> {
        let data = general_purpose::STANDARD
            .decode(request.data_base64.as_bytes())
            .map_err(|e| format!("Invalid base64 input: {}", e))?;

        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?;
        let session = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        session
            .writer
            .write_all(&data)
            .map_err(|e| e.to_string())?;

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.write_stdin".to_string(),
            session_id: Some(request.session_id),
            command: None,
            payload: serde_json::json!({
                "bytes": data.len(),
            }),
        });

        Ok(())
    }

    pub fn resize(&self, request: TerminalResizeRequest, audit: &AuditLog) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?;
        let session = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        session
            .master
            .resize(PtySize {
                rows: request.rows,
                cols: request.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        session.info.cols = request.cols;
        session.info.rows = request.rows;

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.resize".to_string(),
            session_id: Some(request.session_id),
            command: None,
            payload: serde_json::json!({
                "cols": request.cols,
                "rows": request.rows,
            }),
        });

        Ok(())
    }

    pub fn kill(&self, request: TerminalKillRequest, audit: &AuditLog) -> Result<(), String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?;
        if let Some(mut session) = sessions.remove(&request.session_id) {
            let _ = session.child.kill();
            let _ = session.child.wait();
        }
        if let Ok(mut order) = self.order.lock() {
            if let Some(index) = order.iter().position(|id| id == &request.session_id) {
                order.remove(index);
            }
        }

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.kill_session".to_string(),
            session_id: Some(request.session_id),
            command: None,
            payload: serde_json::json!({}),
        });

        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<TerminalSessionInfo>, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?;
        let mut order = self
            .order
            .lock()
            .map_err(|_| "Terminal session order lock poisoned".to_string())?;
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for id in order.iter() {
            if let Some(session) = sessions.get(id) {
                result.push(session.info.clone());
                seen.insert(id.clone());
            }
        }
        for (id, session) in sessions.iter() {
            if !seen.contains(id) {
                result.push(session.info.clone());
                order.push(id.clone());
            }
        }
        Ok(result)
    }

    pub fn replay(&self, request: TerminalReplayRequest) -> Result<TerminalReplayResponse, String> {
        let log_path = self.log_path_for(&request.session_id);
        let mut file = File::open(&log_path)
            .map_err(|e| format!("Unable to open log: {}", e))?;
        let metadata = file
            .metadata()
            .map_err(|e| format!("Unable to read log metadata: {}", e))?;
        let len = metadata.len() as usize;
        let max_bytes = request.max_bytes.min(len);
        let start = len.saturating_sub(max_bytes) as u64;
        if start > 0 {
            file.seek(SeekFrom::Start(start))
                .map_err(|e| format!("Unable to seek log: {}", e))?;
        }
        let mut buffer = Vec::with_capacity(max_bytes);
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Unable to read log: {}", e))?;

        Ok(TerminalReplayResponse {
            session_id: request.session_id,
            data_base64: general_purpose::STANDARD.encode(&buffer),
            bytes: buffer.len(),
            truncated: len > buffer.len(),
        })
    }

    pub fn exec_interactive(
        &self,
        request: TerminalExecRequest,
        cwd: PathBuf,
        audit: &AuditLog,
    ) -> Result<ToolResult, String> {
        if let Some(session_id) = request.session_id.clone() {
            return self.exec_in_session(request, session_id, audit);
        }
        self.exec_in_new_session(request, cwd, audit)
    }

    fn exec_in_new_session(
        &self,
        request: TerminalExecRequest,
        cwd: PathBuf,
        audit: &AuditLog,
    ) -> Result<ToolResult, String> {
        let shell = request.shell.unwrap_or_else(|| default_shell().to_string());
        let cols = request.cols.unwrap_or(120);
        let rows = request.rows.unwrap_or(30);
        let timeout_ms = request.timeout_ms.unwrap_or(15000);
        let max_bytes = request.max_bytes.unwrap_or(24000).min(200_000);
        let token = short_token();

        let (start_marker, end_marker_prefix, start_cmd, end_cmd, wrap_script) =
            build_shell_markers(&shell, &token);
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let mut cmd = CommandBuilder::new(shell.clone());
        cmd.cwd(cwd.clone());
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| e.to_string())?;

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let command_block = build_command_block(&request.command, &start_cmd, &end_cmd, wrap_script);
        writer
            .write_all(command_block.as_bytes())
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;

        let (raw_output, mut exit_code, truncated, mut timed_out) =
            read_until_markers_from_reader(
                &mut reader,
                &start_marker,
                &end_marker_prefix,
                timeout_ms,
                max_bytes,
            );
        if timed_out && !raw_output.trim().is_empty() {
            exit_code = Some(0);
            timed_out = false;
        }

        let _ = child.kill();
        let _ = child.wait();

        let ok = exit_code.unwrap_or(1) == 0;
        let stderr_excerpt = if timed_out {
            Some("Timeout waiting for command completion.".to_string())
        } else {
            None
        };

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.exec_interactive".to_string(),
            session_id: None,
            command: Some(request.command),
            payload: serde_json::json!({
                "cwd": cwd.to_string_lossy(),
                "shell": shell,
                "exit_code": exit_code,
                "timeout_ms": timeout_ms,
                "max_bytes": max_bytes,
                "truncated": truncated,
            }),
        });

        Ok(ToolResult {
            ok,
            stdout_excerpt: Some(raw_output),
            stderr_excerpt,
            exit_code,
            artifacts: Some(serde_json::json!({
                "truncated": truncated,
            })),
            next_suggestion: None,
        })
    }

    fn exec_in_session(
        &self,
        request: TerminalExecRequest,
        session_id: String,
        audit: &AuditLog,
    ) -> Result<ToolResult, String> {
        let timeout_ms = request.timeout_ms.unwrap_or(15000);
        let max_bytes = request.max_bytes.unwrap_or(24000).min(200_000);
        let (shell, log_path, start_marker, end_marker_prefix, start_pos) = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "Terminal session lock poisoned".to_string())?;
            let session = sessions
                .get_mut(&session_id)
                .ok_or_else(|| "Session not found".to_string())?;
            let shell = session.info.shell.clone();
            let log_path = PathBuf::from(&session.info.log_path);
            let start_pos = OpenOptions::new()
                .read(true)
                .create(true)
                .open(&log_path)
                .and_then(|file| file.metadata())
                .map(|meta| meta.len())
                .unwrap_or(0);
            let token = short_token();
            let (start_marker, end_marker_prefix, start_cmd, end_cmd, wrap_script) =
                build_shell_markers(&shell, &token);
            let command_block =
                build_command_block(&request.command, &start_cmd, &end_cmd, wrap_script);
            session
                .writer
                .write_all(command_block.as_bytes())
                .map_err(|e| e.to_string())?;
            session.writer.flush().map_err(|e| e.to_string())?;
            (shell, log_path, start_marker, end_marker_prefix, start_pos)
        };

        let (raw_output, mut exit_code, truncated, mut timed_out) = read_until_markers_from_log(
            &log_path,
            start_pos,
            &start_marker,
            &end_marker_prefix,
            timeout_ms,
            max_bytes,
        )?;
        if timed_out && !raw_output.trim().is_empty() {
            exit_code = Some(0);
            timed_out = false;
        }
        let ok = exit_code.unwrap_or(1) == 0;
        let stderr_excerpt = if timed_out {
            Some("Timeout waiting for command completion.".to_string())
        } else {
            None
        };

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.exec_interactive".to_string(),
            session_id: Some(session_id),
            command: Some(request.command),
            payload: serde_json::json!({
                "shell": shell,
                "exit_code": exit_code,
                "timeout_ms": timeout_ms,
                "max_bytes": max_bytes,
                "truncated": truncated,
            }),
        });

        Ok(ToolResult {
            ok,
            stdout_excerpt: Some(raw_output),
            stderr_excerpt,
            exit_code,
            artifacts: Some(serde_json::json!({
                "truncated": truncated,
            })),
            next_suggestion: None,
        })
    }

    pub fn set_title(
        &self,
        request: TerminalSetTitleRequest,
        audit: &AuditLog,
    ) -> Result<(), String> {
        let title = request.title.trim().to_string();
        if title.is_empty() {
            return Err("Title cannot be empty".to_string());
        }
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?;
        let session = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| "Session not found".to_string())?;
        session.info.title = title.clone();

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.set_title".to_string(),
            session_id: Some(request.session_id),
            command: None,
            payload: serde_json::json!({
                "title": title,
            }),
        });

        Ok(())
    }

    pub fn set_order(
        &self,
        request: TerminalSetOrderRequest,
        audit: &AuditLog,
    ) -> Result<Vec<String>, String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Terminal session lock poisoned".to_string())?;
        let mut next_order = Vec::new();
        let mut seen = HashSet::new();
        for id in request.session_ids.iter() {
            if !sessions.contains_key(id) {
                return Err(format!("Unknown session id: {}", id));
            }
            if seen.insert(id.clone()) {
                next_order.push(id.clone());
            }
        }
        for id in sessions.keys() {
            if !seen.contains(id) {
                next_order.push(id.clone());
            }
        }
        drop(sessions);
        let mut order = self
            .order
            .lock()
            .map_err(|_| "Terminal session order lock poisoned".to_string())?;
        *order = next_order.clone();

        audit.write(AuditEntry {
            timestamp_ms: now_ms(),
            action: "terminal.set_order".to_string(),
            session_id: None,
            command: None,
            payload: serde_json::json!({
                "order": next_order,
            }),
        });

        Ok(order.clone())
    }

    fn log_path_for(&self, session_id: &str) -> PathBuf {
        self.logs_dir.join(format!("pty-{}.log", session_id))
    }
}

fn spawn_reader_thread(
    app_handle: AppHandle,
    session_id: String,
    log_path: PathBuf,
    mut reader: Box<dyn Read + Send>,
) {
    std::thread::spawn(move || {
        if let Some(parent) = log_path.parent() {
            let _ = create_dir_all(parent);
        }
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let bytes = &buffer[..count];
                    let payload = TerminalOutputEvent {
                        session_id: session_id.clone(),
                        data_base64: general_purpose::STANDARD.encode(bytes),
                    };
                    let _ = app_handle.emit(TERMINAL_OUTPUT_EVENT, payload);
                    if let Some(file) = log_file.as_mut() {
                        let _ = file.write_all(bytes);
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn default_shell() -> &'static str {
    if cfg!(windows) {
        "powershell.exe"
    } else {
        "/bin/bash"
    }
}

fn build_shell_markers(shell: &str, token: &str) -> (String, String, String, String, bool) {
    let start_marker = format!("__TAURIHANDS_START:{}__", token);
    let end_marker_prefix = format!("__TAURIHANDS_END:{}:", token);
    let lower = shell.to_lowercase();
    if lower.contains("powershell") || lower.contains("pwsh") {
        let start_cmd = format!("Write-Output '{}'", start_marker);
        let end_cmd = format!(
            "Write-Output ('{}' + $LASTEXITCODE)",
            end_marker_prefix
        );
        (start_marker, end_marker_prefix, start_cmd, end_cmd, true)
    } else {
        let start_cmd = format!("echo \"{}\"", start_marker);
        let end_cmd = format!("echo \"{}$?\"", end_marker_prefix);
        (start_marker, end_marker_prefix, start_cmd, end_cmd, false)
    }
}

fn build_command_block(command: &str, start_cmd: &str, end_cmd: &str, wrap_script: bool) -> String {
    let newline = if cfg!(windows) { "\r\n" } else { "\n" };
    let mut command_block = String::new();
    if wrap_script {
        command_block.push_str(start_cmd);
        command_block.push_str("; & { ");
        command_block.push_str(command);
        command_block.push_str(" }; ");
        command_block.push_str(end_cmd);
        command_block.push_str(newline);
    } else {
        command_block.push_str(start_cmd);
        command_block.push_str(newline);
        command_block.push_str(command);
        command_block.push_str(newline);
        command_block.push_str(end_cmd);
        command_block.push_str(newline);
    }
    command_block
}

fn short_token() -> String {
    let full = Uuid::new_v4().to_string();
    full.chars().take(8).collect()
}

fn extract_between_markers(
    raw: &str,
    start_marker: &str,
    end_marker_prefix: &str,
) -> Option<(String, Option<i32>)> {
    let start_idx = find_marker_line(raw, start_marker)?;
    let after_marker = start_idx + start_marker.len();
    let start_line_end = raw[after_marker..]
        .find('\n')
        .map(|offset| after_marker + offset + 1)
        .unwrap_or(after_marker);
    let end_idx = find_marker_line_from(raw, end_marker_prefix, start_line_end)?;
    let captured = raw[start_line_end..end_idx].to_string();
    let after_end = end_idx + end_marker_prefix.len();
    let line_end = raw[after_end..]
        .find('\n')
        .map(|offset| after_end + offset)
        .unwrap_or(raw.len());
    let exit_segment = &raw[after_end..line_end];
    let exit_code = parse_exit_code(exit_segment);
    Some((captured, exit_code))
}

fn find_marker_line(raw: &str, marker: &str) -> Option<usize> {
    find_marker_line_from(raw, marker, 0)
}

fn find_marker_line_from(raw: &str, marker: &str, start: usize) -> Option<usize> {
    let mut cursor = start;
    while let Some(found) = raw[cursor..].find(marker) {
        let idx = cursor + found;
        if is_marker_line_start(raw, idx) {
            return Some(idx);
        }
        cursor = idx + marker.len();
    }
    None
}

fn is_marker_line_start(raw: &str, idx: usize) -> bool {
    let line_start = raw[..idx].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let bytes = raw.as_bytes();
    let mut i = line_start;
    while i < idx {
        match bytes[i] {
            b' ' | b'\t' | b'\r' => {
                i += 1;
            }
            0x1b => {
                i += 1;
                if i < idx && bytes[i] == b'[' {
                    i += 1;
                    while i < idx {
                        let b = bytes[i];
                        i += 1;
                        if (0x40..=0x7e).contains(&b) {
                            break;
                        }
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

fn read_until_markers_from_reader(
    reader: &mut dyn Read,
    start_marker: &str,
    end_marker_prefix: &str,
    timeout_ms: u64,
    max_bytes: usize,
) -> (String, Option<i32>, bool, bool) {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut raw_output = String::new();
    let mut exit_code = None;
    let mut buffer = [0u8; 8192];

    while Instant::now() < deadline {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                let chunk = String::from_utf8_lossy(&buffer[..count]);
                raw_output.push_str(&chunk);
                if let Some((captured, code)) =
                    extract_between_markers(&raw_output, start_marker, end_marker_prefix)
                {
                    raw_output = captured;
                    exit_code = code;
                    break;
                }
                if raw_output.len() > max_bytes * 4 {
                    if let Some(idx) = raw_output.find(start_marker) {
                        raw_output = raw_output[idx..].to_string();
                    } else {
                        raw_output =
                            raw_output[raw_output.len().saturating_sub(max_bytes * 2)..].to_string();
                    }
                }
            }
            Err(_) => break,
        }
    }

    let timed_out = exit_code.is_none();
    let cleaned = sanitize_terminal_output(&raw_output);
    let (stdout_excerpt, truncated) = truncate_utf8(&cleaned, max_bytes);
    (stdout_excerpt, exit_code, truncated, timed_out)
}

fn read_until_markers_from_log(
    log_path: &PathBuf,
    start_pos: u64,
    start_marker: &str,
    end_marker_prefix: &str,
    timeout_ms: u64,
    max_bytes: usize,
) -> Result<(String, Option<i32>, bool, bool), String> {
    let mut file = OpenOptions::new()
        .read(true)
        .create(true)
        .open(log_path)
        .map_err(|e| format!("Unable to read log: {}", e))?;
    file.seek(SeekFrom::Start(start_pos))
        .map_err(|e| format!("Unable to seek log: {}", e))?;

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut raw_output = String::new();
    let mut exit_code = None;
    let mut buffer = [0u8; 8192];

    while Instant::now() < deadline {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            std::thread::sleep(Duration::from_millis(40));
            continue;
        }
        let chunk = String::from_utf8_lossy(&buffer[..count]);
        raw_output.push_str(&chunk);
        if let Some((captured, code)) =
            extract_between_markers(&raw_output, start_marker, end_marker_prefix)
        {
            raw_output = captured;
            exit_code = code;
            break;
        }
        if raw_output.len() > max_bytes * 4 {
            if let Some(idx) = raw_output.find(start_marker) {
                raw_output = raw_output[idx..].to_string();
            } else {
                raw_output = raw_output[raw_output.len().saturating_sub(max_bytes * 2)..].to_string();
            }
        }
    }

    let timed_out = exit_code.is_none();
    let cleaned = sanitize_terminal_output(&raw_output);
    let (stdout_excerpt, truncated) = truncate_utf8(&cleaned, max_bytes);
    Ok((stdout_excerpt, exit_code, truncated, timed_out))
}

fn parse_exit_code(value: &str) -> Option<i32> {
    let cleaned = sanitize_terminal_output(value);
    let trimmed = cleaned.trim_start();
    let mut buf = String::new();
    for ch in trimmed.chars() {
        if ch == '-' && buf.is_empty() {
            buf.push(ch);
            continue;
        }
        if ch.is_ascii_digit() {
            buf.push(ch);
            continue;
        }
        break;
    }
    if buf.is_empty() || buf == "-" {
        return None;
    }
    buf.parse::<i32>().ok()
}

fn sanitize_terminal_output(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                while let Some(next) = chars.next() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            } else {
                let _ = chars.next();
            }
            continue;
        }
        if ch == '\u{0007}' {
            continue;
        }
        if ch == '\r' {
            continue;
        }
        output.push(ch);
    }
    output
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
