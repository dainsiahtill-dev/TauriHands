<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { agentStore } from "../agents/orchestrator";

const { state, updatePlan, updatePlanStatus, start, resume, continueRun } = agentStore;

const plan = computed(() => state.run?.plan ?? null);
const steps = computed(() => plan.value?.steps ?? []);
const runState = computed(() => state.run?.agentState ?? "IDLE");

const planGoal = ref("");
const planStepsInput = ref("");
const dialogError = ref("");
const showSteps = ref(false);
const actionError = ref("");

const canExecute = computed(() => Boolean(plan.value) && runState.value !== "RUNNING");
const executeLabel = computed(() => {
  switch (runState.value) {
    case "PAUSED":
      return "Resume plan";
    case "AWAITING_USER":
      return "Continue plan";
    case "RUNNING":
      return "Running";
    default:
      return "Execute plan";
  }
});

function toggleStep(id: string, status: string) {
  const next = status === "done" ? "pending" : "done";
  updatePlanStatus(id, next);
}

function parseSteps(input: string) {
  return input
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

async function submitPlan() {
  const goal = planGoal.value.trim();
  if (!goal) {
    dialogError.value = "Goal is required.";
    return;
  }
  dialogError.value = "";
  const steps = showSteps.value ? parseSteps(planStepsInput.value) : [];
  await updatePlan(goal, steps, steps.length === 0);
  planGoal.value = goal;
  planStepsInput.value = "";
  showSteps.value = false;
}

async function executePlan() {
  if (!plan.value || runState.value === "RUNNING") return;
  actionError.value = "";
  try {
    if (runState.value === "PAUSED") {
      await resume();
    } else if (runState.value === "AWAITING_USER") {
      await continueRun();
    } else {
      await start();
    }
  } catch (error) {
    actionError.value = error instanceof Error ? error.message : String(error);
  }
}

watch(
  () => plan.value?.goal,
  (value) => {
    if (!planGoal.value && value) {
      planGoal.value = value;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="plan-panel">
    <div class="plan-actions">
      <button class="btn primary" type="button" @click="submitPlan">Generate plan</button>
      <button class="btn" type="button" :disabled="!canExecute" @click="executePlan">
        {{ executeLabel }}
      </button>
    </div>
    <div class="plan-editor">
      <label class="editor-field">
        <span>Goal</span>
        <textarea
          v-model="planGoal"
          rows="2"
          placeholder="Describe the goal to generate steps"
        ></textarea>
      </label>
      <div class="editor-actions">
        <button class="btn ghost" type="button" @click="showSteps = !showSteps">
          {{ showSteps ? "Hide steps" : "Add steps (optional)" }}
        </button>
        <span class="editor-hint">Optional: add manual steps to lock the outline.</span>
      </div>
      <label v-if="showSteps" class="editor-field">
        <span>Steps</span>
        <textarea
          v-model="planStepsInput"
          rows="4"
          placeholder="Manual steps, one per line"
        ></textarea>
      </label>
      <p v-if="dialogError" class="error-text">{{ dialogError }}</p>
    </div>
    <p v-if="actionError" class="error-text">{{ actionError }}</p>
    <div v-if="!plan" class="empty">No plan yet. Generate a plan to begin.</div>
    <div v-else class="plan-content">
      <p class="goal">{{ plan.goal }}</p>
      <ul class="step-list">
        <li v-for="(step, index) in steps" :key="step.id" :data-status="step.status">
          <div class="step-card">
            <div class="step-main">
              <span class="step-index">{{ String(index + 1).padStart(2, "0") }}</span>
              <div class="step-text">
                <span class="step-title">{{ step.title }}</span>
                <span class="step-status" :data-status="step.status">{{ step.status }}</span>
              </div>
            </div>
            <button class="step-toggle" type="button" @click="toggleStep(step.id, step.status)">
              {{ step.done ? "Done" : "Todo" }}
            </button>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.plan-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.plan-actions {
  display: flex;
  justify-content: flex-end;
}

.plan-editor {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: 14px;
  border: 1px solid rgba(var(--line-rgb), 0.3);
  background: rgba(7, 12, 22, 0.7);
}

.editor-field {
  display: grid;
  gap: 6px;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.editor-field textarea {
  width: 100%;
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: rgba(4, 10, 20, 0.8);
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
  font-size: 0.8rem;
  resize: vertical;
}

.editor-field textarea:focus {
  outline: none;
  border-color: rgba(var(--accent-rgb), 0.6);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.2);
}

.editor-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.editor-hint {
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.goal {
  margin: 0;
  font-size: 0.95rem;
  color: var(--text-primary);
}

.step-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.step-list li {
  list-style: none;
}

.step-card {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 14px;
  border-radius: 14px;
  border: 1px solid rgba(var(--accent-rgb), 0.15);
  background: linear-gradient(135deg, rgba(7, 12, 22, 0.92), rgba(14, 22, 36, 0.9));
  box-shadow: 0 8px 22px rgba(0, 0, 0, 0.25);
  color: var(--text-secondary);
  font-size: 0.86rem;
  overflow: hidden;
}

.step-card::before {
  content: "";
  position: absolute;
  left: 0;
  top: 12px;
  bottom: 12px;
  width: 3px;
  border-radius: 999px;
  background: linear-gradient(180deg, rgba(var(--accent-rgb), 0.95), rgba(var(--status-info-rgb), 0.2));
  opacity: 0.65;
}

.step-card:hover {
  border-color: rgba(var(--accent-rgb), 0.35);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35), 0 0 18px rgba(var(--accent-rgb), 0.15);
}

.step-list li[data-status="done"] .step-card {
  border-color: rgba(var(--status-success-rgb), 0.3);
  background: linear-gradient(135deg, rgba(8, 18, 14, 0.92), rgba(10, 24, 18, 0.9));
}

.step-list li[data-status="done"] .step-card::before {
  background: linear-gradient(180deg, rgba(var(--status-success-rgb), 0.95), rgba(var(--accent-rgb), 0.2));
}

.step-main {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.step-index {
  font-size: 0.75rem;
  letter-spacing: 0.2em;
  color: rgba(var(--accent-rgb), 0.7);
  text-transform: uppercase;
  padding: 6px 8px;
  border-radius: 8px;
  background: rgba(9, 14, 22, 0.8);
  border: 1px solid rgba(var(--accent-rgb), 0.2);
}

.step-text {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.step-toggle {
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--accent);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  cursor: pointer;
}

.step-title {
  color: var(--text-primary);
  font-size: 0.92rem;
  font-weight: 600;
  line-height: 1.4;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.step-status {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.65rem;
  color: var(--text-secondary);
  overflow-wrap: anywhere;
  word-break: break-word;
}

.step-status[data-status="done"] {
  color: var(--status-success);
}

.step-status[data-status="running"] {
  color: var(--accent);
}

.step-status[data-status="error"] {
  color: var(--status-warning);
}

.empty {
  color: var(--text-tertiary);
  margin: 0;
}

.error-text {
  margin: 0;
  color: var(--status-error);
  font-size: 0.75rem;
}
</style>


