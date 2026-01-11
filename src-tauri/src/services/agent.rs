use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::services::audit::{now_ms, AuditLog};
use crate::services::pty::{TerminalExecRequest, TerminalManager};
use crate::services::tools::{
    max_read_bytes, read_file, run_command, search, CommandRequest, ReadFileRequest, SearchMatch,
    SearchRequest, ToolResult,
};
use crate::services::workspace::WorkspaceState;

const AGENT_STATE_EVENT: &str = "agent-state";

#[derive(Clone)]
pub struct AgentManager {
    state: Arc<Mutex<AgentState>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentState {
    pub phase: String,
    pub running: bool,
    pub paused: bool,
    pub auto_run: bool,
    pub current_step_id: Option<String>,
    pub steps: Vec<AgentStep>,
    pub plan_goal: Option<String>,
    pub plan_items: Vec<PlanItem>,
    pub tool_calls: Vec<ToolCall>,
    pub logs: Vec<AgentLog>,
    pub verify_preset: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStep {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub status: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub id: String,
    pub text: String,
    pub status: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub tool: String,
    pub detail: String,
    pub status: String,
    pub started_at: u128,
    pub finished_at: Option<u128>,
    pub duration_ms: Option<u128>,
    pub exit_code: Option<i32>,
    pub summary: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentLog {
    pub id: String,
    pub level: String,
    pub message: String,
    pub timestamp: u128,
}

#[derive(Deserialize)]
pub struct AgentStartRequest {
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct AgentAutoRunRequest {
    pub auto_run: bool,
}

#[derive(Deserialize)]
pub struct AgentVerifyRequest {
    pub preset: String,
}

#[derive(Deserialize)]
pub struct AgentPlanItemsRequest {
    pub items: Vec<String>,
}

#[derive(Deserialize)]
pub struct AgentRemovePlanItemRequest {
    pub id: String,
}

#[derive(Deserialize)]
pub struct AgentGeneratePlanRequest {
    pub goal: String,
    pub max_steps: Option<usize>,
}

#[derive(Deserialize)]
pub struct AgentPlanItemStatusRequest {
    pub id: String,
}

enum PlanAction {
    Terminal { command: String },
    Run { program: String, args: Vec<String> },
    Read { path: String },
    Search { pattern: String, paths: Option<Vec<String>> },
    Test { program: String, args: Vec<String> },
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AgentState::new())),
        }
    }

    pub fn snapshot(&self) -> AgentState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| AgentState::new())
    }

    pub fn set_auto_run(&self, app: &AppHandle, enabled: bool) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            state.auto_run = enabled;
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn set_verify_preset(&self, app: &AppHandle, preset: String) -> Result<AgentState, String> {
        let allowed = ["skip", "npm_build", "npm_test", "cargo_test"];
        if !allowed.contains(&preset.as_str()) {
            return Err("Unknown verify preset".to_string());
        }
        let snapshot = self.with_state(|state| {
            state.verify_preset = preset;
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn add_plan_items(
        &self,
        app: &AppHandle,
        items: Vec<String>,
    ) -> Result<AgentState, String> {
        let normalized: Vec<String> = items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
        let snapshot = self.with_state(|state| {
            for item in normalized {
                state.plan_items.push(PlanItem {
                    id: make_id("plan"),
                    text: item,
                    status: "pending".to_string(),
                });
            }
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn remove_plan_item(
        &self,
        app: &AppHandle,
        id: String,
    ) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            state.plan_items.retain(|item| item.id != id);
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn clear_plan_items(&self, app: &AppHandle) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            state.plan_items.clear();
            state.plan_goal = None;
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn generate_plan(
        &self,
        app: &AppHandle,
        request: AgentGeneratePlanRequest,
    ) -> Result<AgentState, String> {
        let goal = request.goal.trim().to_string();
        if goal.is_empty() {
            return Err("Goal cannot be empty".to_string());
        }
        let running = self
            .state
            .lock()
            .map_err(|_| "Agent state lock poisoned".to_string())?
            .running;
        if running {
            return Err("Cannot generate plan while agent is running".to_string());
        }
        let max_steps = request.max_steps.unwrap_or(6);
        let steps = build_plan_from_goal(&goal, max_steps);
        let snapshot = self.with_state(|state| {
            state.plan_goal = Some(goal.clone());
            state.plan_items = steps
                .into_iter()
                .map(|text| PlanItem {
                    id: make_id("plan"),
                    text,
                    status: "pending".to_string(),
                })
                .collect();
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn skip_plan_item(
        &self,
        app: &AppHandle,
        request: AgentPlanItemStatusRequest,
    ) -> Result<AgentState, String> {
        let mut found = false;
        let snapshot = self.with_state(|state| {
            if let Some(item) = state.plan_items.iter_mut().find(|item| item.id == request.id) {
                item.status = "skipped".to_string();
                found = true;
            }
        })?;
        if !found {
            return Err("Plan item not found".to_string());
        }
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn retry_plan_item(
        &self,
        app: &AppHandle,
        request: AgentPlanItemStatusRequest,
    ) -> Result<AgentState, String> {
        let mut found = false;
        let snapshot = self.with_state(|state| {
            if let Some(item) = state.plan_items.iter_mut().find(|item| item.id == request.id) {
                item.status = "pending".to_string();
                found = true;
            }
        })?;
        if !found {
            return Err("Plan item not found".to_string());
        }
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn reset(&self, app: &AppHandle) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            state.reset_steps();
            state.tool_calls.clear();
            state.logs.clear();
            state.phase = "idle".to_string();
            state.running = false;
            state.paused = false;
            state.current_step_id = None;
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn pause(&self, app: &AppHandle) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            if state.running {
                state.paused = true;
                state.phase = "paused".to_string();
                state.logs.insert(
                    0,
                    AgentLog {
                        id: make_id("log"),
                        level: "warn".to_string(),
                        message: "Agent paused".to_string(),
                        timestamp: now_ms(),
                    },
                );
            }
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn resume(&self, app: &AppHandle) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            if state.paused {
                state.paused = false;
                state.phase = state.current_step_id.clone().unwrap_or_else(|| "idle".to_string());
                state.logs.insert(
                    0,
                    AgentLog {
                        id: make_id("log"),
                        level: "info".to_string(),
                        message: "Agent resumed".to_string(),
                        timestamp: now_ms(),
                    },
                );
            }
        })?;
        self.emit_state(app);
        Ok(snapshot)
    }

    pub fn start(
        &self,
        app: AppHandle,
        terminal: TerminalManager,
        workspace: WorkspaceState,
        audit: AuditLog,
        request: AgentStartRequest,
    ) -> Result<AgentState, String> {
        let snapshot = self.with_state(|state| {
            if state.running {
                return;
            }
            if state.steps.is_empty() || matches!(state.phase.as_str(), "done" | "error") {
                state.reset_steps();
                state.tool_calls.clear();
                state.logs.clear();
            }
            for item in state.plan_items.iter_mut() {
                if item.status != "skipped" {
                    item.status = "pending".to_string();
                }
            }
            state.running = true;
            state.paused = false;
            state.phase = "plan".to_string();
            state.current_step_id = None;
        })?;
        self.emit_state(&app);
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            manager
                .run_pipeline(app, terminal, workspace, audit, request.session_id)
                .await;
        });
        Ok(snapshot)
    }
    fn emit_state(&self, app: &AppHandle) {
        let snapshot = self.snapshot();
        let _ = app.emit(AGENT_STATE_EVENT, snapshot);
    }

    fn with_state<F>(&self, updater: F) -> Result<AgentState, String>
    where
        F: FnOnce(&mut AgentState),
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Agent state lock poisoned".to_string())?;
        updater(&mut state);
        Ok(state.clone())
    }

    async fn run_pipeline(
        &self,
        app: AppHandle,
        terminal: TerminalManager,
        workspace: WorkspaceState,
        audit: AuditLog,
        session_id: Option<String>,
    ) {
        let run = self
            .run_steps(&app, terminal, workspace, audit, session_id)
            .await;
        if let Err(message) = run {
            let _ = self.with_state(|state| {
                state.phase = "error".to_string();
                state.running = false;
                state.paused = false;
                state.current_step_id = None;
                state.logs.insert(
                    0,
                    AgentLog {
                        id: make_id("log"),
                        level: "error".to_string(),
                        message,
                        timestamp: now_ms(),
                    },
                );
            });
            self.emit_state(&app);
        }
    }

    async fn run_steps(
        &self,
        app: &AppHandle,
        terminal: TerminalManager,
        workspace: WorkspaceState,
        audit: AuditLog,
        session_id: Option<String>,
    ) -> Result<(), String> {
        self.run_step(app, "plan", "Plan", || async {
            self.note_plan(app)?;
            Ok(())
        })
        .await?;

        let terminal_exec = terminal.clone();
        let workspace_exec = workspace.clone();
        let audit_exec = audit.clone();
        let session_exec = session_id.clone();
        self.run_step(app, "execute", "Execute", || async move {
            self.execute_plan(app, terminal_exec, workspace_exec, audit_exec, session_exec)
                .await
        })
        .await?;

        let workspace_verify = workspace.clone();
        let audit_verify = audit.clone();
        self.run_step(app, "verify", "Verify", || async move {
            self.verify_step(app, workspace_verify, audit_verify).await
        })
        .await?;

        self.run_step(app, "commit", "Commit", || async {
            self.commit_step(app)?;
            Ok(())
        })
        .await?;

        let _ = self.with_state(|state| {
            state.phase = "done".to_string();
            state.running = false;
            state.paused = false;
            state.current_step_id = None;
            state.logs.insert(
                0,
                AgentLog {
                    id: make_id("log"),
                    level: "info".to_string(),
                    message: "Agent run completed".to_string(),
                    timestamp: now_ms(),
                },
            );
        });
        self.emit_state(app);
        Ok(())
    }

    async fn run_step<F, Fut>(
        &self,
        app: &AppHandle,
        id: &str,
        _title: &str,
        action: F,
    ) -> Result<(), String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        self.wait_if_paused().await;
        let _ = self.with_state(|state| {
            state.current_step_id = Some(id.to_string());
            state.phase = id.to_string();
            set_step_status(state, id, "running", None);
            state.logs.insert(
                0,
                AgentLog {
                    id: make_id("log"),
                    level: "info".to_string(),
                    message: format!("Step {} started", id),
                    timestamp: now_ms(),
                },
            );
        });
        self.emit_state(app);

        if let Err(err) = action().await {
            let _ = self.with_state(|state| {
                set_step_status(state, id, "error", Some(err.clone()));
            });
            self.emit_state(app);
            return Err(err);
        }

        let _ = self.with_state(|state| {
            if let Some(step) = state.steps.iter().find(|step| step.id == id) {
                if step.status == "running" {
                    set_step_status(state, id, "done", None);
                }
            }
            state.logs.insert(
                0,
                AgentLog {
                    id: make_id("log"),
                    level: "info".to_string(),
                    message: format!("Step {} completed", id),
                    timestamp: now_ms(),
                },
            );
        });
        self.emit_state(app);
        Ok(())
    }

    fn note_plan(&self, app: &AppHandle) -> Result<(), String> {
        let _ = self.with_state(|state| {
            let detail = if state.plan_items.is_empty() {
                "No plan items provided".to_string()
            } else {
                format!("User plan: {} items", state.plan_items.len())
            };
            set_step_status(state, "plan", "running", Some(detail));
            if !state.plan_items.is_empty() {
                let joined = state
                    .plan_items
                    .iter()
                    .map(|item| item.text.clone())
                    .collect::<Vec<String>>()
                    .join(" | ");
                state.logs.insert(
                    0,
                    AgentLog {
                        id: make_id("log"),
                        level: "info".to_string(),
                        message: format!("Plan items: {}", joined),
                        timestamp: now_ms(),
                    },
                );
            }
        })?;
        self.emit_state(app);
        Ok(())
    }
    async fn execute_plan(
        &self,
        app: &AppHandle,
        terminal: TerminalManager,
        workspace: WorkspaceState,
        audit: AuditLog,
        session_id: Option<String>,
    ) -> Result<(), String> {
        let is_empty = self
            .state
            .lock()
            .map_err(|_| "Agent state lock poisoned".to_string())?
            .plan_items
            .is_empty();
        if is_empty {
            let _ = self.with_state(|state| {
                set_step_status(state, "execute", "running", Some("No plan items".to_string()));
            });
            self.emit_state(app);
            return Ok(());
        }
        let mut index = 0usize;
        loop {
            let item = {
                let state = self
                    .state
                    .lock()
                    .map_err(|_| "Agent state lock poisoned".to_string())?;
                if index >= state.plan_items.len() {
                    break;
                }
                state.plan_items[index].clone()
            };
            if item.status == "skipped" {
                index += 1;
                continue;
            }
            self.wait_if_paused().await;
            let _ = self.with_state(|state| {
                set_plan_status(state, &item.id, "running");
                set_step_status(
                    state,
                    "execute",
                    "running",
                    Some(format!("Executing: {}", item.text)),
                );
            });
            self.emit_state(app);

            let action = parse_plan_action(&item.text);
            let result = match action {
                Some(action) => {
                    let detail = describe_action(&action);
                    self.run_tool(
                        app,
                        action.tool_name(),
                        detail,
                        || run_action(action, &terminal, &workspace, &audit, session_id.clone()),
                    )
                    .await
                }
                None => {
                    let _ = self.with_state(|state| {
                        set_plan_status(state, &item.id, "skipped");
                        state.logs.insert(
                            0,
                            AgentLog {
                                id: make_id("log"),
                                level: "warn".to_string(),
                                message: format!("No tool mapping for: {}", item.text),
                                timestamp: now_ms(),
                            },
                        );
                    });
                    self.emit_state(app);
                    continue;
                }
            };

            match result {
                Ok(_) => {
                    let _ = self.with_state(|state| {
                        set_plan_status(state, &item.id, "done");
                    });
                    self.emit_state(app);
                }
                Err(err) => {
                    let _ = self.with_state(|state| {
                        set_plan_status(state, &item.id, "error");
                    });
                    self.emit_state(app);
                    return Err(err);
                }
            }
            index += 1;
        }

        Ok(())
    }

    async fn verify_step(
        &self,
        app: &AppHandle,
        workspace: WorkspaceState,
        audit: AuditLog,
    ) -> Result<(), String> {
        let preset = self
            .state
            .lock()
            .map_err(|_| "Agent state lock poisoned".to_string())?
            .verify_preset
            .clone();

        let command = match preset.as_str() {
            "skip" => None,
            "npm_build" => Some(("npm".to_string(), vec!["run".to_string(), "build".to_string()])),
            "npm_test" => Some(("npm".to_string(), vec!["test".to_string()])),
            "cargo_test" => Some(("cargo".to_string(), vec!["test".to_string()])),
            _ => None,
        };

        if let Some((program, args)) = command {
            let detail = format!("{} {}", program, args.join(" "));
            self.run_tool(app, "tests.run", detail, || {
                run_command(
                    CommandRequest {
                        program,
                        args: Some(args),
                        cwd: Some(workspace.root().to_string_lossy().to_string()),
                        env: None,
                        timeout_ms: Some(120_000),
                    },
                    workspace.root().to_string_lossy().as_ref(),
                    &audit,
                )
            })
            .await?;
            Ok(())
        } else {
            let _ = self.with_state(|state| {
                set_step_status(state, "verify", "skipped", Some("Skipped by config".to_string()));
                state.logs.insert(
                    0,
                    AgentLog {
                        id: make_id("log"),
                        level: "warn".to_string(),
                        message: "Verify step skipped".to_string(),
                        timestamp: now_ms(),
                    },
                );
            });
            self.emit_state(app);
            Ok(())
        }
    }

    fn commit_step(&self, app: &AppHandle) -> Result<(), String> {
        let _ = self.with_state(|state| {
            let ok_calls = state.tool_calls.iter().filter(|call| call.status == "ok").count();
            set_step_status(
                state,
                "commit",
                "running",
                Some(format!("Summary prepared ({} tools ok)", ok_calls)),
            );
            state.logs.insert(
                0,
                AgentLog {
                    id: make_id("log"),
                    level: "info".to_string(),
                    message: "Summary prepared".to_string(),
                    timestamp: now_ms(),
                },
            );
        })?;
        self.emit_state(app);
        Ok(())
    }

    async fn run_tool<F>(
        &self,
        app: &AppHandle,
        tool: &str,
        detail: String,
        action: F,
    ) -> Result<ToolResult, String>
    where
        F: FnOnce() -> Result<ToolResult, String>,
    {
        self.wait_if_paused().await;
        let call_id = make_id("tool");
        let started_at = now_ms();
        let _ = self.with_state(|state| {
            state.tool_calls.insert(
                0,
                ToolCall {
                    id: call_id.clone(),
                    tool: tool.to_string(),
                    detail: detail.clone(),
                    status: "running".to_string(),
                    started_at,
                    finished_at: None,
                    duration_ms: None,
                    exit_code: None,
                    summary: None,
                    error: None,
                },
            );
            state.logs.insert(
                0,
                AgentLog {
                    id: make_id("log"),
                    level: "info".to_string(),
                    message: format!("Tool {} started", tool),
                    timestamp: now_ms(),
                },
            );
        });
        self.emit_state(app);

        let result = action();
        let finished_at = now_ms();

        let update = |state: &mut AgentState,
                      status: &str,
                      summary: Option<String>,
                      error: Option<String>,
                      exit_code: Option<i32>| {
            if let Some(call) = state.tool_calls.iter_mut().find(|call| call.id == call_id) {
                call.status = status.to_string();
                call.finished_at = Some(finished_at);
                call.duration_ms = Some(finished_at.saturating_sub(started_at));
                call.summary = summary;
                call.error = error;
                call.exit_code = exit_code;
            }
        };

        match result {
            Ok(result) => {
                if result.ok {
                    let summary = Some(summarize_result(&result));
                    let _ = self.with_state(|state| {
                        update(state, "ok", summary, None, result.exit_code);
                    });
                    self.emit_state(app);
                    Ok(result)
                } else {
                    let summary = Some(summarize_result(&result));
                    let message = result
                        .stderr_excerpt
                        .clone()
                        .unwrap_or_else(|| "Tool failed".to_string());
                    let _ = self.with_state(|state| {
                        update(state, "error", summary, Some(message.clone()), result.exit_code);
                        state.logs.insert(
                            0,
                            AgentLog {
                                id: make_id("log"),
                                level: "error".to_string(),
                                message: format!("Tool {} failed", tool),
                                timestamp: now_ms(),
                            },
                        );
                    });
                    self.emit_state(app);
                    Err(message)
                }
            }
            Err(err) => {
                let _ = self.with_state(|state| {
                    update(state, "error", None, Some(err.clone()), None);
                    state.logs.insert(
                        0,
                        AgentLog {
                            id: make_id("log"),
                            level: "error".to_string(),
                            message: format!("Tool {} failed", tool),
                            timestamp: now_ms(),
                        },
                    );
                });
                self.emit_state(app);
                Err(err)
            }
        }
    }

    async fn wait_if_paused(&self) {
        loop {
            let paused = self
                .state
                .lock()
                .map(|state| state.paused)
                .unwrap_or(false);
            if !paused {
                break;
            }
            std::thread::sleep(Duration::from_millis(120));
        }
    }
}
impl AgentState {
    fn new() -> Self {
        Self {
            phase: "idle".to_string(),
            running: false,
            paused: false,
            auto_run: true,
            current_step_id: None,
            steps: default_steps(),
            plan_goal: None,
            plan_items: Vec::new(),
            tool_calls: Vec::new(),
            logs: Vec::new(),
            verify_preset: "skip".to_string(),
        }
    }

    fn reset_steps(&mut self) {
        self.steps = default_steps();
    }
}

impl PlanAction {
    fn tool_name(&self) -> &'static str {
        match self {
            PlanAction::Terminal { .. } => "terminal.exec_interactive",
            PlanAction::Run { .. } => "terminal.run_command",
            PlanAction::Read { .. } => "fs.read_file",
            PlanAction::Search { .. } => "fs.search",
            PlanAction::Test { .. } => "tests.run",
        }
    }
}

fn default_steps() -> Vec<AgentStep> {
    vec![
        AgentStep {
            id: "plan".to_string(),
            title: "Plan".to_string(),
            detail: "Define goals + tools".to_string(),
            status: "pending".to_string(),
        },
        AgentStep {
            id: "execute".to_string(),
            title: "Execute".to_string(),
            detail: "Run tool calls".to_string(),
            status: "pending".to_string(),
        },
        AgentStep {
            id: "verify".to_string(),
            title: "Verify".to_string(),
            detail: "Run checks".to_string(),
            status: "pending".to_string(),
        },
        AgentStep {
            id: "commit".to_string(),
            title: "Commit".to_string(),
            detail: "Summarize changes".to_string(),
            status: "pending".to_string(),
        },
    ]
}

fn make_id(prefix: &str) -> String {
    format!("{}-{}", prefix, Uuid::new_v4())
}

fn set_step_status(state: &mut AgentState, id: &str, status: &str, detail: Option<String>) {
    if let Some(step) = state.steps.iter_mut().find(|step| step.id == id) {
        step.status = status.to_string();
        if let Some(detail) = detail {
            step.detail = detail;
        }
    }
}

fn set_plan_status(state: &mut AgentState, id: &str, status: &str) {
    if let Some(item) = state.plan_items.iter_mut().find(|item| item.id == id) {
        item.status = status.to_string();
    }
}

fn summarize_result(result: &ToolResult) -> String {
    if let Some(stderr) = &result.stderr_excerpt {
        return stderr.trim().lines().next().unwrap_or("error").to_string();
    }
    if let Some(stdout) = &result.stdout_excerpt {
        return stdout.trim().lines().next().unwrap_or("ok").to_string();
    }
    if result.ok {
        "ok".to_string()
    } else {
        "error".to_string()
    }
}

fn parse_plan_action(text: &str) -> Option<PlanAction> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if let Some(rest) = strip_prefix(trimmed, &lower, "term:") {
        let command = rest.to_string();
        return Some(PlanAction::Terminal { command });
    }
    if let Some(rest) = strip_prefix(trimmed, &lower, "terminal:") {
        let command = rest.to_string();
        return Some(PlanAction::Terminal { command });
    }
    if let Some(rest) = strip_prefix(trimmed, &lower, "run:") {
        let (program, args) = split_command(rest)?;
        return Some(PlanAction::Run { program, args });
    }
    if let Some(rest) = strip_prefix(trimmed, &lower, "read:") {
        let path = rest.to_string();
        return Some(PlanAction::Read { path });
    }
    if let Some(rest) = strip_prefix(trimmed, &lower, "search:") {
        let (pattern, paths) = split_search(rest);
        return Some(PlanAction::Search { pattern, paths });
    }
    if let Some(rest) = strip_prefix(trimmed, &lower, "test:") {
        let (program, args) = split_command(rest)?;
        return Some(PlanAction::Test { program, args });
    }
    None
}

fn strip_prefix<'a>(raw: &'a str, lower: &str, prefix: &str) -> Option<&'a str> {
    if lower.starts_with(prefix) {
        Some(raw[prefix.len()..].trim())
    } else {
        None
    }
}

fn split_command(input: &str) -> Option<(String, Vec<String>)> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let program = parts[0].to_string();
    let args = parts.iter().skip(1).map(|part| part.to_string()).collect();
    Some((program, args))
}

fn split_search(input: &str) -> (String, Option<Vec<String>>) {
    let mut pattern = input.trim().to_string();
    let mut paths: Option<Vec<String>> = None;
    if let Some((left, right)) = input.split_once('|') {
        pattern = left.trim().to_string();
        let path_list = parse_path_list(right);
        if !path_list.is_empty() {
            paths = Some(path_list);
        }
    } else if let Some((left, right)) = input.split_once(" in ") {
        pattern = left.trim().to_string();
        let path_list = parse_path_list(right);
        if !path_list.is_empty() {
            paths = Some(path_list);
        }
    }
    (pattern, paths)
}

fn parse_path_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

fn describe_action(action: &PlanAction) -> String {
    match action {
        PlanAction::Terminal { command } => command.clone(),
        PlanAction::Run { program, args } => format!("{} {}", program, args.join(" ")),
        PlanAction::Read { path } => path.clone(),
        PlanAction::Search { pattern, paths } => match paths {
            Some(paths) => format!("{} in {}", pattern, paths.join(", ")),
            None => pattern.clone(),
        },
        PlanAction::Test { program, args } => format!("{} {}", program, args.join(" ")),
    }
}

fn build_plan_from_goal(goal: &str, max_steps: usize) -> Vec<String> {
    let trimmed = goal.trim();
    let limit = max_steps.clamp(2, 10);
    let mut segments = split_goal_segments(trimmed);
    if segments.len() >= 2 {
        segments.truncate(limit);
        return segments;
    }

    let lower = trimmed.to_lowercase();
    let mut steps = Vec::new();
    push_unique(&mut steps, format!("Clarify objective: {}", trimmed));
    push_unique(&mut steps, "Inspect workspace and relevant files".to_string());

    if lower.contains("ui") || lower.contains("layout") || lower.contains("style") {
        push_unique(&mut steps, "Update UI structure and styling".to_string());
    } else if lower.contains("bug") || lower.contains("error") || lower.contains("fix") {
        push_unique(&mut steps, "Reproduce issue and implement fix".to_string());
    } else if lower.contains("api") || lower.contains("endpoint") {
        push_unique(&mut steps, "Update APIs and data flow".to_string());
    } else {
        push_unique(&mut steps, "Implement required changes".to_string());
    }

    if lower.contains("test") || lower.contains("verify") || lower.contains("build") {
        push_unique(&mut steps, "Run verification checks".to_string());
    } else {
        push_unique(&mut steps, "Verify behavior".to_string());
    }

    push_unique(&mut steps, "Summarize results and next steps".to_string());
    steps.truncate(limit);
    steps
}

fn split_goal_segments(goal: &str) -> Vec<String> {
    goal.split(|ch| ch == '\n' || ch == ';' || ch == '.' || ch == ',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect()
}

fn push_unique(steps: &mut Vec<String>, item: String) {
    if !steps.iter().any(|step| step.eq_ignore_ascii_case(&item)) {
        steps.push(item);
    }
}

fn run_action(
    action: PlanAction,
    terminal: &TerminalManager,
    workspace: &WorkspaceState,
    audit: &AuditLog,
    session_id: Option<String>,
) -> Result<ToolResult, String> {
    match action {
        PlanAction::Terminal { command } => {
            let request = TerminalExecRequest {
                command,
                session_id,
                shell: None,
                cwd: None,
                cols: None,
                rows: None,
                timeout_ms: Some(15_000),
                max_bytes: Some(24_000),
            };
            let cwd = workspace.root();
            terminal.exec_interactive(request, cwd, audit)
        }
        PlanAction::Run { program, args } => {
            let cwd = workspace.root();
            run_command(
                CommandRequest {
                    program,
                    args: Some(args),
                    cwd: Some(cwd.to_string_lossy().to_string()),
                    env: None,
                    timeout_ms: None,
                },
                cwd.to_string_lossy().as_ref(),
                audit,
            )
        }
        PlanAction::Read { path } => read_file_tool(workspace, audit, path),
        PlanAction::Search { pattern, paths } => search_tool(workspace, audit, pattern, paths),
        PlanAction::Test { program, args } => {
            let cwd = workspace.root();
            run_command(
                CommandRequest {
                    program,
                    args: Some(args),
                    cwd: Some(cwd.to_string_lossy().to_string()),
                    env: None,
                    timeout_ms: Some(120_000),
                },
                cwd.to_string_lossy().as_ref(),
                audit,
            )
        }
    }
}

fn read_file_tool(
    workspace: &WorkspaceState,
    audit: &AuditLog,
    path: String,
) -> Result<ToolResult, String> {
    let request = ReadFileRequest { path };
    let resolved = workspace.resolve_path(&request.path)?;
    let max_bytes = max_read_bytes();
    let file = File::open(&resolved).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    let mut handle = file.take(max_bytes as u64);
    handle.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let truncated = metadata.len() as usize > buffer.len();
    let content = String::from_utf8_lossy(&buffer).to_string();
    Ok(read_file(request, content, truncated, audit))
}

fn search_tool(
    workspace: &WorkspaceState,
    audit: &AuditLog,
    pattern: String,
    paths: Option<Vec<String>>,
) -> Result<ToolResult, String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--json");
    cmd.arg(&pattern);

    let resolved_paths = if let Some(paths) = &paths {
        let mut resolved = Vec::new();
        for path in paths {
            resolved.push(workspace.resolve_path(path)?);
        }
        resolved
    } else {
        vec![workspace.root()]
    };

    for path in &resolved_paths {
        cmd.arg(path);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_string());
    }

    let matches = parse_rg_json(&output.stdout, 200);
    Ok(search(
        SearchRequest {
            pattern,
            paths,
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
