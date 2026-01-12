<script setup lang="ts">
import { computed, ref } from "vue";
import { agentStore } from "../agents/orchestrator";

const { state, updatePlan, updatePlanStatus } = agentStore;

const plan = computed(() => state.run?.plan ?? null);
const steps = computed(() => plan.value?.steps ?? []);

const showDialog = ref(false);
const planGoal = ref("");
const planStepsInput = ref("");
const dialogError = ref("");

function toggleStep(id: string, status: string) {
  const next = status === "done" ? "pending" : "done";
  updatePlanStatus(id, next);
}

function openDialog() {
  planGoal.value = plan.value?.goal ?? "";
  planStepsInput.value = "";
  dialogError.value = "";
  showDialog.value = true;
}

function closeDialog() {
  showDialog.value = false;
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
  const steps = parseSteps(planStepsInput.value);
  await updatePlan(goal, steps, steps.length === 0);
  closeDialog();
}
</script>

<template>
  <div class="plan-panel">
    <div class="plan-actions">
      <button class="btn primary" type="button" @click="openDialog">Generate plan</button>
    </div>
    <div v-if="!plan" class="empty">No plan yet. Generate a plan to begin.</div>
    <div v-else class="plan-content">
      <p class="goal">{{ plan.goal }}</p>
      <ul class="step-list">
        <li v-for="step in steps" :key="step.id" :data-status="step.status">
          <button class="step-toggle" type="button" @click="toggleStep(step.id, step.status)">
            {{ step.done ? "Done" : "Todo" }}
          </button>
          <span class="step-title">{{ step.title }}</span>
          <span class="step-status">{{ step.status }}</span>
        </li>
      </ul>
    </div>
    <div v-if="showDialog" class="plan-dialog-backdrop" @click.self="closeDialog">
      <div class="plan-dialog">
        <div class="dialog-header">
          <h4>Generate plan</h4>
          <button class="btn ghost" type="button" @click="closeDialog">Close</button>
        </div>
        <label class="dialog-field">
          <span>Goal</span>
          <textarea
            v-model="planGoal"
            rows="2"
            placeholder="Describe the goal to generate steps"
          ></textarea>
        </label>
        <label class="dialog-field">
          <span>Steps (optional)</span>
          <textarea
            v-model="planStepsInput"
            rows="4"
            placeholder="Optional manual steps, one per line"
          ></textarea>
        </label>
        <p v-if="dialogError" class="error-text">{{ dialogError }}</p>
        <div class="dialog-actions">
          <button class="btn ghost" type="button" @click="closeDialog">Cancel</button>
          <button class="btn primary" type="button" @click="submitPlan">Generate</button>
        </div>
      </div>
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
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.step-toggle {
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: rgba(45, 246, 255, 0.08);
  color: #2df6ff;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  cursor: pointer;
}

.step-title {
  color: var(--text-primary);
}

.step-status {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.65rem;
}

.empty {
  color: var(--text-tertiary);
  margin: 0;
}

.plan-dialog-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(3, 6, 12, 0.72);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  z-index: 30;
}

.plan-dialog {
  width: min(520px, 100%);
  background: #0a0f1b;
  border: 1px solid var(--line);
  border-radius: 16px;
  padding: 16px;
  display: grid;
  gap: 12px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.35);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dialog-header h4 {
  margin: 0;
  font-size: 0.95rem;
  color: var(--text-primary);
}

.dialog-field {
  display: grid;
  gap: 6px;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.dialog-field textarea {
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

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.error-text {
  margin: 0;
  color: #ff9b9b;
  font-size: 0.75rem;
}
</style>
