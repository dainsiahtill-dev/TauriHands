<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { agentStore } from "../agents/orchestrator";
import { missionStore } from "../stores/mission";
import WorkspacePanel from "./WorkspacePanel.vue";
import StatusPills from "./StatusPills.vue";

const { state } = agentStore;
const { state: missionState, loadActive, saveActive, defaultTaskConfig } = missionStore;

const run = computed(() => state.run);
const workspacePath = computed(() => run.value?.toolContext?.cwd ?? "Not set");
const runId = computed(() => run.value?.runId ?? "-");
const agentState = computed(() => run.value?.agentState ?? "IDLE");
const taskCount = computed(() => run.value?.tasks?.items?.length ?? 0);
const lastError = computed(() => {
  const direct = run.value?.lastError?.trim() ?? "";
  if (direct) return direct;
  const log = state.logs.find((entry) => entry.level === "error");
  return log?.message?.trim() ?? "";
});
const errorDetail = computed(() => {
  if (agentState.value !== "ERROR") return "";
  return lastError.value || "No error details recorded.";
});

const pills = computed(() => [
  {
    label: "State",
    value: agentState.value,
    tone: agentState.value === "ERROR" ? "error" as const : "info" as const,
    detail: errorDetail.value,
  },
  { label: "Run", value: runId.value.slice(0, 8) || "-" },
  { label: "Tasks", value: String(taskCount.value) },
]);

const task = reactive(defaultTaskConfig());
const completionText = computed({
  get: () => task.completion.join("\n"),
  set: (value: string) => {
    task.completion = value
      .split(/\n+/)
      .map((item) => item.trim())
      .filter(Boolean);
  },
});
const saveStatus = ref("");
const judgeText = ref("[]");
const judgeStatus = ref("");
const showAdvanced = ref(false);
const showJudgeRules = ref(false);

async function loadJudgeRules(taskId: string) {
  try {
    const rules = (await invoke("judge_get_rules", { task_id: taskId })) as unknown;
    judgeText.value = JSON.stringify(rules ?? [], null, 2);
  } catch {
    judgeText.value = "[]";
  }
}

watch(
  () => missionState.active,
  (value) => {
    if (!value) return;
    Object.assign(task, value);
  },
  { immediate: true },
);

onMounted(() => {
  void loadActive();
});

async function saveConfig() {
  saveStatus.value = "";
  try {
    if (workspacePath.value && workspacePath.value !== "Not set") {
      task.workspace = workspacePath.value;
    }
    const result = await saveActive({ ...task });
    Object.assign(task, result);
    saveStatus.value = "Saved";
  } catch {
    saveStatus.value = "Save failed";
  }
}

async function saveJudgeRules() {
  judgeStatus.value = "";
  try {
    if (!task.taskId) {
      judgeStatus.value = "Save task first.";
      return;
    }
    const rules = JSON.parse(judgeText.value || "[]");
    await invoke("judge_set_rules", { request: { task_id: task.taskId, rules } });
    judgeStatus.value = "Saved";
  } catch (error) {
    judgeStatus.value = error instanceof Error ? error.message : "Save failed";
  }
}

function toggleAdvanced() {
  showAdvanced.value = !showAdvanced.value;
}

function toggleJudgeRules() {
  showJudgeRules.value = !showJudgeRules.value;
}

function quickStartMission() {
  // Quick start logic - auto-save and start
  saveConfig().then(() => {
    // Trigger start via agent store
    const { start } = agentStore;
    start();
  });
}

function quickResetMission() {
  // Reset mission state
  Object.assign(task, defaultTaskConfig());
  saveStatus.value = "Reset";
}

watch(
  () => task.taskId,
  (value) => {
    if (!value) return;
    void loadJudgeRules(value);
  },
  { immediate: true },
);
</script>

<template>
  <div class="mission-panel">
    <div class="section">
      <p class="eyebrow">Mission status</p>
      <StatusPills :items="pills" />
      <div class="quick-actions">
        <button 
          class="btn primary quick-btn" 
          type="button" 
          @click="quickStartMission"
          :disabled="agentState === 'RUNNING'"
        >
          <span class="btn-icon">▶</span>
          Quick Start
        </button>
        <button 
          class="btn ghost quick-btn" 
          type="button" 
          @click="quickResetMission"
        >
          <span class="btn-icon">↺</span>
          Reset
        </button>
      </div>
    </div>

    <div class="section">
      <p class="eyebrow">Workspace</p>
      <p class="path">{{ workspacePath }}</p>
      <div class="workspace-block">
        <WorkspacePanel />
      </div>
    </div>

    <div class="section">
      <div class="section-header" @click="toggleAdvanced">
        <p class="eyebrow">Task config</p>
        <span class="toggle-icon" :class="{ 'is-open': showAdvanced }">▼</span>
      </div>
      <div v-show="showAdvanced" class="collapsible-content">
      <div class="form-grid">
        <label>
          Task ID
          <input v-model="task.taskId" type="text" class="field" placeholder="task-001" />
        </label>
        <label>
          Goal
          <input v-model="task.goal" type="text" class="field" placeholder="Describe the mission goal" />
        </label>
        <label>
          Completion criteria (one per line)
          <textarea
            v-model="completionText"
            class="field"
            rows="3"
            placeholder="tests pass&#10;git clean"
          ></textarea>
        </label>
        <label>
          Autonomy
          <select v-model="task.autonomy" class="field">
            <option value="auto">auto</option>
            <option value="semi">semi</option>
            <option value="plan_only">plan_only</option>
          </select>
        </label>
        <label>
          Budget: max iterations
          <input v-model.number="task.budget.maxIterations" type="number" class="field" min="1" />
        </label>
        <label>
          Budget: max tool calls
          <input v-model.number="task.budget.maxToolCalls" type="number" class="field" min="1" />
        </label>
        <label>
          Budget: max wall time (ms)
          <input v-model.number="task.budget.maxWallTimeMs" type="number" class="field" min="0" />
        </label>
        <label>
          Command policy
          <select v-model="task.riskPolicy.commandPolicy" class="field">
            <option value="confirm">confirm</option>
            <option value="allowlist">allowlist</option>
            <option value="blocklist">blocklist</option>
          </select>
        </label>
        <label>
          Path policy
          <select v-model="task.riskPolicy.pathPolicy" class="field">
            <option value="workspace_only">workspace_only</option>
            <option value="allowlist">allowlist</option>
          </select>
        </label>
        <label>
          Allow network
          <select v-model="task.riskPolicy.allowNetwork" class="field">
            <option :value="false">false</option>
            <option :value="true">true</option>
          </select>
        </label>
      </div>
      <div class="form-actions">
        <button class="btn primary" type="button" @click="saveConfig" :disabled="missionState.loading">
          Save task
        </button>
        <span class="status" v-if="saveStatus">{{ saveStatus }}</span>
        <span class="error" v-else-if="missionState.error">{{ missionState.error }}</span>
      </div>
      </div>
    </div>

    <div class="section">
      <div class="section-header" @click="toggleJudgeRules">
        <p class="eyebrow">Judge rules (JSON)</p>
        <span class="toggle-icon" :class="{ 'is-open': showJudgeRules }">▼</span>
      </div>
      <div v-show="showJudgeRules" class="collapsible-content">
        <textarea v-model="judgeText" class="field" rows="6"></textarea>
        <div class="form-actions">
          <button class="btn" type="button" @click="saveJudgeRules">Save rules</button>
          <span class="status" v-if="judgeStatus === 'Saved'">Saved</span>
          <span class="error" v-else-if="judgeStatus">{{ judgeStatus }}</span>
        </div>
      </div>
    </div>

    <div class="section">
      <p class="eyebrow">Tasks</p>
      <p v-if="taskCount === 0" class="empty">No tasks yet. Create a mission to begin.</p>
      <ul v-else class="task-list">
        <li v-for="task in run?.tasks?.items ?? []" :key="task.id">
          <span class="task-title">{{ task.title }}</span>
          <span class="task-status" :data-status="task.status">{{ task.status }}</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.mission-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
}

.section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  user-select: none;
  padding: 4px 0;
  border-radius: 0;
  transition: all 0.2s ease;
}

.section-header:hover {
  background: rgba(var(--accent-rgb), 0.05);
}

.toggle-icon {
  font-size: 0.8rem;
  color: var(--text-secondary);
  transition: transform 0.2s ease;
}

.toggle-icon.is-open {
  transform: rotate(180deg);
}

.collapsible-content {
  display: grid;
  gap: 10px;
  animation: slideDown 0.2s ease;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.quick-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.quick-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.7rem;
  padding: 6px 10px;
}

.btn-icon {
  font-size: 0.8rem;
}

.path {
  margin: 0;
  font-size: 0.85rem;
  color: var(--text-secondary);
}

.workspace-block {
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.04);
}

.form-grid {
  display: grid;
  gap: 10px;
}

.form-grid label {
  display: grid;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--text-secondary);
  letter-spacing: 0.02em;
  font-family: var(--font-body);
}

.field {
  padding: 8px 10px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.05);
  color: var(--text-primary);
  font-size: 0.85rem;
  transition: all 0.2s ease;
}

.field:focus {
  outline: none;
  border-color: rgba(var(--accent-rgb), 0.5);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.15);
  background: rgba(var(--line-rgb), 0.06);
}

.field:hover {
  border-color: rgba(var(--accent-rgb), 0.4);
}

.btn {
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  padding: 8px 12px;
  font-size: 0.75rem;
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-primary);
  cursor: pointer;
  letter-spacing: 0.02em;
  transition: all 0.2s ease;
}

.btn:hover {
  border-color: rgba(var(--accent-rgb), 0.4);
  box-shadow: 0 8px 16px rgba(var(--accent-rgb), 0.12);
}

.btn.primary {
  background: rgba(var(--accent-rgb), 0.2);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.btn.primary:hover {
  background: rgba(var(--accent-rgb), 0.28);
}

.btn.ghost {
  background: transparent;
  border-color: rgba(var(--line-rgb), 0.18);
  color: var(--text-secondary);
}

.btn.ghost:hover {
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-primary);
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status {
  font-size: 0.65rem;
  letter-spacing: 0.04em;
  padding: 4px 10px;
  color: var(--status-success);
  border: 1px solid rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.12);
  border-radius: 999px;
}

.error {
  font-size: 0.65rem;
  letter-spacing: 0.04em;
  padding: 4px 10px;
  color: var(--status-error);
  border: 1px solid rgba(var(--status-error-rgb), 0.4);
  background: rgba(var(--status-error-rgb), 0.12);
  border-radius: 999px;
}

.task-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.task-list li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
  color: var(--text-secondary);
  font-size: 0.85rem;
  position: relative;
}

.task-list li::before {
  content: "";
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  background: linear-gradient(180deg, rgba(var(--accent-rgb), 0.9), rgba(var(--status-info-rgb), 0.2));
  opacity: 0.65;
}

.task-title {
  color: var(--text-primary);
}

.task-status {
  letter-spacing: 0.04em;
  font-size: 0.65rem;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.25);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
}

.task-status[data-status="pending"],
.task-status[data-status="todo"] {
  color: var(--text-tertiary);
  border-color: rgba(var(--line-rgb), 0.2);
  background: rgba(4, 8, 16, 0.7);
}

.task-status[data-status="running"] {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.5);
  background: rgba(var(--accent-rgb), 0.12);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.2);
}

.task-status[data-status="done"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.5);
  background: rgba(var(--status-success-rgb), 0.12);
}

.task-status[data-status="skipped"] {
  color: var(--text-tertiary);
  border-color: rgba(var(--line-rgb), 0.25);
  background: rgba(6, 10, 18, 0.7);
}

.task-status[data-status="error"] {
  color: var(--status-error);
  border-color: rgba(var(--status-error-rgb), 0.55);
  background: rgba(var(--status-error-rgb), 0.12);
}

.empty {
  color: var(--text-tertiary);
  margin: 0;
}
</style>
