use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::process::Command;
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
use services::workspace::{default_workspace_root, display_path, WorkspaceState};

#[derive(Clone)]
struct AppState {
    terminal: TerminalManager,
    workspace: WorkspaceState,
    audit: AuditLog,
    agent: AgentManager,
    kernel: KernelManager,
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
    Ok(display_path(&resolved))
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
    let path = state.workspace.resolve_path(&request.path)?;
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
    let mut cmd = Command::new("rg");
    cmd.arg("--json");
    if let Some(glob) = &request.glob {
        cmd.arg("--glob").arg(glob);
    }

    if request.pattern.starts_with("*") {
        cmd.arg("--fixed-strings");
    }
    cmd.arg(&request.pattern);

    let paths = if let Some(paths) = &request.paths {
        let mut resolved = Vec::new();
        for path in paths {
            resolved.push(state.workspace.resolve_path(path)?);
        }
        resolved
    } else {
        vec![state.workspace.root()]
    };

    for path in &paths {
        cmd.arg(path);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_string());
    }

    let max_results = request.max_results.unwrap_or(200);
    let matches = parse_rg_json(&output.stdout, max_results);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let workspace_root = default_workspace_root();
    let audit = AuditLog::new(workspace_root.join(".taurihands").join("audit.log"));
    let terminal = TerminalManager::new(workspace_root.join(".taurihands").join("terminal"));
    let workspace = WorkspaceState::new(workspace_root);
    let agent = AgentManager::new();
    let kernel = KernelManager::new(
        workspace.root(),
        terminal.clone(),
        workspace.clone(),
        audit.clone(),
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
            kernel_user_input,
            kernel_plan_update,
            kernel_plan_status,
            llm_get_profile,
            llm_save_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
