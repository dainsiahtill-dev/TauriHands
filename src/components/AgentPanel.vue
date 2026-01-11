<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { agentStore } from "../agents/orchestrator";

const {
  state,
  initKernelStore,
  start,
  pause,
  resume,
  reset,
  updatePlan,
  updatePlanStatus,
} = agentStore;

const run = computed(() => state.run);
const agentState = computed(() => run.value?.agentState ?? "IDLE");
const isRunning = computed(() => agentState.value === "RUNNING");
const isPaused = computed(() => agentState.value === "PAUSED");
const isAwaiting = computed(() => agentState.value === "AWAITING_USER");
const canTogglePause = computed(() => isRunning.value || isPaused.value);
const planSteps = computed(() => run.value?.plan?.steps ?? []);
const tasks = computed(() => run.value?.tasks?.items ?? []);
const toolCalls = computed(() => state.toolCalls);
const logs = computed(() => state.logs);
const llmMessages = computed(() => run.value?.messages ?? []);
const visibleMessages = computed(() => llmMessages.value.slice(-20));
const activeToolCall = computed(() => toolCalls.value.find((call) => call.status === "running"));
const activeToolOutput = computed(() => {
  const call = activeToolCall.value;
  if (!call) return "";
  return state.toolOutputs?.[call.id] ?? "";
});
const isThinking = computed(() => agentState.value === "RUNNING" && !activeToolCall.value);
const latestErrorId = computed(() => logs.value.find((log) => log.level === "error")?.id ?? "");
const lastError = computed(() => run.value?.lastError ?? "");
const showError = computed(() => agentState.value === "ERROR" || !!lastError.value);
const errorMessage = computed(() => {
  const trimmed = lastError.value.trim();
  return trimmed || "Unknown error. Check logs for details.";
});
const budgetLabel = computed(() => {
  const budget = run.value?.budget;
  if (!budget) return "-";
  return `${budget.usedSteps}/${budget.maxSteps}`;
});

const planInput = ref("");
const planGoal = ref("");
const activeTab = ref("overview");

const tabs = [
  { id: "overview", label: "Overview" },
  { id: "plan", label: "Plan" },
  { id: "tasks", label: "Tasks" },
  { id: "llm", label: "LLM" },
  { id: "tools", label: "Tools" },
  { id: "logs", label: "Logs" },
];

const summary = computed(() => ({
  steps: planSteps.value.length,
  tasks: tasks.value.length,
  tools: toolCalls.value.length,
  logs: logs.value.length,
  messages: llmMessages.value.length,
}));

const emit = defineEmits<{
  (e: "toggle-detach"): void;
}>();

const props = defineProps<{
  isDetached?: boolean;
}>();

function togglePause() {
  if (isPaused.value) {
    resume();
  } else {
    pause();
  }
}

function focusChatInput() {
  window.dispatchEvent(new Event("focus-chat-input"));
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}

function parsePlanInput(input: string) {
  return input
    .split(/[\n;]+/)
    .map((line) => line.replace(/^[\s\-*\d\.\)\]]+/, "").trim())
    .filter(Boolean);
}

function tabCount(id: string) {
  switch (id) {
    case "plan":
      return summary.value.steps;
    case "tasks":
      return summary.value.tasks;
    case "llm":
      return summary.value.messages;
    case "tools":
      return summary.value.tools;
    case "logs":
      return summary.value.logs;
    default:
      return 0;
  }
}

const lastMessage = computed(() => {
  const list = llmMessages.value;
  if (!list.length) return "";
  return list[list.length - 1].content ?? "";
});

const lastAssistantMessage = computed(() => {
  const list = llmMessages.value;
  for (let i = list.length - 1; i >= 0; i -= 1) {
    if (list[i].role === "assistant") {
      return list[i].content ?? "";
    }
  }
  return "";
});

async function addPlan() {
  const items = parsePlanInput(planInput.value);
  if (items.length === 0) return;
  const goal = planGoal.value.trim();
  if (!goal) return;
  const current = planSteps.value.map((step) => step.title);
  await updatePlan(goal, [...current, ...items]);
  planInput.value = "";
}

async function removePlanItem(id: string) {
  const goal = planGoal.value.trim();
  if (!goal) return;
  const next = planSteps.value.filter((step) => step.id !== id).map((step) => step.title);
  await updatePlan(goal, next);
}

async function skipPlanItem(id: string) {
  await updatePlanStatus(id, "skipped");
}

async function retryPlanItem(id: string) {
  await updatePlanStatus(id, "pending");
}

async function clearPlanItems() {
  const goal = planGoal.value.trim();
  if (!goal) return;
  await updatePlan(goal, [], false);
}

async function generatePlanFromGoal() {
  const trimmed = planGoal.value.trim();
  if (!trimmed) return;
  await updatePlan(trimmed, [], true);
}

onMounted(async () => {
  await initKernelStore();
});

watch(
  () => run.value?.plan?.goal,
  (value) => {
    if (!value) {
      planGoal.value = "";
      return;
    }
    if (value !== planGoal.value) {
      planGoal.value = value;
    }
  },
);
</script>

<template>
  <div class="agent-panel">
    <div class="panel-header">
      <div>
        <p class="eyebrow">Agent</p>
        <h3>Run controller</h3>
      </div>
      <div class="header-actions">
        <button class="btn ghost" type="button" @click="emit('toggle-detach')">
          {{ props.isDetached ? "Dock" : "Detach" }}
        </button>
        <button class="btn" type="button" @click="start" :disabled="isRunning">Start</button>
        <button v-if="isAwaiting" class="btn primary" type="button" @click="focusChatInput">
          Continue
        </button>
        <button v-else class="btn" type="button" @click="togglePause" :disabled="!canTogglePause">
          {{ isPaused ? "Resume" : "Pause" }}
        </button>
        <button class="btn" type="button" @click="reset" :disabled="isRunning">Reset</button>
      </div>
    </div>

    <div class="tab-row">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        class="tab"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >
        <span>{{ tab.label }}</span>
        <span v-if="tab.id !== 'overview'" class="tab-count">{{ tabCount(tab.id) }}</span>
      </button>
    </div>

    <div v-show="activeTab === 'overview'" class="section">
      <div class="section-title">Status</div>
      <div class="controls">
        <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
        <span class="budget-chip">Steps {{ budgetLabel }}</span>
      </div>
      <div class="summary-grid">
        <div class="summary-card">
          <span class="summary-label">Plan steps</span>
          <strong>{{ summary.steps }}</strong>
        </div>
        <div class="summary-card">
          <span class="summary-label">Tasks</span>
          <strong>{{ summary.tasks }}</strong>
        </div>
        <div class="summary-card">
          <span class="summary-label">Tool calls</span>
          <strong>{{ summary.tools }}</strong>
        </div>
        <div class="summary-card">
          <span class="summary-label">Logs</span>
          <strong>{{ summary.logs }}</strong>
        </div>
      </div>
      <div class="llm-preview">
        <div class="llm-preview-header">
          <span class="summary-label">Live activity</span>
          <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
        </div>
        <div v-if="activeToolCall" class="llm-preview-block">
          <span class="llm-preview-title">Running tool</span>
          <p class="llm-preview-text">
            {{ activeToolCall.tool }} · {{ activeToolCall.detail }}
          </p>
          <pre v-if="activeToolOutput" class="llm-preview-output">{{ activeToolOutput }}</pre>
        </div>
        <p v-else-if="isThinking" class="llm-preview-text">LLM is selecting the next action...</p>
        <p v-else class="llm-preview-text">
          {{ lastAssistantMessage || lastMessage || "No messages yet." }}
        </p>
      </div>
      <p v-if="isAwaiting" class="awaiting-hint">Waiting for input in chat to continue.</p>
      <div v-if="showError" class="error-card">
        <div class="error-title">Last error</div>
        <pre class="error-detail">{{ errorMessage }}</pre>
      </div>
    </div>

    <div v-show="activeTab === 'plan'" class="section">
      <div class="section-title">Planner</div>
      <div class="plan-builder">
        <label class="plan-goal">
          <span>Goal</span>
          <textarea
            v-model="planGoal"
            rows="2"
            placeholder="Describe the goal to auto-generate plan steps"
          ></textarea>
        </label>
        <div class="plan-actions">
          <button class="btn" type="button" @click="generatePlanFromGoal">
            Generate plan
          </button>
          <button class="btn ghost" type="button" @click="clearPlanItems">Clear</button>
        </div>
        <div v-if="planSteps.length" class="plan-list">
          <div v-for="item in planSteps" :key="item.id" class="plan-item">
            <span class="plan-text">{{ item.title }}</span>
            <span class="plan-status" :data-status="item.status">{{ item.status }}</span>
            <div class="plan-item-actions">
              <button
                v-if="item.status !== 'skipped'"
                class="btn ghost"
                type="button"
                @click="skipPlanItem(item.id)"
              >
                Skip
              </button>
              <button
                v-if="item.status === 'skipped' || item.status === 'error'"
                class="btn ghost"
                type="button"
                @click="retryPlanItem(item.id)"
              >
                Retry
              </button>
              <button class="btn ghost" type="button" @click="removePlanItem(item.id)">
                Remove
              </button>
            </div>
          </div>
        </div>
        <textarea
          v-model="planInput"
          rows="3"
          placeholder="Add plan steps (term:/run:/read:/search:/test: prefixes supported)"
        ></textarea>
        <div class="plan-actions">
          <button class="btn" type="button" @click="addPlan">Add items</button>
        </div>
      </div>
    </div>

    <div v-show="activeTab === 'tasks'" class="section">
      <div class="section-title">Tasks</div>
      <div v-if="tasks.length" class="plan-list">
        <div v-for="task in tasks" :key="task.id" class="plan-item">
          <span class="plan-text">{{ task.title }}</span>
          <span class="plan-status" :data-status="task.status">{{ task.status }}</span>
        </div>
      </div>
      <p v-else class="empty-text">No tasks generated yet.</p>
    </div>

    <div v-show="activeTab === 'llm'" class="section">
      <div class="section-title">LLM live</div>
      <div class="controls">
        <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
        <span class="budget-chip">Turn {{ run?.turn ?? 0 }}</span>
      </div>
      <div class="llm-preview">
        <div class="llm-preview-header">
          <span class="summary-label">Live activity</span>
          <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
        </div>
        <div v-if="activeToolCall" class="llm-preview-block">
          <span class="llm-preview-title">Running tool</span>
          <p class="llm-preview-text">
            {{ activeToolCall.tool }} · {{ activeToolCall.detail }}
          </p>
          <pre v-if="activeToolOutput" class="llm-preview-output">{{ activeToolOutput }}</pre>
        </div>
        <p v-else-if="isThinking" class="llm-preview-text">LLM is selecting the next action...</p>
        <p v-else class="llm-preview-text">
          {{ lastAssistantMessage || lastMessage || "No messages yet." }}
        </p>
      </div>
      <div class="llm-messages">
        <div
          v-for="(message, index) in visibleMessages"
          :key="`llm-${index}`"
          class="llm-message"
          :data-role="message.role"
        >
          <span class="llm-role">{{ message.role }}</span>
          <p>{{ message.content }}</p>
        </div>
        <p v-if="!visibleMessages.length" class="empty-text">No messages yet.</p>
      </div>
    </div>

    <div v-show="activeTab === 'tools'" class="section">
      <div class="section-title">Tool calls</div>
      <div class="tool-call" v-for="call in toolCalls" :key="call.id">
        <div>
          <strong>{{ call.tool }}</strong>
          <p>{{ call.detail }}</p>
          <p v-if="call.summary || call.status === 'error'" class="tool-summary">
            {{ call.summary || "Error without details." }}
          </p>
        </div>
        <span class="chip" :data-status="call.status">{{ call.status }}</span>
      </div>
      <p v-if="!toolCalls.length" class="empty-text">No tool calls yet.</p>
    </div>

    <div v-show="activeTab === 'logs'" class="section">
      <div class="section-title">Logs</div>
      <div class="logs">
        <div
          v-for="log in logs"
          :key="log.id"
          class="log-row"
          :data-level="log.level"
          :data-latest="log.id === latestErrorId"
        >
          <span class="log-time">{{ formatTime(log.timestamp) }}</span>
          <span class="log-level">{{ log.level }}</span>
          <span class="log-message">{{ log.message }}</span>
        </div>
        <p v-if="!logs.length" class="empty-text">No logs yet.</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.agent-panel {
  display: flex;
  flex-direction: column;
  gap: 18px;
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 12px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.panel-header h3 {
  margin: 0;
  font-size: 1.2rem;
}

.header-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.tab-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.tab {
  border-radius: 999px;
  border: 1px solid var(--line);
  padding: 6px 12px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  background: var(--panel-glass);
  color: #9bb0c6;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.tab.active {
  color: #2df6ff;
  border-color: rgba(45, 246, 255, 0.55);
  background: rgba(45, 246, 255, 0.12);
}

.tab-count {
  font-size: 0.6rem;
  padding: 2px 6px;
  border-radius: 999px;
  background: rgba(8, 12, 20, 0.8);
  border: 1px solid var(--line);
  color: #c7d7ec;
}

.section {
  display: grid;
  gap: 12px;
}

.section-title {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: #9bb0c6;
}

.controls {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  align-items: center;
}

.summary-grid {
  display: grid;
  gap: 10px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.summary-card {
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: rgba(8, 12, 20, 0.7);
  display: grid;
  gap: 6px;
}

.summary-label {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: #8aa0b7;
}

.summary-card strong {
  font-size: 1rem;
  color: #e6f3ff;
}

.llm-preview {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
}

.llm-preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.llm-preview-text {
  margin: 0;
  font-size: 0.8rem;
  color: #cfe6ff;
  white-space: pre-wrap;
  max-height: 120px;
  overflow: auto;
}

.llm-preview-block {
  display: grid;
  gap: 6px;
}

.llm-preview-title {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: #8aa0b7;
}

.llm-preview-output {
  margin: 0;
  border-radius: 10px;
  border: 1px solid var(--line);
  padding: 8px 10px;
  background: rgba(5, 8, 14, 0.7);
  color: #c7d7ec;
  font-size: 0.7rem;
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  max-height: 140px;
  overflow: auto;
}

.llm-messages {
  display: grid;
  gap: 10px;
  max-height: 360px;
  overflow: auto;
}

.llm-message {
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: rgba(8, 12, 20, 0.8);
  display: grid;
  gap: 6px;
  color: #c7d7ec;
}

.llm-message[data-role="assistant"] {
  border-color: rgba(182, 255, 75, 0.35);
}

.llm-message[data-role="user"] {
  border-color: rgba(45, 246, 255, 0.35);
}

.llm-message[data-role="system"] {
  border-color: rgba(138, 160, 183, 0.35);
}

.llm-role {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  color: #8aa0b7;
}

.llm-message p {
  margin: 0;
  font-size: 0.8rem;
  white-space: pre-wrap;
}

.awaiting-hint {
  margin: 0;
  font-size: 0.75rem;
  color: #b6ff4b;
}

.error-card {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(255, 184, 77, 0.4);
  background: rgba(255, 184, 77, 0.08);
  display: grid;
  gap: 6px;
}

.error-title {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: #ffb84d;
}

.error-detail {
  margin: 0;
  max-height: 160px;
  overflow: auto;
  font-size: 0.75rem;
  white-space: pre-wrap;
  color: #f2d4a0;
  font-family: "JetBrains Mono", monospace;
}

.plan-builder {
  display: grid;
  gap: 12px;
}

.plan-goal textarea,
.plan-builder textarea {
  width: 100%;
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: var(--panel-glass);
  color: #e6f3ff;
}

.plan-actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.plan-list {
  display: grid;
  gap: 10px;
}

.plan-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(9, 14, 22, 0.8);
  border: 1px solid var(--line);
}

.plan-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plan-status {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #8aa0b7;
}

.plan-item-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.tool-call {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.8);
}

.tool-summary {
  margin: 6px 0 0;
  font-size: 0.75rem;
  color: #9bb0c6;
  white-space: pre-wrap;
}

.logs {
  display: grid;
  gap: 8px;
}

.log-row {
  display: grid;
  grid-template-columns: auto auto 1fr;
  gap: 8px;
  align-items: center;
  padding: 6px 10px;
  border-radius: 10px;
  background: rgba(8, 12, 20, 0.8);
  border: 1px solid var(--line);
  font-size: 0.75rem;
}

.log-row[data-latest="true"] {
  border-color: rgba(255, 184, 77, 0.6);
  box-shadow: 0 0 16px rgba(255, 184, 77, 0.15);
}

.log-time {
  color: #9bb0c6;
}

.log-level {
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 0.6rem;
}

.log-message {
  color: #e6f3ff;
}

.phase-chip,
.budget-chip {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #c7d7ec;
  background: rgba(8, 12, 20, 0.8);
}

.chip {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: #c7d7ec;
}

.btn {
  border-radius: 10px;
  border: 1px solid var(--line);
  padding: 8px 10px;
  font-size: 0.7rem;
  background: var(--panel-glass);
  color: #9bb0c6;
  cursor: pointer;
}

.btn.primary {
  background: linear-gradient(135deg, rgba(45, 246, 255, 0.9), rgba(74, 125, 255, 0.9));
  color: #05060a;
  border-color: transparent;
  box-shadow: 0 0 18px rgba(45, 246, 255, 0.4);
}

.btn.ghost:hover {
  border-color: rgba(45, 246, 255, 0.5);
  color: var(--accent);
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: #b6ff4b;
}

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: #8aa0b7;
}
</style>
