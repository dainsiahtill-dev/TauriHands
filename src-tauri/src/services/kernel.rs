use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::services::audit::now_ms;
use crate::services::audit::AuditLog;
use crate::services::judge::{JudgeContext, JudgeEngine, JudgeRule};
use crate::services::llm::{request_completion, request_completion_stream, LlmProfile, LlmStore};
use crate::services::pty::{TerminalExecRequest, TerminalManager};
use crate::services::tool_dispatcher::ToolDispatcher as ToolDispatcherTrait;
use crate::services::tools::{
    max_read_bytes, read_file, run_command, search, write_file, CommandRequest, ReadFileRequest,
    SearchMatch, SearchRequest, ToolResult, WriteFileRequest,
};
use crate::services::workspace::{display_path, resolve_read_path_with_fallback, WorkspaceState};

const KERNEL_EVENT_NAME: &str = "kernel-event";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelEvent {
    pub id: String,
    pub run_id: String,
    pub ts: u128,
    pub seq: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Clone)]
struct EventBus {
    base_dir: Arc<Mutex<PathBuf>>,
    run_id: Arc<Mutex<String>>,
    seq: Arc<AtomicU64>,
}

impl EventBus {
    fn new(base_dir: PathBuf, run_id: String) -> Self {
        Self {
            base_dir: Arc::new(Mutex::new(base_dir)),
            run_id: Arc::new(Mutex::new(run_id)),
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    fn set_run(&self, run_id: String) {
        if let Ok(mut current) = self.run_id.lock() {
            *current = run_id;
        }
        self.seq.store(0, Ordering::SeqCst);
    }

    fn set_base_dir(&self, base_dir: PathBuf) {
        if let Ok(mut current) = self.base_dir.lock() {
            *current = base_dir;
        }
    }

    fn emit<T: Serialize>(&self, app: &AppHandle, event_type: &str, payload: &T) -> KernelEvent {
        let run_id = self
            .run_id
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "default".to_string());
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let event = KernelEvent {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            ts: now_ms(),
            seq,
            event_type: event_type.to_string(),
            payload: serde_json::to_value(payload).unwrap_or_else(|_| serde_json::json!({})),
        };
        self.append_event(&event);
        let _ = app.emit(KERNEL_EVENT_NAME, event.clone());
        event
    }

    fn append_event(&self, event: &KernelEvent) {
        let path = self.log_path(&event.run_id);
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            if let Ok(line) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    fn log_path(&self, run_id: &str) -> PathBuf {
        let base_dir = self
            .base_dir
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| PathBuf::from("."));
        base_dir.join(format!("{}.jsonl", run_id))
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunAgentState {
    Idle,
    Running,
    Paused,
    AwaitingUser,
    Error,
    Finished,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub run_id: String,
    pub agent_state: RunAgentState,
    pub turn: u32,
    pub messages: Vec<ChatMessage>,
    pub tool_context: ToolContext,
    #[serde(default)]
    pub task_id: Option<String>,
    pub plan: Option<Plan>,
    pub tasks: Option<TaskList>,
    pub budget: Budget,
    pub recent_observations: Vec<String>,
    pub auto_run: bool,
    pub last_error: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolContext {
    pub cwd: String,
    pub env: HashMap<String, String>,
    pub session_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    pub max_steps: u32,
    pub used_steps: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub version: u32,
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub status: String,
    pub done: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskList {
    pub version: u32,
    pub items: Vec<Task>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: String,
    pub notes: Option<String>,
}

impl RunState {
    fn new(run_id: String, cwd: String) -> Self {
        Self {
            run_id,
            agent_state: RunAgentState::Idle,
            turn: 0,
            messages: Vec::new(),
            tool_context: ToolContext {
                cwd,
                env: HashMap::new(),
                session_id: None,
            },
            task_id: None,
            plan: None,
            tasks: None,
            budget: Budget {
                max_steps: 8,
                used_steps: 0,
            },
            recent_observations: Vec::new(),
            auto_run: true,
            last_error: None,
        }
    }
}

fn is_stop_command(input: &str) -> bool {
    let normalized = input.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "stop" | "cancel" | "abort" | "停止" | "停止任务" | "取消"
    )
}

fn is_continue_command(input: &str) -> bool {
    let normalized = input.trim().to_lowercase();
    matches!(normalized.as_str(), "continue" | "继续" | "继续吧" | "go on" | "继续执行")
}

fn infer_default_continue_reply(state: &RunState) -> Option<String> {
    let last_assistant = state
        .messages
        .iter()
        .rev()
        .find(|msg| msg.role == "assistant")?;
    let text = last_assistant.content.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    if lower.contains("user input required") || lower.contains("prompt:") {
        return None;
    }
    let mentions_choice = lower.contains("还是")
        || lower.contains("选择")
        || lower.contains("确认")
        || lower.contains("example")
        || lower.contains("示例")
        || lower.contains("只看")
        || lower.contains("仅")
        || lower.contains("workspace")
        || lower.contains("本地")
        || lower.contains("创建")
        || lower.contains("修改");
    if !mentions_choice {
        return None;
    }
    Some("请在当前工作区实际创建/修改文件并继续执行。".to_string())
}


#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "terminal.exec")]
    TerminalExec {
        id: String,
        cmd: String,
        cwd: Option<String>,
    },
    #[serde(rename = "terminal.run")]
    TerminalRun {
        id: String,
        program: String,
        args: Vec<String>,
        cwd: Option<String>,
    },
    #[serde(rename = "fs.read")]
    FsRead { id: String, path: String },
    #[serde(rename = "fs.write")]
    FsWrite {
        id: String,
        path: String,
        content: String,
    },
    #[serde(rename = "fs.search")]
    FsSearch {
        id: String,
        pattern: String,
        paths: Option<Vec<String>>,
    },
    #[serde(rename = "git.status")]
    GitStatus { id: String },
    #[serde(rename = "git.diff")]
    GitDiff { id: String, path: Option<String> },
    #[serde(rename = "tests.run")]
    TestsRun {
        id: String,
        program: String,
        args: Vec<String>,
    },
    #[serde(rename = "plan.update")]
    PlanUpdate { id: String, plan: Plan },
    #[serde(rename = "task.update")]
    TaskUpdate { id: String, tasks: TaskList },
    #[serde(rename = "user.ask")]
    UserAsk { id: String, question: String },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub ok: bool,
    pub summary: String,
    pub exit_code: Option<i32>,
    pub artifacts: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
    #[serde(default)]
    pub requires_user: bool,
}

struct LlmDecision {
    message: Option<String>,
    actions: Vec<Action>,
}

#[derive(Clone)]
struct Runtime {
    terminal: TerminalManager,
    workspace: WorkspaceState,
    audit: AuditLog,
}

impl Runtime {
    fn new(terminal: TerminalManager, workspace: WorkspaceState, audit: AuditLog) -> Self {
        Self {
            terminal,
            workspace,
            audit,
        }
    }

    fn execute(
        &self,
        action: &Action,
        session_id: Option<String>,
        on_chunk: &mut dyn FnMut(String),
    ) -> Result<Observation, String> {
        let result = match action {
            Action::TerminalExec { cmd, cwd, .. } => {
                let request = TerminalExecRequest {
                    command: cmd.clone(),
                    session_id,
                    shell: None,
                    cwd: cwd.clone(),
                    cols: None,
                    rows: None,
                    timeout_ms: Some(15_000),
                    max_bytes: Some(24_000),
                };
                let resolved_cwd = match cwd {
                    Some(path) => self.workspace.resolve_path(path)?,
                    None => self.workspace.root(),
                };
                self.terminal.exec_interactive(request, resolved_cwd, &self.audit)
            }
            Action::TerminalRun {
                program,
                args,
                cwd,
                ..
            } => {
                let resolved_cwd = match cwd {
                    Some(path) => self.workspace.resolve_path(path)?,
                    None => self.workspace.root(),
                };
                run_command(
                    CommandRequest {
                        program: program.clone(),
                        args: Some(args.clone()),
                        cwd: Some(resolved_cwd.to_string_lossy().to_string()),
                        env: None,
                        timeout_ms: None,
                    },
                    resolved_cwd.to_string_lossy().as_ref(),
                    &self.audit,
                )
            }
            Action::FsRead { path, .. } => read_file_tool(&self.workspace, &self.audit, path),
            Action::FsSearch { pattern, paths, .. } => {
                search_tool(&self.workspace, &self.audit, pattern, paths)
            }
            Action::TestsRun {
                program, args, ..
            } => {
                let cwd = self.workspace.root();
                run_command(
                    CommandRequest {
                        program: program.clone(),
                        args: Some(args.clone()),
                        cwd: Some(cwd.to_string_lossy().to_string()),
                        env: None,
                        timeout_ms: Some(120_000),
                    },
                    cwd.to_string_lossy().as_ref(),
                    &self.audit,
                )
            }
            Action::GitStatus { .. } => {
                let cwd = self.workspace.root();
                run_command(
                    CommandRequest {
                        program: "git".to_string(),
                        args: Some(vec![
                            "status".to_string(),
                            "--porcelain=v1".to_string(),
                            "--untracked-files=all".to_string(),
                        ]),
                        cwd: Some(cwd.to_string_lossy().to_string()),
                        env: None,
                        timeout_ms: None,
                    },
                    cwd.to_string_lossy().as_ref(),
                    &self.audit,
                )
            }
            Action::GitDiff { path, .. } => {
                let cwd = self.workspace.root();
                let mut args = vec!["diff".to_string()];
                if let Some(path) = path {
                    let resolved = self.workspace.resolve_path(path)?;
                    args.push("--".to_string());
                    args.push(resolved.to_string_lossy().to_string());
                }
                run_command(
                    CommandRequest {
                        program: "git".to_string(),
                        args: Some(args),
                        cwd: Some(cwd.to_string_lossy().to_string()),
                        env: None,
                        timeout_ms: None,
                    },
                    cwd.to_string_lossy().as_ref(),
                    &self.audit,
                )
            }
            Action::FsWrite { path, content, .. } => {
                let resolved = self.workspace.resolve_path_for_write(path)?;
                if let Some(parent) = resolved.parent() {
                    create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&resolved, content.as_bytes()).map_err(|e| e.to_string())?;
                let request = WriteFileRequest {
                    path: path.clone(),
                    content: content.clone(),
                };
                Ok(write_file(request, content.len(), &self.audit))
            }
            Action::PlanUpdate { .. }
            | Action::TaskUpdate { .. }
            | Action::UserAsk { .. } => {
                return Ok(Observation {
                    ok: true,
                    summary: "State update".to_string(),
                    exit_code: None,
                    artifacts: None,
                    raw: None,
                    requires_user: false,
                });
            }
        }?;
        let observation = tool_result_to_observation(result, on_chunk);
        Ok(observation)
    }

    fn dispatch(
        &self,
        action: &Action,
        session_id: Option<String>,
        on_chunk: &mut dyn FnMut(String),
    ) -> Result<Observation, String> {
        self.execute(action, session_id, on_chunk)
    }
}

impl ToolDispatcherTrait for Runtime {
    fn dispatch(
        &self,
        action: &Action,
        session_id: Option<String>,
        on_chunk: &mut dyn FnMut(String),
    ) -> Result<Observation, String> {
        self.execute(action, session_id, on_chunk)
    }
}

#[derive(Clone)]
struct StateStore {
    base_dir: PathBuf,
}

impl StateStore {
    fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn set_base_dir(&mut self, base_dir: PathBuf) {
        self.base_dir = base_dir;
    }

    fn save(&self, state: &RunState) -> Result<(), String> {
        let path = self.base_dir.join(format!("{}.json", state.run_id));
        if let Some(parent) = path.parent() {
            create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?;
        std::fs::write(path, data).map_err(|e| e.to_string())
    }
}

#[derive(Clone)]
pub struct KernelManager {
    state: Arc<Mutex<RunState>>,
    runtime: Runtime,
    store: Arc<Mutex<StateStore>>,
    events: EventBus,
    llm: LlmStore,
    judge: Arc<Mutex<JudgeEngine>>,
    paused: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

#[derive(Deserialize)]
pub struct KernelStartRequest {
    pub session_id: Option<String>,
    pub max_steps: Option<u32>,
    pub task_id: Option<String>,
}

#[derive(Deserialize)]
pub struct KernelUserInputRequest {
    pub content: String,
}

#[derive(Deserialize)]
pub struct KernelPlanUpdateRequest {
    pub goal: String,
    pub steps: Vec<String>,
    pub auto_generate: Option<bool>,
}

#[derive(Deserialize)]
pub struct KernelPlanStatusRequest {
    pub id: String,
    pub status: String,
}

impl KernelManager {
    pub fn new(
        workspace_root: PathBuf,
        terminal: TerminalManager,
        workspace: WorkspaceState,
        audit: AuditLog,
        llm_root: PathBuf,
    ) -> Self {
        let run_id = "default".to_string();
        let state = RunState::new(run_id.clone(), display_path(&workspace_root));
        let events = EventBus::new(
            workspace_root.join(".taurihands").join("events"),
            run_id,
        );
        let store = StateStore::new(workspace_root.join(".taurihands").join("runs"));
        let llm = LlmStore::new(llm_root);
        Self {
            state: Arc::new(Mutex::new(state)),
            runtime: Runtime::new(terminal, workspace, audit),
            store: Arc::new(Mutex::new(store)),
            events,
            llm,
            judge: Arc::new(Mutex::new(JudgeEngine::new())),
            paused: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn update_workspace_root(&self, root: PathBuf) {
        if let Ok(mut store) = self.store.lock() {
            store.set_base_dir(root.join(".taurihands").join("runs"));
        }
        self.events.set_base_dir(root.join(".taurihands").join("events"));
        let _ = self.update_state(|state| {
            state.tool_context.cwd = display_path(&root);
        });
    }

    pub fn get_llm_profile(&self) -> Option<LlmProfile> {
        self.llm.get_active_profile()
    }

    pub fn save_llm_profile(&self, profile: LlmProfile) -> Result<LlmProfile, String> {
        self.llm.save_profile(profile.clone())?;
        Ok(self.llm.get_active_profile().unwrap_or(profile))
    }

    pub fn set_task_id(&self, task_id: Option<String>) -> Result<RunState, String> {
        let snapshot = self.update_state(|state| {
            state.task_id = task_id.clone();
        })?;
        Ok(snapshot)
    }

    pub fn set_judge_rules(&self, rules: Vec<JudgeRule>) -> Result<(), String> {
        let mut judge = self
            .judge
            .lock()
            .map_err(|_| "Judge lock poisoned".to_string())?;
        judge.set_rules(rules);
        Ok(())
    }

    pub fn get_judge_rules(&self) -> Result<Vec<JudgeRule>, String> {
        let judge = self
            .judge
            .lock()
            .map_err(|_| "Judge lock poisoned".to_string())?;
        Ok(judge.rules().to_vec())
    }

    pub fn snapshot(&self) -> RunState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| RunState::new("default".to_string(), "".to_string()))
    }

    pub fn start(&self, app: AppHandle, request: KernelStartRequest) -> Result<RunState, String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Kernel already running".to_string());
        }
        let run_id = Uuid::new_v4().to_string();
        self.events.set_run(run_id.clone());
        let snapshot = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Kernel state lock poisoned".to_string())?;
            let cwd = display_path(&self.runtime.workspace.root());
            let existing_plan = state.plan.clone();
            let existing_tasks = state.tasks.clone();
            let existing_messages = state.messages.clone();
            let existing_turn = state.turn;
            let existing_task_id = state.task_id.clone();
            *state = RunState::new(run_id.clone(), cwd);
            state.plan = existing_plan;
            state.tasks = existing_tasks;
            state.messages = existing_messages;
            state.turn = existing_turn;
            state.task_id = request.task_id.clone().or(existing_task_id);
            state.agent_state = RunAgentState::Running;
            state.tool_context.session_id = request.session_id.clone();
            if let Some(max_steps) = request.max_steps {
                state.budget.max_steps = max_steps;
            }
            state.clone()
        };
        self.emit_state(&app, "start");
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            manager.run_loop(app).await;
        });
        Ok(snapshot)
    }

    pub fn pause(&self, app: &AppHandle) -> Result<RunState, String> {
        self.paused.store(true, Ordering::SeqCst);
        let snapshot = self.update_state(|state| {
            if state.agent_state == RunAgentState::Running {
                state.agent_state = RunAgentState::Paused;
            }
        })?;
        self.emit_state(app, "pause");
        Ok(snapshot)
    }

    pub fn resume(&self, app: &AppHandle) -> Result<RunState, String> {
        self.paused.store(false, Ordering::SeqCst);
        let mut should_spawn = false;
        let snapshot = self.update_state(|state| {
            if state.agent_state == RunAgentState::Paused {
                state.agent_state = RunAgentState::Running;
                should_spawn = true;
            }
        })?;
        self.emit_state(app, "resume");
        if should_spawn && !self.running.swap(true, Ordering::SeqCst) {
            let manager = self.clone();
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                manager.run_loop(app_handle).await;
            });
        }
        Ok(snapshot)
    }

    pub fn stop(&self, app: &AppHandle) -> Result<RunState, String> {
        self.paused.store(false, Ordering::SeqCst);
        let snapshot = self.update_state(|state| {
            if state.agent_state != RunAgentState::Idle {
                state.agent_state = RunAgentState::Finished;
            }
        })?;
        self.emit_state(app, "stop");
        Ok(snapshot)
    }

    pub fn continue_run(&self, app: &AppHandle) -> Result<RunState, String> {
        self.paused.store(false, Ordering::SeqCst);
        let mut should_spawn = false;
        let snapshot = self.update_state(|state| {
            if state.agent_state == RunAgentState::AwaitingUser {
                state.agent_state = RunAgentState::Running;
                state.budget.used_steps = 0;
                state.last_error = None;
                should_spawn = true;
            }
        })?;
        self.emit_state(app, "continue");
        if should_spawn && !self.running.swap(true, Ordering::SeqCst) {
            let manager = self.clone();
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                manager.run_loop(app_handle).await;
            });
        }
        Ok(snapshot)
    }

    pub fn user_input(
        &self,
        app: &AppHandle,
        request: KernelUserInputRequest,
    ) -> Result<RunState, String> {
        let content = request.content.trim();
        if content.is_empty() {
            return Err("User input cannot be empty".to_string());
        }
        let mut final_content = content.to_string();
        let stop_command = is_stop_command(content);
        let mut control_stop = false;
        let mut should_spawn = false;
        let snapshot = self.update_state(|state| {
            if stop_command {
                control_stop = true;
                state.agent_state = RunAgentState::Finished;
                state.last_error = None;
                return;
            }
            if state.agent_state == RunAgentState::AwaitingUser {
                if is_continue_command(content) {
                    if let Some(reply) = infer_default_continue_reply(state) {
                        final_content = reply;
                    }
                }
                state.budget.used_steps = 0;
                state.last_error = None;
            }
            state.messages.push(ChatMessage {
                role: "user".to_string(),
                content: final_content.clone(),
            });
            state.turn = state.turn.saturating_add(1);
            if state.agent_state != RunAgentState::Running {
                state.agent_state = RunAgentState::Running;
            }
            should_spawn = true;
        })?;
        self.events.emit(
            app,
            "UserMessage",
            &serde_json::json!({ "content": final_content }),
        );
        if control_stop {
            self.paused.store(false, Ordering::SeqCst);
            self.emit_state(app, "user_stop");
            return Ok(snapshot);
        }
        self.emit_state(app, "user_input");
        if should_spawn && !self.running.swap(true, Ordering::SeqCst) {
            let manager = self.clone();
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                manager.run_loop(app_handle).await;
            });
        }
        Ok(snapshot)
    }

    pub async fn update_plan(
        &self,
        app: &AppHandle,
        request: KernelPlanUpdateRequest,
    ) -> Result<RunState, String> {
        let goal = request.goal.trim().to_string();
        if goal.is_empty() {
            return Err("Plan goal cannot be empty".to_string());
        }
        if request.auto_generate.unwrap_or(false) {
            let plan = self.generate_plan_from_llm(&goal).await?;
            return self.apply_plan(app, plan, "PlanUpdated");
        }
        let steps = request
            .steps
            .into_iter()
            .filter(|step| !step.trim().is_empty())
            .map(|step| PlanStep {
                id: make_id("plan"),
                title: step.trim().to_string(),
                status: "pending".to_string(),
                done: false,
            })
            .collect::<Vec<_>>();
        if steps.is_empty() {
            return Err("Plan steps cannot be empty".to_string());
        }
        let plan = Plan {
            version: 1,
            goal,
            steps,
        };
        self.apply_plan(app, plan, "PlanUpdated")
    }

    pub fn update_plan_status(
        &self,
        app: &AppHandle,
        request: KernelPlanStatusRequest,
    ) -> Result<RunState, String> {
        let status = request.status.trim().to_string();
        let snapshot = self.update_state(|state| {
            if let Some(plan) = &mut state.plan {
                if let Some(step) = plan.steps.iter_mut().find(|item| item.id == request.id) {
                    step.status = status.clone();
                    step.done = step.status == "done" || step.status == "skipped";
                }
            }
            if let Some(tasks) = &mut state.tasks {
                if let Some(task) = tasks.items.iter_mut().find(|item| item.id == request.id) {
                    task.status = status.clone();
                }
            }
        })?;
        self.events.emit(
            app,
            "PlanUpdated",
            &serde_json::json!({ "plan": snapshot.plan }),
        );
        self.emit_state(app, "plan_status");
        Ok(snapshot)
    }

    async fn generate_plan_from_llm(&self, goal: &str) -> Result<Plan, String> {
        let profile = self.llm.get_active_profile().ok_or_else(|| {
            "LLM profile not configured. Save a profile in LLM Settings.".to_string()
        })?;
        let system_prompt = build_plan_system_prompt(&profile);
        let user_prompt = format!(
            "Goal: {}\nReturn JSON only. Format: {{\"goal\":\"...\",\"steps\":[\"step 1\",\"step 2\"]}} or [\"step 1\",\"step 2\"].",
            goal
        );
        let raw = request_completion(&profile, &system_prompt, &user_prompt).await?;
        parse_plan_response(&raw, Some(goal))
    }

    fn apply_plan(&self, app: &AppHandle, plan: Plan, event_type: &str) -> Result<RunState, String> {
        let snapshot = self.update_state(|state| {
            state.plan = Some(plan.clone());
            state.tasks = Some(TaskList {
                version: 1,
                items: plan
                    .steps
                    .iter()
                    .map(|step| Task {
                        id: step.id.clone(),
                        title: step.title.clone(),
                        status: "todo".to_string(),
                        notes: None,
                    })
                    .collect(),
            });
        })?;
        self.events.emit(app, event_type, &serde_json::json!({ "plan": plan }));
        self.emit_state(app, "plan_update");
        if let Some(task_id) = snapshot.task_id.clone() {
            let _ = self.save_plan_for_task(&task_id, &plan);
        }
        Ok(snapshot)
    }

    fn save_plan_for_task(&self, task_id: &str, plan: &Plan) -> Result<(), String> {
        let base = self
            .runtime
            .workspace
            .root()
            .join(".taurihands")
            .join("tasks")
            .join(task_id);
        create_dir_all(&base).map_err(|e| e.to_string())?;
        let path = base.join("plan.json");
        let data = serde_json::to_vec_pretty(plan).map_err(|e| e.to_string())?;
        std::fs::write(path, data).map_err(|e| e.to_string())
    }

    fn emit_state(&self, app: &AppHandle, reason: &str) {
        let snapshot = self.snapshot();
        let payload = serde_json::json!({
            "reason": reason,
            "state": snapshot,
        });
        self.events.emit(app, "StateChanged", &payload);
        if let Ok(store) = self.store.lock() {
            let _ = store.save(&snapshot);
        }
    }

    fn update_state<F>(&self, updater: F) -> Result<RunState, String>
    where
        F: FnOnce(&mut RunState),
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Kernel state lock poisoned".to_string())?;
        updater(&mut state);
        Ok(state.clone())
    }

    async fn run_loop(&self, app: AppHandle) {
        'run: loop {
            if self.paused.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(300));
                continue;
            }
            let snapshot = match self.snapshot_agent_state() {
                Ok(state) => state,
                Err(err) => {
                    self.events.emit(&app, "Error", &serde_json::json!({ "message": err }));
                    break;
                }
            };
            if snapshot.agent_state != RunAgentState::Running {
                break;
            }
            if snapshot.budget.used_steps >= snapshot.budget.max_steps {
                let notice = format!(
                    "Step budget reached ({} steps). Reply \"continue\" to proceed or \"stop\" to end.",
                    snapshot.budget.max_steps
                );
                let _ = self.update_state(|state| {
                    state.agent_state = RunAgentState::AwaitingUser;
                    state.last_error = None;
                    state.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: notice.clone(),
                    });
                });
                self.events.emit(
                    &app,
                    "AgentMessage",
                    &serde_json::json!({ "content": notice }),
                );
                self.emit_state(&app, "step_budget");
                break;
            }
            let decision = match self.decide_actions_with_llm(&app, &snapshot).await {
                Ok(decision) => decision,
                Err(err) => {
                    let _ = self.update_state(|state| {
                        state.agent_state = RunAgentState::Error;
                        state.last_error = Some(err.clone());
                        state.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: err.clone(),
                        });
                    });
                    self.events.emit(&app, "Error", &serde_json::json!({ "message": err }));
                    self.emit_state(&app, "agent_error");
                    break;
                }
            };
            let actions = decision.actions;
            let mut message = decision.message;
            if message.is_none() {
                for action in &actions {
                    if let Action::UserAsk { question, .. } = action {
                        message = Some(question.clone());
                        break;
                    }
                }
            }
            if let Some(message) = &message {
                let _ = self.update_state(|state| {
                    state.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: message.clone(),
                    });
                });
                self.events.emit(
                    &app,
                    "AgentMessage",
                    &serde_json::json!({ "content": message }),
                );
                self.emit_state(&app, "assistant_message");
            }
            if actions.is_empty() {
                if message.is_some() {
                    let _ = self.update_state(|state| {
                        state.agent_state = RunAgentState::Finished;
                    });
                    self.emit_state(&app, "finished");
                    break;
                }
                let message = "LLM returned no actions".to_string();
                let _ = self.update_state(|state| {
                    state.agent_state = RunAgentState::AwaitingUser;
                    state.last_error = Some(message.clone());
                    state.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: message.clone(),
                    });
                });
                self.events.emit(&app, "Error", &serde_json::json!({ "message": message }));
                self.emit_state(&app, "awaiting_user");
                break;
            }
            self.events.emit(
                &app,
                "AgentActionProposed",
                &serde_json::json!({ "actions": actions }),
            );
            for action in actions {
                let current_state = match self.snapshot_agent_state() {
                    Ok(state) => state,
                    Err(err) => {
                        self.events.emit(&app, "Error", &serde_json::json!({ "message": err }));
                        break 'run;
                    }
                };
                if current_state.agent_state != RunAgentState::Running {
                    break 'run;
                }
                if matches!(action, Action::UserAsk { .. }) {
                    let _ = self.update_state(|state| {
                        state.agent_state = RunAgentState::AwaitingUser;
                    });
                    self.events.emit(
                        &app,
                        "AgentActionProposed",
                        &serde_json::json!({ "action": action }),
                    );
                    self.emit_state(&app, "awaiting_user");
                    break 'run;
                }

                self.events.emit(
                    &app,
                    "ToolCallStarted",
                    &serde_json::json!({ "action": action }),
                );
                let mut chunk_handler = |chunk: String| {
                    let _ = self.events.emit(
                        &app,
                        "ToolCallChunk",
                        &serde_json::json!({ "action_id": action_id(&action), "chunk": chunk }),
                    );
                };
                let observation = match self.runtime.dispatch(
                    &action,
                    snapshot.tool_context.session_id.clone(),
                    &mut chunk_handler,
                ) {
                    Ok(obs) => obs,
                    Err(err) => {
                        let message = if err.trim().is_empty() {
                            "Runtime error".to_string()
                        } else {
                            err.clone()
                        };
                        let _ = self.update_state(|state| {
                            state.agent_state = RunAgentState::Error;
                            state.last_error = Some(message.clone());
                        });
                        self.events.emit(
                            &app,
                            "ToolCallFinished",
                            &serde_json::json!({
                                "action": action,
                                "summary": message,
                                "ok": false,
                                "exit_code": serde_json::Value::Null,
                            }),
                        );
                        self.events
                            .emit(&app, "Error", &serde_json::json!({ "message": message }));
                        self.emit_state(&app, "runtime_error");
                        break 'run;
                    }
                };
                self.events.emit(
                    &app,
                    "ToolCallFinished",
                    &serde_json::json!({
                        "action": action,
                        "summary": observation.summary,
                        "ok": observation.ok,
                        "exit_code": observation.exit_code,
                    }),
                );
                self.events
                    .emit(&app, "Observation", &serde_json::json!({ "observation": observation }));
                let _ = self.apply_observation(&app, &action, &observation);
                if observation.requires_user {
                    self.emit_state(&app, "awaiting_user");
                    break 'run;
                }
            }
            let _ = self.update_state(|state| {
                state.budget.used_steps = state.budget.used_steps.saturating_add(1);
            });
            self.emit_state(&app, "step_complete");
            if let Ok(snapshot) = self.snapshot_agent_state() {
                let context = JudgeContext {
                    iteration: snapshot.budget.used_steps,
                    last_error: snapshot.last_error.clone(),
                };
                if let Ok(judge) = self.judge.lock() {
                    let result = judge.evaluate(&context);
                    self.events.emit(
                        &app,
                        "JudgeResult",
                        &serde_json::json!({ "result": result }),
                    );
                }
            }
        }
        self.running.store(false, Ordering::SeqCst);
    }

    fn snapshot_agent_state(&self) -> Result<RunState, String> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| "Kernel state lock poisoned".to_string())
    }

    async fn decide_actions_with_llm(
        &self,
        app: &AppHandle,
        state: &RunState,
    ) -> Result<LlmDecision, String> {
        let profile = self.llm.get_active_profile().ok_or_else(|| {
            "LLM profile not configured. Save a profile in LLM Settings.".to_string()
        })?;
        let allowed = build_allowed_action_set(&profile);
        let system_prompt = build_system_prompt(&profile, &allowed);
        let user_prompt = build_user_prompt(state);
        let events = self.events.clone();
        let app_handle = app.clone();
        let raw = request_completion_stream(&profile, &system_prompt, &user_prompt, |chunk| {
            if !chunk.trim().is_empty() {
                events.emit(
                    &app_handle,
                    "AgentMessageChunk",
                    &serde_json::json!({ "content": chunk }),
                );
            }
        })
        .await?;
        events.emit(&app_handle, "AgentMessageDone", &serde_json::json!({}));
        let goal_hint = state
            .plan
            .as_ref()
            .map(|plan| plan.goal.as_str())
            .or_else(|| state.messages.last().map(|msg| msg.content.as_str()));
        let mut decision = parse_llm_response(&raw, goal_hint)?;
        decision.actions.retain(|action| action_allowed(action, &allowed));
        Ok(decision)
    }

    fn apply_observation(
        &self,
        app: &AppHandle,
        action: &Action,
        observation: &Observation,
    ) -> Result<(), String> {
        let snapshot = self.update_state(|state| {
            if let Action::PlanUpdate { plan, .. } = action {
                state.plan = Some(plan.clone());
            }
            if let Action::TaskUpdate { tasks, .. } = action {
                state.tasks = Some(tasks.clone());
            }
            let summary = trim_to(&observation.summary, 2000);
            if !summary.is_empty() {
                state
                    .recent_observations
                    .push(format!("{}: {}", action_id(action), summary));
                if state.recent_observations.len() > 6 {
                    state.recent_observations.remove(0);
                }
            }
            if observation.requires_user {
                state.agent_state = RunAgentState::AwaitingUser;
                state.last_error = Some(observation.summary.clone());
                return;
            }
            if observation.ok {
                if let Some(plan) = &mut state.plan {
                    if let Some(step) =
                        plan.steps.iter_mut().find(|step| step.id == action_id(action))
                    {
                        step.status = "done".to_string();
                        step.done = true;
                    }
                }
                if let Some(tasks) = &mut state.tasks {
                    if let Some(task) =
                        tasks.items.iter_mut().find(|item| item.id == action_id(action))
                    {
                        task.status = "done".to_string();
                    }
                }
            } else {
                state.agent_state = RunAgentState::Error;
                state.last_error = Some(observation.summary.clone());
            }
        })?;
        if matches!(action, Action::PlanUpdate { .. }) {
            self.events.emit(
                app,
                "PlanUpdated",
                &serde_json::json!({ "plan": snapshot.plan }),
            );
        }
        if matches!(action, Action::TaskUpdate { .. }) {
            self.events.emit(
                app,
                "TaskUpdated",
                &serde_json::json!({ "tasks": snapshot.tasks }),
            );
        }
        Ok(())
    }
}

fn action_id(action: &Action) -> String {
    match action {
        Action::TerminalExec { id, .. }
        | Action::TerminalRun { id, .. }
        | Action::FsRead { id, .. }
        | Action::FsWrite { id, .. }
        | Action::FsSearch { id, .. }
        | Action::GitStatus { id, .. }
        | Action::GitDiff { id, .. }
        | Action::TestsRun { id, .. }
        | Action::PlanUpdate { id, .. }
        | Action::TaskUpdate { id, .. }
        | Action::UserAsk { id, .. } => id.clone(),
    }
}

fn tool_result_to_observation(result: ToolResult, on_chunk: &mut dyn FnMut(String)) -> Observation {
    let mut summary = String::new();
    if let Some(stdout) = &result.stdout_excerpt {
        if !stdout.trim().is_empty() {
            summary.push_str(stdout.trim());
        }
    }
    if let Some(stderr) = &result.stderr_excerpt {
        if !stderr.trim().is_empty() {
            if !summary.is_empty() {
                summary.push('\n');
            }
            summary.push_str(stderr.trim());
        }
    }
    if summary.is_empty() {
        summary = if result.ok {
            "ok".to_string()
        } else {
            "error".to_string()
        };
    }
    on_chunk(summary.clone());
    Observation {
        ok: result.ok,
        summary,
        exit_code: result.exit_code,
        artifacts: result.artifacts,
        raw: None,
        requires_user: result.requires_user,
    }
}

fn read_file_tool(
    workspace: &WorkspaceState,
    audit: &AuditLog,
    path: &str,
) -> Result<ToolResult, String> {
    let request = ReadFileRequest {
        path: path.to_string(),
    };
    let resolved = resolve_read_path_with_fallback(workspace, &request.path)?;
    let max_bytes = max_read_bytes();
    let file = std::fs::File::open(&resolved).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    let mut handle = file.take(max_bytes as u64);
    std::io::Read::read_to_end(&mut handle, &mut buffer).map_err(|e| e.to_string())?;
    let truncated = metadata.len() as usize > buffer.len();
    let content = String::from_utf8_lossy(&buffer).to_string();
    Ok(read_file(request, content, truncated, audit))
}

fn search_tool(
    workspace: &WorkspaceState,
    audit: &AuditLog,
    pattern: &str,
    paths: &Option<Vec<String>>,
) -> Result<ToolResult, String> {
    let (resolved_paths, globs) = resolve_search_targets(workspace, paths);
    let trimmed = pattern.trim();
    if trimmed == "*" {
        let output = run_rg_files(&resolved_paths, &globs)?;
        let matches = parse_rg_files(&output, 200);
        return Ok(search(
            SearchRequest {
                pattern: pattern.to_string(),
                paths: paths.clone(),
                glob: None,
                max_results: Some(200),
            },
            matches,
            audit,
        ));
    }
    let (normalized, force_fixed) = normalize_search_pattern(trimmed);
    let output = run_rg_search(&normalized, &resolved_paths, &globs, force_fixed)?;
    let matches = parse_rg_json(&output, 200);
    Ok(search(
        SearchRequest {
            pattern: pattern.to_string(),
            paths: paths.clone(),
            glob: None,
            max_results: Some(200),
        },
        matches,
        audit,
    ))
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
    let mut cmd = std::process::Command::new("rg");
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
    let mut cmd = std::process::Command::new("rg");
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

fn build_allowed_action_set(profile: &LlmProfile) -> Option<HashSet<String>> {
    if profile.tool_toggles.is_empty() {
        return None;
    }
    let mut allowed = HashSet::new();
    for toggle in &profile.tool_toggles {
        if !toggle.enabled {
            continue;
        }
        if let Some(action) = map_tool_toggle_to_action(&toggle.id) {
            allowed.insert(action.to_string());
        }
    }
    if allowed.is_empty() {
        None
    } else {
        Some(allowed)
    }
}

fn map_tool_toggle_to_action(toggle_id: &str) -> Option<&'static str> {
    match toggle_id {
        "terminal.exec_interactive" | "terminal.exec" => Some("terminal.exec"),
        "terminal.run_command" | "terminal.run" => Some("terminal.run"),
        "fs.read_file" | "fs.read" => Some("fs.read"),
        "fs.write_file" | "fs.write" => Some("fs.write"),
        "fs.apply_patch" => Some("fs.write"),
        "fs.search" => Some("fs.search"),
        "git.status" => Some("git.status"),
        "git.diff" => Some("git.diff"),
        "tests.run" => Some("tests.run"),
        _ => None,
    }
}

fn action_allowed(action: &Action, allowed: &Option<HashSet<String>>) -> bool {
    match action {
        Action::PlanUpdate { .. } | Action::TaskUpdate { .. } | Action::UserAsk { .. } => true,
        _ => match allowed {
            Some(allowed) => allowed.contains(action_type(action)),
            None => true,
        },
    }
}

fn action_type(action: &Action) -> &'static str {
    match action {
        Action::TerminalExec { .. } => "terminal.exec",
        Action::TerminalRun { .. } => "terminal.run",
        Action::FsRead { .. } => "fs.read",
        Action::FsWrite { .. } => "fs.write",
        Action::FsSearch { .. } => "fs.search",
        Action::GitStatus { .. } => "git.status",
        Action::GitDiff { .. } => "git.diff",
        Action::TestsRun { .. } => "tests.run",
        Action::PlanUpdate { .. } => "plan.update",
        Action::TaskUpdate { .. } => "task.update",
        Action::UserAsk { .. } => "user.ask",
    }
}

fn build_system_prompt(profile: &LlmProfile, allowed: &Option<HashSet<String>>) -> String {
    let mut prompt = String::new();
    let base = profile.prompt.trim();
    if !base.is_empty() {
        prompt.push_str(base);
        prompt.push_str("\n\n");
    }
    let allowed_list = allowed_action_list(allowed);
    prompt.push_str("You are the TauriHands kernel agent.\n");
    prompt.push_str("Respond with strict JSON only. Do not wrap in markdown.\n");
    prompt.push_str("If the user asks to run a command or list files, you must include a tool action.\n");
    prompt.push_str("Do not reply with only text when a tool action is required.\n");
    if !allowed_list.is_empty() {
        prompt.push_str(&format!(
            "Allowed actions: {}.\n",
            allowed_list.join(", ")
        ));
    }
    prompt.push_str("Default behavior: apply changes directly in the current workspace.\n");
    prompt.push_str("Do not ask whether to show sample code vs create files; proceed with workspace changes unless the user explicitly asks for sample-only.\n");
    prompt.push_str("If you must ask for confirmation, ask once and wait for user input. If the user replies \"continue\" or \"继续\", treat that as approval to proceed with the default.\n");
    prompt.push_str("Return a single JSON object with this shape:\n");
    prompt.push_str("{\"message\":\"brief update\",\"actions\":[...]}.\n");
    prompt.push_str("Action schemas:\n");
    prompt.push_str(
        "- terminal.exec: {\"type\":\"terminal.exec\",\"id\":\"...\",\"cmd\":\"...\",\"cwd\":\"optional\"}\n",
    );
    prompt.push_str(
        "- terminal.run: {\"type\":\"terminal.run\",\"id\":\"...\",\"program\":\"...\",\"args\":[\"arg\"],\"cwd\":\"optional\"}\n",
    );
    prompt.push_str("- fs.read: {\"type\":\"fs.read\",\"id\":\"...\",\"path\":\"...\"}\n");
    prompt.push_str(
        "- fs.write: {\"type\":\"fs.write\",\"id\":\"...\",\"path\":\"...\",\"content\":\"...\"}\n",
    );
    prompt.push_str(
        "- fs.search: {\"type\":\"fs.search\",\"id\":\"...\",\"pattern\":\"...\",\"paths\":[\"...\"]}\n",
    );
    prompt.push_str("- git.status: {\"type\":\"git.status\",\"id\":\"...\"}\n");
    prompt.push_str("- git.diff: {\"type\":\"git.diff\",\"id\":\"...\",\"path\":\"optional\"}\n");
    prompt.push_str(
        "- tests.run: {\"type\":\"tests.run\",\"id\":\"...\",\"program\":\"...\",\"args\":[\"arg\"]}\n",
    );
    prompt.push_str(
        "- plan.update: {\"type\":\"plan.update\",\"id\":\"...\",\"plan\":{\"goal\":\"...\",\"steps\":[{\"id\":\"...\",\"title\":\"...\",\"status\":\"pending\",\"done\":false}]}}\n",
    );
    prompt.push_str(
        "- task.update: {\"type\":\"task.update\",\"id\":\"...\",\"tasks\":{\"items\":[{\"id\":\"...\",\"title\":\"...\",\"status\":\"todo\"}]}}\n",
    );
    prompt.push_str("- user.ask: {\"type\":\"user.ask\",\"id\":\"...\",\"question\":\"...\"}\n");
    prompt.push_str("Use plan.update when planning is needed, but execute tools for direct requests.\n");
    prompt.push_str("Ask the user only if required inputs are missing.\n");
    prompt.push_str("Avoid repeating identical tool calls when recent observations already contain the answer.\n");
    prompt.push_str("If the user asks to scan or read the entire project, confirm with user.ask before broad searches.\n");
    prompt.push_str("For directory listing on Windows, use terminal.exec with command \"dir\".\n");
    prompt
}

fn build_plan_system_prompt(profile: &LlmProfile) -> String {
    let mut prompt = String::new();
    let base = profile.prompt.trim();
    if !base.is_empty() {
        prompt.push_str(base);
        prompt.push_str("\n\n");
    }
    prompt.push_str("You generate concise execution plans for a coding agent.\n");
    prompt.push_str("Return JSON only. Do not wrap in markdown.\n");
    prompt.push_str("Provide 4-8 concrete steps.\n");
    prompt
}

fn allowed_action_list(allowed: &Option<HashSet<String>>) -> Vec<String> {
    let ordered = [
        "terminal.exec",
        "terminal.run",
        "fs.read",
        "fs.write",
        "fs.search",
        "git.status",
        "git.diff",
        "tests.run",
        "plan.update",
        "task.update",
        "user.ask",
    ];
    let mut list = Vec::new();
    for action in ordered {
        if matches!(action, "plan.update" | "task.update" | "user.ask") {
            list.push(action.to_string());
            continue;
        }
        if let Some(allowed) = allowed {
            if allowed.contains(action) {
                list.push(action.to_string());
            }
        } else {
            list.push(action.to_string());
        }
    }
    list
}

fn build_user_prompt(state: &RunState) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!("Platform: {}\n", std::env::consts::OS));
    prompt.push_str(&format!("Workspace: {}\n", state.tool_context.cwd));
    prompt.push_str(&format!(
        "Budget: {}/{}\n",
        state.budget.used_steps, state.budget.max_steps
    ));
    if let Some(err) = &state.last_error {
        prompt.push_str(&format!("Last error: {}\n", trim_to(err, 600)));
    }
    prompt.push_str("Plan:\n");
    if let Some(plan) = &state.plan {
        prompt.push_str(&format!("Goal: {}\n", trim_to(&plan.goal, 400)));
        for step in &plan.steps {
            prompt.push_str(&format!(
                "- [{}] {} ({})\n",
                if step.done { "x" } else { " " },
                trim_to(&step.title, 240),
                step.status
            ));
        }
    } else {
        prompt.push_str("none\n");
    }
    prompt.push_str("Tasks:\n");
    if let Some(tasks) = &state.tasks {
        for task in &tasks.items {
            prompt.push_str(&format!(
                "- {} ({})\n",
                trim_to(&task.title, 240),
                task.status
            ));
        }
    } else {
        prompt.push_str("none\n");
    }
    if !state.recent_observations.is_empty() {
        prompt.push_str("Recent observations:\n");
        for obs in &state.recent_observations {
            prompt.push_str(&format!("- {}\n", trim_to(obs, 600)));
        }
    }
    prompt.push_str("Conversation:\n");
    let start = state.messages.len().saturating_sub(6);
    for msg in state.messages.iter().skip(start) {
        prompt.push_str(&format!(
            "- {}: {}\n",
            msg.role,
            trim_to(&msg.content, 1200)
        ));
    }
    prompt
}

fn parse_plan_response(raw: &str, goal_hint: Option<&str>) -> Result<Plan, String> {
    let value = parse_json_payload(raw)?;
    match value {
        serde_json::Value::Array(_) => {
            let steps = parse_plan_steps(&value);
            if steps.is_empty() {
                return Err("Plan steps missing".to_string());
            }
            Ok(Plan {
                version: 1,
                goal: goal_hint.unwrap_or("Plan").to_string(),
                steps,
            })
        }
        _ => parse_plan_value(&value, goal_hint),
    }
}

fn parse_llm_response(raw: &str, goal_hint: Option<&str>) -> Result<LlmDecision, String> {
    let value = parse_json_payload(raw)?;
    let message = extract_message(&value);
    let actions = match &value {
        serde_json::Value::Array(_) => parse_actions_value(&value, goal_hint)?,
        serde_json::Value::Object(map) => {
            if let Some(actions_value) = map.get("actions") {
                parse_actions_value(actions_value, goal_hint)?
            } else if let Some(action_value) = map.get("action") {
                parse_actions_value(action_value, goal_hint)?
            } else if map.get("type").is_some() {
                parse_actions_value(&value, goal_hint)?
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };
    Ok(LlmDecision { message, actions })
}

fn parse_json_payload(raw: &str) -> Result<serde_json::Value, String> {
    let cleaned = strip_code_fence(raw);
    match serde_json::from_str::<serde_json::Value>(&cleaned) {
        Ok(value) => return Ok(value),
        Err(primary_err) => {
            if let Some(snippet) = extract_json_snippet(&cleaned) {
                match serde_json::from_str::<serde_json::Value>(&snippet) {
                    Ok(value) => return Ok(value),
                    Err(snippet_err) => {
                        return Err(build_json_parse_error(
                            raw,
                            &cleaned,
                            Some(&snippet),
                            &primary_err,
                            Some(&snippet_err),
                        ));
                    }
                }
            }
            Err(build_json_parse_error(raw, &cleaned, None, &primary_err, None))
        }
    }
}

fn strip_code_fence(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let mut lines = trimmed.lines();
    let _ = lines.next();
    let mut content = lines.collect::<Vec<&str>>().join("\n");
    if let Some(end) = content.rfind("```") {
        content.truncate(end);
    }
    content.trim().to_string()
}

fn extract_json_snippet(raw: &str) -> Option<String> {
    let start_obj = raw.find('{');
    let start_arr = raw.find('[');
    let (start, end_char) = match (start_obj, start_arr) {
        (Some(obj), Some(arr)) => {
            if obj < arr {
                (obj, '}')
            } else {
                (arr, ']')
            }
        }
        (Some(obj), None) => (obj, '}'),
        (None, Some(arr)) => (arr, ']'),
        _ => return None,
    };
    let end = if end_char == '}' {
        raw.rfind('}')
    } else {
        raw.rfind(']')
    }?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].to_string())
}

fn build_json_parse_error(
    raw: &str,
    cleaned: &str,
    snippet: Option<&str>,
    primary_err: &serde_json::Error,
    snippet_err: Option<&serde_json::Error>,
) -> String {
    let raw_preview = truncate_preview(raw, 400);
    let cleaned_preview = truncate_preview(cleaned, 400);
    let snippet_preview = snippet
        .map(|value| truncate_preview(value, 400))
        .unwrap_or_else(|| "none".to_string());
    let snippet_err_text = snippet_err
        .map(|err| err.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "Unable to parse JSON response from LLM.\nprimary_error: {}\nsnippet_error: {}\nraw_len: {}\ncleaned_len: {}\nraw_preview: {}\ncleaned_preview: {}\nsnippet_preview: {}",
        primary_err,
        snippet_err_text,
        raw.len(),
        cleaned.len(),
        raw_preview,
        cleaned_preview,
        snippet_preview
    )
}

fn truncate_preview(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut end = max_len;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &value[..end])
}

fn extract_message(value: &serde_json::Value) -> Option<String> {
    let obj = value.as_object()?;
    for key in ["message", "assistantMessage", "assistant_message"] {
        let value = coerce_string(obj.get(key));
        if let Some(text) = value {
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }
    None
}

fn parse_actions_value(
    value: &serde_json::Value,
    goal_hint: Option<&str>,
) -> Result<Vec<Action>, String> {
    match value {
        serde_json::Value::Array(items) => {
            let mut actions = Vec::new();
            for item in items {
                actions.push(parse_action(item, goal_hint)?);
            }
            Ok(actions)
        }
        serde_json::Value::Object(_) => Ok(vec![parse_action(value, goal_hint)?]),
        _ => Err("Actions must be a JSON array or object".to_string()),
    }
}

fn parse_action(value: &serde_json::Value, goal_hint: Option<&str>) -> Result<Action, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "Action must be an object".to_string())?;
    let action_type = obj
        .get("type")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Action type is required".to_string())?;
    let id = coerce_string(obj.get("id")).unwrap_or_else(|| make_id(action_id_prefix(action_type)));
    match action_type {
        "terminal.exec" => {
            let cmd = required_string_field(obj, "cmd")?;
            let cwd = coerce_string(obj.get("cwd")).filter(|value| !value.is_empty());
            Ok(Action::TerminalExec { id, cmd, cwd })
        }
        "terminal.run" => {
            let program = required_string_field(obj, "program")?;
            let args = parse_string_list(obj.get("args"));
            let cwd = coerce_string(obj.get("cwd")).filter(|value| !value.is_empty());
            Ok(Action::TerminalRun {
                id,
                program,
                args,
                cwd,
            })
        }
        "fs.read" => {
            let path = required_string_field(obj, "path")?;
            Ok(Action::FsRead { id, path })
        }
        "fs.write" => {
            let path = required_string_field(obj, "path")?;
            let content = required_string_field(obj, "content")?;
            Ok(Action::FsWrite { id, path, content })
        }
        "fs.search" => {
            let pattern = required_string_field(obj, "pattern")?;
            let paths = parse_string_list(obj.get("paths"));
            let paths = if paths.is_empty() { None } else { Some(paths) };
            Ok(Action::FsSearch { id, pattern, paths })
        }
        "git.status" => Ok(Action::GitStatus { id }),
        "git.diff" => {
            let path = coerce_string(obj.get("path")).filter(|value| !value.is_empty());
            Ok(Action::GitDiff { id, path })
        }
        "tests.run" => {
            let program = required_string_field(obj, "program")?;
            let args = parse_string_list(obj.get("args"));
            Ok(Action::TestsRun { id, program, args })
        }
        "plan.update" => {
            let plan_value = obj.get("plan").cloned().unwrap_or_else(|| value.clone());
            let plan = parse_plan_value(&plan_value, goal_hint)?;
            Ok(Action::PlanUpdate { id, plan })
        }
        "task.update" => {
            let tasks_value = obj.get("tasks").cloned().unwrap_or_else(|| value.clone());
            let tasks = parse_task_list(&tasks_value)?;
            Ok(Action::TaskUpdate { id, tasks })
        }
        "user.ask" => {
            let question = required_string_field(obj, "question")?;
            Ok(Action::UserAsk { id, question })
        }
        _ => Err(format!("Unsupported action type: {}", action_type)),
    }
}

fn action_id_prefix(action_type: &str) -> &str {
    match action_type {
        "terminal.exec" => "term",
        "terminal.run" => "run",
        "fs.read" => "read",
        "fs.write" => "write",
        "fs.search" => "search",
        "git.status" => "git",
        "git.diff" => "diff",
        "tests.run" => "test",
        "plan.update" => "plan",
        "task.update" => "task",
        "user.ask" => "ask",
        _ => "act",
    }
}

fn parse_plan_value(
    value: &serde_json::Value,
    goal_hint: Option<&str>,
) -> Result<Plan, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "Plan must be an object".to_string())?;
    let goal = coerce_string(obj.get("goal"))
        .or_else(|| goal_hint.map(|value| value.to_string()))
        .unwrap_or_default();
    if goal.trim().is_empty() {
        return Err("Plan goal is required".to_string());
    }
    let steps_value = obj
        .get("steps")
        .ok_or_else(|| "Plan steps are required".to_string())?;
    let steps = parse_plan_steps(steps_value);
    if steps.is_empty() {
        return Err("Plan steps are required".to_string());
    }
    Ok(Plan {
        version: 1,
        goal,
        steps,
    })
}

fn parse_plan_steps(value: &serde_json::Value) -> Vec<PlanStep> {
    let mut steps = Vec::new();
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(step) = parse_plan_step(item) {
                    steps.push(step);
                }
            }
        }
        serde_json::Value::String(text) => {
            if !text.trim().is_empty() {
                steps.push(PlanStep {
                    id: make_id("plan"),
                    title: text.trim().to_string(),
                    status: "pending".to_string(),
                    done: false,
                });
            }
        }
        _ => {}
    }
    steps
}

fn parse_plan_step(value: &serde_json::Value) -> Option<PlanStep> {
    if let Some(text) = value.as_str() {
        if text.trim().is_empty() {
            return None;
        }
        return Some(PlanStep {
            id: make_id("plan"),
            title: text.trim().to_string(),
            status: "pending".to_string(),
            done: false,
        });
    }
    let obj = value.as_object()?;
    let title = coerce_string(obj.get("title"))
        .or_else(|| coerce_string(obj.get("text")))
        .or_else(|| coerce_string(obj.get("step")))?;
    let id = coerce_string(obj.get("id")).unwrap_or_else(|| make_id("plan"));
    let status = coerce_string(obj.get("status")).unwrap_or_else(|| "pending".to_string());
    let done = obj
        .get("done")
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| status == "done" || status == "skipped");
    Some(PlanStep {
        id,
        title,
        status,
        done,
    })
}

fn parse_task_list(value: &serde_json::Value) -> Result<TaskList, String> {
    let items_value = match value {
        serde_json::Value::Object(map) => map
            .get("items")
            .or_else(|| map.get("tasks"))
            .unwrap_or(value),
        _ => value,
    };
    let items = parse_task_items(items_value);
    if items.is_empty() {
        return Err("Task items are required".to_string());
    }
    Ok(TaskList {
        version: 1,
        items,
    })
}

fn parse_task_items(value: &serde_json::Value) -> Vec<Task> {
    let mut items = Vec::new();
    match value {
        serde_json::Value::Array(entries) => {
            for entry in entries {
                if let Some(task) = parse_task_entry(entry) {
                    items.push(task);
                }
            }
        }
        serde_json::Value::String(text) => {
            if !text.trim().is_empty() {
                items.push(Task {
                    id: make_id("task"),
                    title: text.trim().to_string(),
                    status: "todo".to_string(),
                    notes: None,
                });
            }
        }
        _ => {}
    }
    items
}

fn parse_task_entry(value: &serde_json::Value) -> Option<Task> {
    if let Some(text) = value.as_str() {
        if text.trim().is_empty() {
            return None;
        }
        return Some(Task {
            id: make_id("task"),
            title: text.trim().to_string(),
            status: "todo".to_string(),
            notes: None,
        });
    }
    let obj = value.as_object()?;
    let title = coerce_string(obj.get("title"))
        .or_else(|| coerce_string(obj.get("text")))
        .or_else(|| coerce_string(obj.get("task")))?;
    let id = coerce_string(obj.get("id")).unwrap_or_else(|| make_id("task"));
    let status = coerce_string(obj.get("status")).unwrap_or_else(|| "todo".to_string());
    let notes = coerce_string(obj.get("notes"));
    Some(Task {
        id,
        title,
        status,
        notes,
    })
}

fn parse_string_list(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|value| coerce_string(Some(value)))
            .filter(|value| !value.is_empty())
            .collect(),
        Some(serde_json::Value::String(text)) => text
            .split_whitespace()
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn required_string_field(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<String, String> {
    let value = coerce_string(obj.get(key));
    match value {
        Some(text) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        _ => Err(format!("Field '{}' is required", key)),
    }
}

fn coerce_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(text)) => Some(text.trim().to_string()),
        Some(serde_json::Value::Number(num)) => Some(num.to_string()),
        Some(serde_json::Value::Bool(flag)) => Some(flag.to_string()),
        _ => None,
    }
}

fn trim_to(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    value.chars().take(max_len).collect()
}

fn make_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4())
}
