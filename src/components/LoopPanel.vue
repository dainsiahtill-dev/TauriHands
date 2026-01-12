<script setup lang="ts">
import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";
import ChatPanel from "./ChatPanel.vue";
import AgentPanel from "./AgentPanel.vue";
import StatusPills from "./StatusPills.vue";

const { state, start, pause, resume, stop, userInput } = agentStore;

const run = computed(() => state.run);
const agentState = computed(() => run.value?.agentState ?? "IDLE");
const isRunning = computed(() => agentState.value === "RUNNING");
const isPaused = computed(() => agentState.value === "PAUSED");
const isAwaiting = computed(() => agentState.value === "AWAITING_USER");
const canStop = computed(() => agentState.value !== "IDLE");

const iteration = computed(() => run.value?.turn ?? 0);
const budget = computed(() => run.value?.budget);
const activeTool = computed(() => state.toolCalls.find((call) => call.status === "running"));
const stream = computed(() => state.llmStream?.content ?? "");

const pills = computed(() => [
  { label: "State", value: agentState.value, tone: agentState.value === "ERROR" ? "error" : "info" },
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
        <p class="activity-detail">{{ stream }}</p>
      </div>
      <div v-else class="activity-card muted">No activity yet.</div>
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
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
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

.loop-chat {
  min-height: 240px;
  border-radius: 12px;
  border: 1px solid var(--line);
  overflow: hidden;
}

.loop-details {
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 8px 10px;
  background: rgba(8, 12, 20, 0.6);
}

.loop-details summary {
  cursor: pointer;
  color: var(--text-secondary);
}
</style>
