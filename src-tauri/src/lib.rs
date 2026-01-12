use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;
use tauri::{AppHandle, State};

mod services;

use services::audit::AuditLog;
use services::agent::{
    AgentAutoRunRequest, AgentGeneratePlanRequest, AgentManager, AgentPlanItemStatusRequest,
    AgentPlanItemsRequest, AgentRemovePlanItemRequest, AgentStartRequest, AgentState,
    AgentVerifyRequest,
};
use services::kernel::{
    KernelManager, KernelPlanStatusRequest, KernelPlanUpdateRequest, KernelStartRequest,
    KernelUserInputRequest, RunState,
};
use services::judge::JudgeRule;
use services::llm::LlmProfile;
use services::pty::{
    TerminalCreateRequest, TerminalExecRequest, TerminalKillRequest, TerminalManager,
    TerminalReplayRequest, TerminalReplayResponse, TerminalResizeRequest, TerminalSessionInfo,
    TerminalSetOrderRequest, TerminalSetTitleRequest, TerminalWriteRequest,
};
use services::tools::{
    max_read_bytes, read_file, run_command, search, write_file, CommandRequest, ReadFileRequest,
    SearchMatch, SearchRequest, ToolResult, WriteFileRequest,
};
use services::workspace::{
    default_workspace_root, display_path, resolve_read_path_with_fallback, WorkspaceState,
};

#[derive(Clone)]
struct AppState {
    terminal: TerminalManager,
    workspace: WorkspaceState,
    audit: AuditLog,
    agent: AgentManager,
    kernel: KernelManager,
    settings_path: PathBuf,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskBudget {
    max_iterations: Option<u32>,
    max_tool_calls: Option<u32>,
    max_wall_time_ms: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskRiskPolicy {
    allow_network: bool,
    command_policy: String,
    path_policy: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskConfig {
    task_id: String,
    workspace: String,
    goal: String,
    completion: Vec<String>,
    budget: TaskBudget,
    risk_policy: TaskRiskPolicy,
    autonomy: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskPointer {
    task_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceSettings {
    last_workspace: String,
}

#[derive(Deserialize)]
struct JudgeRulesRequest {
    task_id: String,
    rules: Vec<JudgeRule>,
}

#[derive(Deserialize)]
struct GitDiffRequest {
    path: Option<String>,
}

#[derive(Clone, Serialize)]
struct TreeNode {
    name: String,
    path: String,
    #[serde(rename = "type")]
    node_type: String,
    children: Option<Vec<TreeNode>>,
}

#[tauri::command]
fn get_workspace_root(state: State<AppState>) -> Result<String, String> {
    Ok(display_path(&state.workspace.root()))
}

#[tauri::command]
fn set_workspace_root(state: State<AppState>, root: String) -> Result<String, String> {
    let resolved = state.workspace.set_root(&root)?;
    state.kernel.update_workspace_root(resolved.clone());
    let display = display_path(&resolved);
    save_workspace_settings(&state.settings_path, &display)?;
    Ok(display)
}

#[tauri::command]
fn terminal_create_session(
    app: AppHandle,
    state: State<AppState>,
    request: TerminalCreateRequest,
) -> Result<TerminalSessionInfo, String> {
    let cwd = match &request.cwd {
        Some(path) => state.workspace.resolve_path(path)?,
        None => state.workspace.root(),
    };
    if !cwd.is_dir() {
        return Err("cwd must be a directory".to_string());
    }
    state
        .terminal
        .create_session(app, request, cwd, &state.audit)
}

#[tauri::command]
fn terminal_write(state: State<AppState>, request: TerminalWriteRequest) -> Result<(), String> {
    state.terminal.write(request, &state.audit)
}

#[tauri::command]
fn terminal_resize(state: State<AppState>, request: TerminalResizeRequest) -> Result<(), String> {
    state.terminal.resize(request, &state.audit)
}

#[tauri::command]
fn terminal_kill(state: State<AppState>, request: TerminalKillRequest) -> Result<(), String> {
    state.terminal.kill(request, &state.audit)
}

#[tauri::command]
fn terminal_list_sessions(state: State<AppState>) -> Result<Vec<TerminalSessionInfo>, String> {
    state.terminal.list_sessions()
}

#[tauri::command]
fn terminal_replay(
    state: State<AppState>,
    request: TerminalReplayRequest,
) -> Result<TerminalReplayResponse, String> {
    state.terminal.replay(request)
}

#[tauri::command]
fn terminal_exec_interactive(
    state: State<AppState>,
    request: TerminalExecRequest,
) -> Result<ToolResult, String> {
    let cwd = if request.session_id.is_some() {
        state.workspace.root()
    } else {
        match &request.cwd {
            Some(path) => state.workspace.resolve_path(path)?,
            None => state.workspace.root(),
        }
    };
    state.terminal.exec_interactive(request, cwd, &state.audit)
}

#[tauri::command]
fn terminal_set_title(state: State<AppState>, request: TerminalSetTitleRequest) -> Result<(), String> {
    state.terminal.set_title(request, &state.audit)
}

#[tauri::command]
fn terminal_set_order(
    state: State<AppState>,
    request: TerminalSetOrderRequest,
) -> Result<Vec<String>, String> {
    state.terminal.set_order(request, &state.audit)
}

#[tauri::command]
fn tool_run_command(state: State<AppState>, request: CommandRequest) -> Result<ToolResult, String> {
    let cwd = match &request.cwd {
        Some(path) => state.workspace.resolve_path(path)?,
        None => state.workspace.root(),
    };
    let mut request = request;
    request.cwd = Some(cwd.to_string_lossy().to_string());
    run_command(request, cwd.to_string_lossy().as_ref(), &state.audit)
}

#[tauri::command]
fn fs_read_file(state: State<AppState>, request: ReadFileRequest) -> Result<ToolResult, String> {
    let path = resolve_read_path_with_fallback(&state.workspace, &request.path)?;
    let max_bytes = max_read_bytes();
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    let mut handle = file.take(max_bytes as u64);
    handle.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let truncated = metadata.len() as usize > buffer.len();
    let content = String::from_utf8_lossy(&buffer).to_string();
    Ok(read_file(request, content, truncated, &state.audit))
}

#[tauri::command]
fn fs_write_file(state: State<AppState>, request: WriteFileRequest) -> Result<ToolResult, String> {
    let path = state.workspace.resolve_path_for_write(&request.path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, request.content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(write_file(request, path.metadata().map(|m| m.len() as usize).unwrap_or(0), &state.audit))
}

#[tauri::command]
fn fs_search(state: State<AppState>, request: SearchRequest) -> Result<ToolResult, String> {
    let trimmed = request.pattern.trim();
    let (paths, mut globs) = resolve_search_targets(&state.workspace, &request.paths);
    if let Some(glob) = &request.glob {
        globs.push(glob.clone());
    }

    if trimmed == "*" {
        let output = run_rg_files(&paths, &globs)?;
        let max_results = request.max_results.unwrap_or(200);
        let matches = parse_rg_files(&output, max_results);
        return Ok(search(request, matches, &state.audit));
    }

    let (pattern, force_fixed) = normalize_search_pattern(trimmed);
    let output = run_rg_search(&pattern, &paths, &globs, force_fixed)?;
    let max_results = request.max_results.unwrap_or(200);
    let matches = parse_rg_json(&output, max_results);
    Ok(search(request, matches, &state.audit))
}

#[tauri::command]
fn git_status(state: State<AppState>) -> Result<ToolResult, String> {
    let request = CommandRequest {
        program: "git".to_string(),
        args: Some(vec!["status".into(), "--porcelain=v1".into(), "--untracked-files=all".into()]),
        cwd: Some(state.workspace.root().to_string_lossy().to_string()),
        env: None,
        timeout_ms: None,
    };
    run_command(request, state.workspace.root().to_string_lossy().as_ref(), &state.audit)
}

#[tauri::command]
fn git_diff(state: State<AppState>, request: GitDiffRequest) -> Result<ToolResult, String> {
    let mut args = vec!["diff".to_string()];
    if let Some(path) = request.path {
        let resolved = state.workspace.resolve_path(&path)?;
        args.push("--".to_string());
        args.push(resolved.to_string_lossy().to_string());
    }
    let request = CommandRequest {
        program: "git".to_string(),
        args: Some(args),
        cwd: Some(state.workspace.root().to_string_lossy().to_string()),
        env: None,
        timeout_ms: None,
    };
    run_command(request, state.workspace.root().to_string_lossy().as_ref(), &state.audit)
}

#[tauri::command]
fn tests_run(state: State<AppState>, request: CommandRequest) -> Result<ToolResult, String> {
    tool_run_command(state, request)
}

#[tauri::command]
fn fs_list_tree(
    state: State<AppState>,
    max_depth: Option<usize>,
    max_entries: Option<usize>,
    show_hidden: Option<bool>,
) -> Result<Vec<TreeNode>, String> {
    let root = state.workspace.root();
    let max_depth = max_depth.unwrap_or(4);
    let max_entries = max_entries.unwrap_or(2000);
    let show_hidden = show_hidden.unwrap_or(false);
    let mut count = 0usize;
    list_tree(
        &root,
        &root,
        0,
        max_depth,
        max_entries,
        show_hidden,
        &mut count,
    )
}

#[tauri::command]
fn agent_get_state(state: State<AppState>) -> Result<AgentState, String> {
    Ok(state.agent.snapshot())
}

#[tauri::command]
fn agent_start(
    app: AppHandle,
    state: State<AppState>,
    request: AgentStartRequest,
) -> Result<AgentState, String> {
    state.agent.start(
        app,
        state.terminal.clone(),
        state.workspace.clone(),
        state.audit.clone(),
        request,
    )
}

#[tauri::command]
fn agent_pause(app: AppHandle, state: State<AppState>) -> Result<AgentState, String> {
    state.agent.pause(&app)
}

#[tauri::command]
fn agent_resume(app: AppHandle, state: State<AppState>) -> Result<AgentState, String> {
    state.agent.resume(&app)
}

#[tauri::command]
fn agent_reset(app: AppHandle, state: State<AppState>) -> Result<AgentState, String> {
    state.agent.reset(&app)
}

#[tauri::command]
fn agent_set_auto_run(
    app: AppHandle,
    state: State<AppState>,
    request: AgentAutoRunRequest,
) -> Result<AgentState, String> {
    state.agent.set_auto_run(&app, request.auto_run)
}

#[tauri::command]
fn agent_set_verify_preset(
    app: AppHandle,
    state: State<AppState>,
    request: AgentVerifyRequest,
) -> Result<AgentState, String> {
    state.agent.set_verify_preset(&app, request.preset)
}

#[tauri::command]
fn agent_add_plan_items(
    app: AppHandle,
    state: State<AppState>,
    request: AgentPlanItemsRequest,
) -> Result<AgentState, String> {
    state.agent.add_plan_items(&app, request.items)
}

#[tauri::command]
fn agent_remove_plan_item(
    app: AppHandle,
    state: State<AppState>,
    request: AgentRemovePlanItemRequest,
) -> Result<AgentState, String> {
    state.agent.remove_plan_item(&app, request.id)
}

#[tauri::command]
fn agent_clear_plan_items(app: AppHandle, state: State<AppState>) -> Result<AgentState, String> {
    state.agent.clear_plan_items(&app)
}

#[tauri::command]
fn agent_generate_plan(
    app: AppHandle,
    state: State<AppState>,
    request: AgentGeneratePlanRequest,
) -> Result<AgentState, String> {
    state.agent.generate_plan(&app, request)
}

#[tauri::command]
fn agent_skip_plan_item(
    app: AppHandle,
    state: State<AppState>,
    request: AgentPlanItemStatusRequest,
) -> Result<AgentState, String> {
    state.agent.skip_plan_item(&app, request)
}

#[tauri::command]
fn agent_retry_plan_item(
    app: AppHandle,
    state: State<AppState>,
    request: AgentPlanItemStatusRequest,
) -> Result<AgentState, String> {
    state.agent.retry_plan_item(&app, request)
}

#[tauri::command]
fn kernel_get_state(state: State<AppState>) -> Result<RunState, String> {
    Ok(state.kernel.snapshot())
}

#[tauri::command]
fn kernel_start(
    app: AppHandle,
    state: State<AppState>,
    request: KernelStartRequest,
) -> Result<RunState, String> {
    state.kernel.start(app, request)
}

#[tauri::command]
fn kernel_pause(app: AppHandle, state: State<AppState>) -> Result<RunState, String> {
    state.kernel.pause(&app)
}

#[tauri::command]
fn kernel_resume(app: AppHandle, state: State<AppState>) -> Result<RunState, String> {
    state.kernel.resume(&app)
}

#[tauri::command]
fn kernel_stop(app: AppHandle, state: State<AppState>) -> Result<RunState, String> {
    state.kernel.stop(&app)
}

#[tauri::command]
fn kernel_continue(app: AppHandle, state: State<AppState>) -> Result<RunState, String> {
    state.kernel.continue_run(&app)
}

#[tauri::command]
fn kernel_user_input(
    app: AppHandle,
    state: State<AppState>,
    request: KernelUserInputRequest,
) -> Result<RunState, String> {
    state.kernel.user_input(&app, request)
}

#[tauri::command]
async fn kernel_plan_update(
    app: AppHandle,
    state: State<'_, AppState>,
    request: KernelPlanUpdateRequest,
) -> Result<RunState, String> {
    state.kernel.update_plan(&app, request).await
}

#[tauri::command]
fn kernel_plan_status(
    app: AppHandle,
    state: State<AppState>,
    request: KernelPlanStatusRequest,
) -> Result<RunState, String> {
    state.kernel.update_plan_status(&app, request)
}

#[tauri::command]
fn llm_get_profile(state: State<AppState>) -> Result<Option<LlmProfile>, String> {
    Ok(state.kernel.get_llm_profile())
}

#[tauri::command]
fn llm_save_profile(
    state: State<AppState>,
    profile: LlmProfile,
) -> Result<LlmProfile, String> {
    state.kernel.save_llm_profile(profile)
}

#[tauri::command]
fn task_get_active(state: State<AppState>) -> Result<Option<TaskConfig>, String> {
    let root = state.workspace.root();
    let pointer_path = task_base_dir(&root).join("active.json");
    if !pointer_path.exists() {
        return Ok(None);
    }
    let pointer: TaskPointer = read_json(&pointer_path)?;
    let config_path = task_dir(&root, &pointer.task_id).join("task.json");
    if !config_path.exists() {
        return Ok(None);
    }
    let config: TaskConfig = read_json(&config_path)?;
    let rules_path = task_dir(&root, &pointer.task_id).join("judge.json");
    if rules_path.exists() {
        if let Ok(rules) = read_json(&rules_path) {
            let _ = state.kernel.set_judge_rules(rules);
        }
    }
    Ok(Some(config))
}

#[tauri::command]
fn task_save_config(state: State<AppState>, request: TaskConfig) -> Result<TaskConfig, String> {
    let root = state.workspace.root();
    let task_id = if request.task_id.trim().is_empty() {
        Uuid::new_v4().to_string()
    } else {
        request.task_id.trim().to_string()
    };
    let workspace = if request.workspace.trim().is_empty() {
        display_path(&root)
    } else {
        request.workspace.trim().to_string()
    };
    let config = TaskConfig {
        task_id: task_id.clone(),
        workspace,
        goal: request.goal,
        completion: request.completion,
        budget: request.budget,
        risk_policy: request.risk_policy,
        autonomy: request.autonomy,
    };
    let config_path = task_dir(&root, &task_id).join("task.json");
    write_json(&config_path, &config)?;
    let pointer = TaskPointer {
        task_id: task_id.clone(),
    };
    let pointer_path = task_base_dir(&root).join("active.json");
    write_json(&pointer_path, &pointer)?;
    let _ = state.kernel.set_task_id(Some(task_id));
    Ok(config)
}

#[tauri::command]
fn judge_get_rules(state: State<AppState>, task_id: String) -> Result<Vec<JudgeRule>, String> {
    if task_id.trim().is_empty() {
        return Ok(Vec::new());
    }
    let root = state.workspace.root();
    let rules_path = task_dir(&root, &task_id).join("judge.json");
    if !rules_path.exists() {
        return Ok(Vec::new());
    }
    read_json(&rules_path)
}

#[tauri::command]
fn judge_set_rules(
    state: State<AppState>,
    request: JudgeRulesRequest,
) -> Result<Vec<JudgeRule>, String> {
    if request.task_id.trim().is_empty() {
        return Err("task_id is required".to_string());
    }
    let root = state.workspace.root();
    let rules_path = task_dir(&root, &request.task_id).join("judge.json");
    write_json(&rules_path, &request.rules)?;
    let _ = state.kernel.set_judge_rules(request.rules.clone());
    Ok(request.rules)
}

fn parse_rg_json(output: &[u8], max_results: usize) -> Vec<SearchMatch> {
    let mut matches = Vec::new();
    let stdout = String::from_utf8_lossy(output);
    for line in stdout.lines() {
        if matches.len() >= max_results {
            break;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("type").and_then(|v| v.as_str()) != Some("match") {
            continue;
        }
        let data = &value["data"];
        let path = data["path"]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let line_number = data["line_number"].as_u64().unwrap_or(0);
        let text = data["lines"]["text"]
            .as_str()
            .unwrap_or_default()
            .trim_end_matches('\n')
            .to_string();
        let column = data["submatches"]
            .get(0)
            .and_then(|s| s["start"].as_u64())
            .map(|c| c + 1)
            .unwrap_or(1);
        matches.push(SearchMatch {
            path,
            line: line_number,
            column,
            text,
        });
    }
    matches
}

fn parse_rg_files(output: &[u8], max_results: usize) -> Vec<SearchMatch> {
    let mut matches = Vec::new();
    let stdout = String::from_utf8_lossy(output);
    for line in stdout.lines() {
        if matches.len() >= max_results {
            break;
        }
        let path = line.trim();
        if path.is_empty() {
            continue;
        }
        matches.push(SearchMatch {
            path: path.to_string(),
            line: 0,
            column: 0,
            text: path.to_string(),
        });
    }
    matches
}

fn normalize_search_pattern(pattern: &str) -> (String, bool) {
    let trimmed = pattern.trim();
    if trimmed.is_empty() || trimmed == "*" {
        return (".".to_string(), false);
    }
    let force_fixed = trimmed.starts_with('*') || trimmed.starts_with('?');
    (trimmed.to_string(), force_fixed)
}

fn resolve_search_targets(
    workspace: &WorkspaceState,
    paths: &Option<Vec<String>>,
) -> (Vec<PathBuf>, Vec<String>) {
    let mut resolved = Vec::new();
    let mut globs = Vec::new();
    if let Some(paths) = paths {
        for path in paths {
            if is_glob_like(path) {
                globs.push(path.to_string());
                continue;
            }
            if let Ok(found) = workspace.resolve_path(path) {
                resolved.push(found);
            }
        }
    }
    if resolved.is_empty() {
        resolved.push(workspace.root());
    }
    (resolved, globs)
}

fn is_glob_like(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}

fn run_rg_search(
    pattern: &str,
    paths: &[PathBuf],
    globs: &[String],
    force_fixed: bool,
) -> Result<Vec<u8>, String> {
    let output = run_rg_search_inner(pattern, paths, globs, force_fixed)?;
    if is_rg_ok(&output) {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !force_fixed && is_rg_regex_error(&stderr) {
        let retry = run_rg_search_inner(pattern, paths, globs, true)?;
        if is_rg_ok(&retry) {
            return Ok(retry.stdout);
        }
        let retry_err = String::from_utf8_lossy(&retry.stderr);
        return Err(retry_err.trim().to_string());
    }
    Err(stderr.trim().to_string())
}

fn run_rg_files(paths: &[PathBuf], globs: &[String]) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--files");
    for glob in globs {
        cmd.arg("--glob").arg(glob);
    }
    for path in paths {
        cmd.arg(path);
    }
    let output = cmd.output().map_err(|e| e.to_string())?;
    if is_rg_ok(&output) {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(stderr.trim().to_string())
}

fn run_rg_search_inner(
    pattern: &str,
    paths: &[PathBuf],
    globs: &[String],
    force_fixed: bool,
) -> Result<std::process::Output, String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--json");
    for glob in globs {
        cmd.arg("--glob").arg(glob);
    }
    if force_fixed {
        cmd.arg("--fixed-strings");
    }
    cmd.arg(pattern);
    for path in paths {
        cmd.arg(path);
    }
    cmd.output().map_err(|e| e.to_string())
}

fn is_rg_ok(output: &std::process::Output) -> bool {
    output.status.success() || output.status.code() == Some(1)
}

fn is_rg_regex_error(stderr: &str) -> bool {
    let lowered = stderr.to_lowercase();
    lowered.contains("regex parse error") || lowered.contains("repetition operator")
}

fn task_base_dir(root: &Path) -> std::path::PathBuf {
    root.join(".taurihands").join("tasks")
}

fn task_dir(root: &Path, task_id: &str) -> std::path::PathBuf {
    task_base_dir(root).join(task_id)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    serde_json::from_slice(&buffer).map_err(|e| e.to_string())
}

fn list_tree(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    show_hidden: bool,
    count: &mut usize,
) -> Result<Vec<TreeNode>, String> {
    if depth > max_depth {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() && is_ignored_dir(&name) {
            continue;
        }
        *count += 1;
        if *count > max_entries {
            break;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");
        let children = if file_type.is_dir() && depth < max_depth {
            Some(list_tree(
                root,
                &path,
                depth + 1,
                max_depth,
                max_entries,
                show_hidden,
                count,
            )?)
        } else {
            None
        };
        items.push(TreeNode {
            name,
            path: rel,
            node_type: if file_type.is_dir() {
                "folder".to_string()
            } else {
                "file".to_string()
            },
            children,
        });
    }
    items.sort_by(|a, b| {
        let a_key = (a.node_type != "folder", a.name.to_lowercase());
        let b_key = (b.node_type != "folder", b.name.to_lowercase());
        a_key.cmp(&b_key)
    });
    Ok(items)
}

fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".idea"
            | ".vscode"
            | ".taurihands"
            | "node_modules"
            | "dist"
            | "target"
              | "out"
      )
  }

fn workspace_settings_path(identifier: &str, fallback_root: &Path) -> PathBuf {
    if let Some(base) = app_data_root(identifier) {
        return base.join("settings.json");
    }
    fallback_root
        .join(".taurihands")
        .join("app-settings.json")
}

fn app_data_root(identifier: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        return env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .map(|base| base.join(identifier));
    }
    #[cfg(target_os = "macos")]
    {
        return env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support").join(identifier));
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        if let Ok(dir) = env::var("XDG_DATA_HOME") {
            return Some(PathBuf::from(dir).join(identifier));
        }
        return env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .map(|home| home.join(".local").join("share").join(identifier));
    }
}

fn load_workspace_settings(path: &Path) -> Option<WorkspaceSettings> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_workspace_settings(path: &Path, workspace: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let settings = WorkspaceSettings {
        last_workspace: workspace.to_string(),
    };
    let data = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    let fallback_root = default_workspace_root();
    let identifier = context.config().identifier.clone();
    let settings_path = workspace_settings_path(&identifier, &fallback_root);
    let workspace_root = load_workspace_settings(&settings_path)
        .and_then(|settings| {
            let candidate = PathBuf::from(settings.last_workspace);
            if candidate.is_dir() {
                Some(candidate)
            } else {
                None
            }
        })
        .unwrap_or(fallback_root);
    let llm_root = app_data_root(&identifier).unwrap_or_else(|| workspace_root.clone());
    let llm_store_path = llm_root.join(".taurihands").join("llm.json");
    let legacy_llm_path = workspace_root.join(".taurihands").join("llm.json");
    if !llm_store_path.exists() && legacy_llm_path.exists() {
        if let Some(parent) = llm_store_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::copy(&legacy_llm_path, &llm_store_path);
    }
    let audit = AuditLog::new(workspace_root.join(".taurihands").join("audit.log"));
    let terminal = TerminalManager::new(workspace_root.join(".taurihands").join("terminal"));
    let workspace = WorkspaceState::new(workspace_root);
    let agent = AgentManager::new();
    let kernel = KernelManager::new(
        workspace.root(),
        terminal.clone(),
        workspace.clone(),
        audit.clone(),
        llm_root,
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            terminal,
            workspace,
            audit,
            agent,
            kernel,
            settings_path,
        })
        .invoke_handler(tauri::generate_handler![
            get_workspace_root,
            set_workspace_root,
            terminal_create_session,
            terminal_write,
            terminal_resize,
            terminal_kill,
            terminal_list_sessions,
            terminal_replay,
            terminal_exec_interactive,
            terminal_set_title,
            terminal_set_order,
            tool_run_command,
            fs_read_file,
            fs_write_file,
            fs_search,
            fs_list_tree,
            git_status,
            git_diff,
            tests_run,
            agent_get_state,
            agent_start,
            agent_pause,
            agent_resume,
            agent_reset,
            agent_set_auto_run,
            agent_set_verify_preset,
            agent_add_plan_items,
            agent_remove_plan_item,
            agent_clear_plan_items,
            agent_generate_plan,
            agent_skip_plan_item,
            agent_retry_plan_item,
            kernel_get_state,
            kernel_start,
            kernel_pause,
            kernel_resume,
            kernel_stop,
            kernel_continue,
            kernel_user_input,
            kernel_plan_update,
            kernel_plan_status,
            llm_get_profile,
            llm_save_profile,
            task_get_active,
            task_save_config,
            judge_get_rules,
            judge_set_rules
        ])
        .run(context)
        .expect("error while running tauri application");
}
