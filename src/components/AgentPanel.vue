<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { agentStore } from "../agents/orchestrator";
import AgentOverview from "./agent/AgentOverview.vue";
import PlanManager from "./agent/PlanManager.vue";
import TaskList from "./agent/TaskList.vue";
import LLMLive from "./agent/LLMLive.vue";
import ToolCallList from "./agent/ToolCallList.vue";
import LogViewer from "./agent/LogViewer.vue";

const {
  state,
  initKernelStore,
  start,
  pause,
  resume,
  stop,
  userInput,
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
const canStop = computed(
  () => agentState.value === "RUNNING" || agentState.value === "PAUSED" || isAwaiting.value,
);
const planSteps = computed(() => run.value?.plan?.steps ?? []);
const tasks = computed(() => run.value?.tasks?.items ?? []);
const toolCalls = computed(() => state.toolCalls);
const logs = computed(() => state.logs);
const llmMessages = computed(() => run.value?.messages ?? []);
const activeToolCall = computed(() => toolCalls.value.find((call) => call.status === "running"));
const activeToolOutput = computed(() => {
  const call = activeToolCall.value;
  if (!call) return "";
  return state.toolOutputs?.[call.id] ?? "";
});
const streamContent = computed(() => state.llmStream?.content ?? "");
const isStreaming = computed(() => state.llmStream?.active && !!streamContent.value);
const isThinking = computed(
  () => agentState.value === "RUNNING" && !activeToolCall.value && !isStreaming.value,
);
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
  showHeader?: boolean;
}>();

const showHeader = computed(() => props.showHeader !== false);

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

function togglePause() {
  if (isPaused.value) {
    resume();
  } else {
    pause();
  }
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
    <div v-if="showHeader" class="panel-header">
      <div>
        <p class="eyebrow">Agent</p>
        <h3>Run controller</h3>
      </div>
      <div class="header-actions">
        <button class="btn ghost" type="button" @click="emit('toggle-detach')">
          {{ props.isDetached ? "Dock" : "Detach" }}
        </button>
        <button class="btn" type="button" @click="start" :disabled="isRunning">Start</button>
        <button v-if="isAwaiting" class="btn primary" type="button" @click="userInput('继续')">
          Continue
        </button>
        <button v-else class="btn" type="button" @click="togglePause" :disabled="!canTogglePause">
          {{ isPaused ? "Resume" : "Pause" }}
        </button>
        <button class="btn" type="button" @click="stop" :disabled="!canStop">Stop</button>
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
      <AgentOverview
        :agent-state="agentState"
        :budget-label="budgetLabel"
        :summary="summary"
        :active-tool-call="activeToolCall"
        :active-tool-output="activeToolOutput"
        :stream-content="streamContent"
        :is-streaming="isStreaming"
        :is-thinking="isThinking"
        :last-assistant-message="lastAssistantMessage"
        :last-message="lastMessage"
        :is-awaiting="isAwaiting"
        :show-error="showError"
        :error-message="errorMessage"
      />
    </div>

    <div v-show="activeTab === 'plan'" class="section">
      <PlanManager
        :plan-steps="planSteps"
        :plan-goal="planGoal"
        @updatePlan="updatePlan"
        @updatePlanStatus="updatePlanStatus"
        @update:planGoal="planGoal = $event"
      />
    </div>

    <div v-show="activeTab === 'tasks'" class="section">
      <TaskList :tasks="tasks" />
    </div>

    <div v-show="activeTab === 'llm'" class="section">
      <LLMLive
        :agent-state="agentState"
        :run="run"
        :active-tool-call="activeToolCall"
        :active-tool-output="activeToolOutput"
        :stream-content="streamContent"
        :is-streaming="isStreaming"
        :is-thinking="isThinking"
        :last-assistant-message="lastAssistantMessage"
        :last-message="lastMessage"
        :llm-messages="llmMessages"
      />
    </div>

    <div v-show="activeTab === 'tools'" class="section">
      <ToolCallList :tool-calls="toolCalls" />
    </div>

    <div v-show="activeTab === 'logs'" class="section">
      <LogViewer :logs="logs" :latest-error-id="latestErrorId" />
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
  font-family: var(--font-display);
  text-transform: uppercase;
  letter-spacing: 0.2em;
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
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  padding: 6px 12px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  background: rgba(4, 12, 22, 0.75);
  color: var(--text-secondary);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  clip-path: polygon(
    var(--hud-cut-xs) 0,
    calc(100% - var(--hud-cut-xs)) 0,
    100% var(--hud-cut-xs),
    100% calc(100% - var(--hud-cut-xs)),
    calc(100% - var(--hud-cut-xs)) 100%,
    var(--hud-cut-xs) 100%,
    0 calc(100% - var(--hud-cut-xs)),
    0 var(--hud-cut-xs)
  );
}

.tab.active {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.55);
  background: rgba(var(--accent-rgb), 0.12);
}

.tab-count {
  font-size: 0.6rem;
  padding: 2px 6px;
  border-radius: 0;
  background: rgba(8, 12, 20, 0.85);
  border: 1px solid rgba(var(--line-rgb), 0.4);
  color: var(--text-soft);
  clip-path: polygon(
    var(--hud-cut-xs) 0,
    calc(100% - var(--hud-cut-xs)) 0,
    100% var(--hud-cut-xs),
    100% calc(100% - var(--hud-cut-xs)),
    calc(100% - var(--hud-cut-xs)) 100%,
    var(--hud-cut-xs) 100%,
    0 calc(100% - var(--hud-cut-xs)),
    0 var(--hud-cut-xs)
  );
}

.section {
  display: grid;
  gap: 12px;
}

.btn {
  border-radius: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.5);
  padding: 8px 12px;
  font-size: 0.7rem;
  background: linear-gradient(135deg, rgba(3, 12, 24, 0.95), rgba(2, 8, 16, 0.85));
  color: var(--text-primary);
  cursor: pointer;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  clip-path: polygon(
    var(--hud-cut-xs) 0,
    calc(100% - var(--hud-cut-xs)) 0,
    100% var(--hud-cut-xs),
    100% calc(100% - var(--hud-cut-xs)),
    calc(100% - var(--hud-cut-xs)) 100%,
    var(--hud-cut-xs) 100%,
    0 calc(100% - var(--hud-cut-xs)),
    0 var(--hud-cut-xs)
  );
}

.btn.primary {
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.95), rgba(var(--status-info-rgb), 0.8));
  color: var(--bg);
  border-color: transparent;
  box-shadow: 0 0 18px rgba(var(--accent-rgb), 0.4);
}

.btn.ghost:hover {
  border-color: rgba(var(--accent-rgb), 0.5);
  color: var(--accent);
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: var(--status-success);
  font-family: var(--font-display);
}
</style>
