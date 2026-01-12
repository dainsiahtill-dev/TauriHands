# MILESTONES

This file defines the delivery order for the TauriHands final blueprint
and the minimum acceptance criteria for each milestone.

## M0 - Task + Timeline Foundation

Scope:
- Task config persisted per task id.
- Timeline writer (append-only ndjson).
- Event schema stable enough for UI consumption.

Deliverables:
- .taurihands/tasks/<taskId>/task.json
- .taurihands/tasks/<taskId>/plan.json
- .taurihands/tasks/<taskId>/timeline.ndjson
- Kernel event bus writes to timeline.
- UI can render Timeline panel from events.

Definition of done:
- Start a task -> timeline is created and appended.
- Events are replayable in UI after restart.

## M1 - Tool Dispatcher + Guardrails

Scope:
- Unified tool entry point in Rust.
- Path sandbox, command policy, network policy.

Deliverables:
- Tool dispatcher with allow/block/confirm hooks.
- Sandbox denies out-of-root reads/writes by default.
- Audit log for each tool call.

Definition of done:
- Forbidden path access is blocked with a clear error.
- Dangerous commands are blocked or confirmed.

## M2 - Loop Engine MVP (Planner/Executor/Judge)

Scope:
- Single model loop engine.
- Planner -> Executor -> Judge cycle.
- Budget tracking (steps/tool calls/wall time).

Deliverables:
- Loop state machine.
- Budget enforcement -> needs_user state.
- Judge rules for tests/build/lint/git clean (configurable).

Definition of done:
- Given a failing test, the loop iterates until tests pass or budget stops.
- Judge, not the model, decides completion.

## M3 - Evidence Pack + Checkpoints

Scope:
- Evidence capture per iteration.
- Checkpoint create/restore.

Deliverables:
- Evidence bundle per iteration (commands, output, diff, error summary).
- checkpoint.create / checkpoint.restore tools.
- Rollback to last checkpoint from UI.

Definition of done:
- One-click rollback restores last checkpoint.
- Evidence is visible in Timeline and Diff panels.

## M4 - Cockpit UI (Seven Panels)

Scope:
- Mission / Plan / Loop / Terminal / Diff / Git / Timeline.

Deliverables:
- New layout with resizable panels.
- Panel routing or tabbing with state sync.
- Mission panel to create/run tasks.

Definition of done:
- All seven panels are reachable and render live data.
- Panel actions call kernel tools and update Timeline.

## M5 - Decision Slots + Autonomy Levels

Scope:
- Human decision checkpoints.
- Autonomy level switches.

Deliverables:
- HARD STOP steps pause loop.
- Autonomy levels: auto / semi / plan-only.
- Decision slot UI with continue/redirect/stop.

Definition of done:
- Loop pauses at HARD STOP and resumes only on user action.

## M6 - Optional Extensions

Scope:
- Sidecar PTY host.
- Browser host for fetch/extract.

Deliverables:
- Sidecar lifecycle + IO forwarding.
- Browser tool gated by allowNetwork.

Definition of done:
- PTY sidecar runs cross-platform.
- Browser tool is fully disabled when network is off.
