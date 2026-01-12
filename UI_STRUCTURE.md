# UI_STRUCTURE

This file proposes the cockpit layout and component map for the seven panels.

## Routes

- /console (main cockpit)
- /settings/llm
- /settings/tools
- /settings/security

## Cockpit Layout (default)

- Top: Workbench tabs (Mission / Plan / Loop / Terminal / Diff / Git / Timeline)
- Left rail: Mission + Plan stack
- Center: Loop + Terminal stack
- Right rail: Diff / Git / Timeline (tabbed)

Layout behavior:
- Each rail is resizable.
- Panels can be undocked to a drawer or overlay.

## Component Map

Core panels:
- src/components/MissionPanel.vue
- src/components/PlanPanel.vue
- src/components/LoopPanel.vue
- src/components/TerminalPanel.vue
- src/components/DiffPanel.vue
- src/components/GitPanel.vue
- src/components/TimelinePanel.vue

Shared UI:
- src/components/WorkbenchTabs.vue
- src/components/ResizableSplit.vue
- src/components/PanelShell.vue
- src/components/StatusPills.vue
- src/components/EvidenceList.vue

Pages:
- src/pages/ConsolePage.vue (cockpit shell)
- src/pages/LLMSettingsPage.vue
- src/pages/ToolSettingsPage.vue
- src/pages/SecuritySettingsPage.vue

Layouts:
- src/layouts/AppShell.vue

## State + Events

Stores:
- src/stores/mission.ts
- src/stores/plan.ts
- src/stores/loop.ts
- src/stores/timeline.ts
- src/stores/tools.ts
- src/stores/git.ts
- src/stores/terminal.ts

Event subscription:
- src/agents/orchestrator.ts (single kernel event stream)
- Panels read from stores only, no direct tool calls.

## Panel Responsibilities

Mission:
- Task list, status, budget, last failure.
- Create/Run/Pause/Stop/Rollback actions.

Plan:
- Step list with criteria and HARD STOP.
- Edit plan -> plan.update actions.

Loop:
- Iteration counter, active step, action queue, gate status.
- Decision slots: Continue / Redirect / Stop.

Terminal:
- PTY output with tool association.
- Copy/search in output.

Diff:
- Patch-first diff view.
- One-click rollback to checkpoint.

Git:
- status/diff/commit/patch/branch.

Timeline:
- Event stream with filters and replay.

## Wiring Rules

- UI is read-only over state; writes go through kernel commands.
- Timeline is the source of truth for what happened.
- Each tool call must appear as a Timeline event and in the Loop panel queue.
