<script setup lang="ts">
import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";
import ChatPanel from "./ChatPanel.vue";
import AgentPanel from "./AgentPanel.vue";
import StatusPills from "./StatusPills.vue";
import StreamPreview from "./StreamPreview.vue";

const { state, start, pause, resume, stop, userInput } = agentStore;

const run = computed(() => state.run);
const agentState = computed(() => run.value?.agentState ?? "IDLE");
const isRunning = computed(() => agentState.value === "RUNNING");
const isPaused = computed(() => agentState.value === "PAUSED");
const isAwaiting = computed(() => agentState.value === "AWAITING_USER");
const canStop = computed(() => agentState.value !== "IDLE");
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

const iteration = computed(() => run.value?.turn ?? 0);
const budget = computed(() => run.value?.budget);
const activeTool = computed(() => state.toolCalls.find((call) => call.status === "running"));
const stream = computed(() => state.llmStream?.content ?? "");
const judgeResult = computed(() => state.judgeResult);
const judgeStatus = computed(() => judgeResult.value?.status ?? "none");
const judgeReasons = computed(() => judgeResult.value?.reasons ?? []);
const judgeChecks = computed(() => judgeResult.value?.checks ?? []);

const pills = computed(() => [
  {
    label: "State",
    value: agentState.value,
    tone: agentState.value === "ERROR" ? "error" : "info",
    detail: errorDetail.value,
  },
  { label: "Iter", value: String(iteration.value) },
  { label: "Budget", value: budget.value ? `${budget.value.usedSteps}/${budget.value.maxSteps}` : "-" },
]);

function togglePause() {
  if (isPaused.value) {
    resume();
  } else {
    pause();
  }
}

function continueRun() {
  userInput("继续");
}
</script>

<template>
  <div class="loop-panel">
    <div class="loop-header">
      <StatusPills :items="pills" />
      <div class="loop-actions">
        <button class="btn" type="button" @click="start" :disabled="isRunning">Start</button>
        <button v-if="isAwaiting" class="btn primary" type="button" @click="continueRun">
          Continue
        </button>
        <button v-else class="btn" type="button" @click="togglePause" :disabled="!isRunning && !isPaused">
          {{ isPaused ? "Resume" : "Pause" }}
        </button>
        <button class="btn ghost" type="button" @click="stop" :disabled="!canStop">Stop</button>
      </div>
    </div>

    <div class="loop-activity">
      <p class="eyebrow">Live activity</p>
      <div v-if="activeTool" class="activity-card">
        <p class="activity-title">{{ activeTool.tool }}</p>
        <p class="activity-detail">{{ activeTool.detail }}</p>
      </div>
      <div v-else-if="stream" class="activity-card">
        <p class="activity-title">LLM streaming</p>
        <StreamPreview :content="stream" />
      </div>
      <div v-else class="activity-card muted">No activity yet.</div>
    </div>

    <div class="loop-judge">
      <div class="judge-header">
        <p class="eyebrow">Judge</p>
        <span class="judge-chip" :data-status="judgeStatus">{{ judgeStatus }}</span>
      </div>
      <div v-if="judgeResult" class="judge-card">
        <p v-if="judgeReasons.length" class="judge-reasons">
          {{ judgeReasons.join(" | ") }}
        </p>
        <div v-if="judgeChecks.length" class="judge-checks">
          <details v-for="check in judgeChecks" :key="check.id" class="judge-check">
            <summary>
              <span class="judge-check-type">{{ check.type }}</span>
              <span class="judge-check-status" :data-status="check.status">{{ check.status }}</span>
              <span v-if="check.reason" class="judge-check-reason">{{ check.reason }}</span>
            </summary>
            <div v-if="check.evidence && check.evidence.length" class="judge-evidence">
              <pre v-for="(item, index) in check.evidence" :key="index">{{ item }}</pre>
            </div>
            <p v-else class="judge-empty">No evidence.</p>
          </details>
        </div>
        <p v-else class="judge-empty">No checks executed yet.</p>
      </div>
      <div v-else class="judge-card muted">No judge result yet.</div>
    </div>

    <div class="loop-chat">
      <ChatPanel />
    </div>

    <details class="loop-details">
      <summary>Execution details</summary>
      <AgentPanel :show-header="false" />
    </details>
  </div>
</template>

<style scoped>
.loop-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.loop-header {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.loop-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.loop-activity {
  display: grid;
  gap: 8px;
}

.activity-card {
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.36);
  background: linear-gradient(140deg, rgba(9, 16, 30, 0.85), rgba(7, 12, 22, 0.78));
  box-shadow: inset 0 0 14px rgba(var(--accent-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.activity-card.muted {
  color: var(--text-tertiary);
}

.activity-title {
  margin: 0 0 4px;
  color: var(--text-primary);
  font-weight: 600;
}

.activity-detail {
  margin: 0;
  word-break: break-word;
}

.loop-judge {
  display: grid;
  gap: 8px;
}

.judge-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.judge-chip {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--text-secondary-rgb), 0.3);
  color: var(--text-secondary);
  background: rgba(var(--text-secondary-rgb), 0.12);
}

.judge-chip[data-status="pass"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.12);
}

.judge-chip[data-status="fail"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.4);
  background: rgba(var(--status-warning-rgb), 0.12);
}

.judge-chip[data-status="pending"] {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.35);
  background: rgba(var(--accent-rgb), 0.12);
}

.judge-card {
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.36);
  background: linear-gradient(140deg, rgba(9, 16, 30, 0.85), rgba(7, 12, 22, 0.78));
  box-shadow: inset 0 0 14px rgba(var(--accent-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.85rem;
  display: grid;
  gap: 8px;
}

.judge-card.muted {
  color: var(--text-tertiary);
}

.judge-reasons {
  margin: 0;
  color: var(--status-warning);
  font-size: 0.8rem;
  word-break: break-word;
}

.judge-checks {
  display: grid;
  gap: 8px;
}

.judge-check {
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.3);
  background: rgba(5, 8, 14, 0.7);
  overflow: hidden;
}

.judge-check summary {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: var(--text-secondary);
  list-style: none;
}

.judge-check summary::-webkit-details-marker {
  display: none;
}

.judge-check-type {
  color: var(--accent);
}

.judge-check-status {
  padding: 2px 6px;
  border-radius: 999px;
  border: 1px solid var(--line);
  font-size: 0.6rem;
  color: var(--text-soft);
}

.judge-check-status[data-status="pass"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.12);
}

.judge-check-status[data-status="fail"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.4);
  background: rgba(var(--status-warning-rgb), 0.12);
}

.judge-check-status[data-status="pending"] {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.35);
  background: rgba(var(--accent-rgb), 0.12);
}

.judge-check-reason {
  text-transform: none;
  letter-spacing: 0;
  color: var(--text-secondary);
  font-size: 0.7rem;
}

.judge-evidence {
  display: grid;
  gap: 6px;
  padding: 8px 10px 10px;
  border-top: 1px solid var(--line);
}

.judge-evidence pre {
  margin: 0;
  font-size: 0.72rem;
  color: var(--text-soft);
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  max-height: 160px;
  overflow: auto;
  background: rgba(5, 8, 14, 0.75);
  border: 1px solid rgba(var(--line-rgb), 0.3);
  border-radius: 8px;
  padding: 8px;
}

.judge-empty {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.loop-chat {
  min-height: 260px;
}

.loop-details {
  border: 1px solid rgba(var(--line-rgb), 0.3);
  border-radius: 12px;
  padding: 8px 10px;
  background: rgba(8, 12, 20, 0.7);
}

.loop-details summary {
  cursor: pointer;
  color: var(--text-secondary);
}
</style>


