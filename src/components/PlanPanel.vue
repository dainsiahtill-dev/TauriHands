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
const showDialog = ref(false);
const selectedStepId = ref<string | null>(null);
const stepFilter = ref("all");

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

const filteredSteps = computed(() => {
  const allSteps = steps.value;
  if (stepFilter.value === "all") return allSteps;
  if (stepFilter.value === "pending") return allSteps.filter(s => s.status === "pending");
  if (stepFilter.value === "done") return allSteps.filter(s => s.status === "done");
  if (stepFilter.value === "running") return allSteps.filter(s => s.status === "running");
  return allSteps;
});

const stepStats = computed(() => {
  const total = steps.value.length;
  const done = steps.value.filter(s => s.status === "done").length;
  const pending = steps.value.filter(s => s.status === "pending").length;
  const running = steps.value.filter(s => s.status === "running").length;
  return { total, done, pending, running };
});

function toggleStep(id: string, status: string) {
  const next = status === "done" ? "pending" : "done";
  updatePlanStatus(id, next);
}

function selectStep(id: string) {
  selectedStepId.value = selectedStepId.value === id ? null : id;
}

function setStepFilter(filter: string) {
  stepFilter.value = filter;
}

function markAllDone() {
  steps.value.forEach(step => {
    if (step.status !== "done") {
      updatePlanStatus(step.id, "done");
    }
  });
}

function resetAllSteps() {
  steps.value.forEach(step => {
    if (step.status !== "pending") {
      updatePlanStatus(step.id, "pending");
    }
  });
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
  try {
    await updatePlan(goal, steps, steps.length === 0);
    planGoal.value = goal;
    planStepsInput.value = "";
    showSteps.value = false;
    showDialog.value = false;
  } catch (error) {
    dialogError.value = error instanceof Error ? error.message : String(error);
  }
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

function openDialog() {
  showDialog.value = true;
}

function closeDialog() {
  showDialog.value = false;
  dialogError.value = "";
  showSteps.value = false;
  planStepsInput.value = "";
}
</script>

<template>
  <div class="plan-panel">
    <div class="plan-header">
      <div class="plan-stats">
        <div class="stat-item">
          <span class="stat-label">Total</span>
          <span class="stat-value">{{ stepStats.total }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">Done</span>
          <span class="stat-value success">{{ stepStats.done }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">Pending</span>
          <span class="stat-value pending">{{ stepStats.pending }}</span>
        </div>
        <div class="stat-item" v-if="stepStats.running > 0">
          <span class="stat-label">Running</span>
          <span class="stat-value running">{{ stepStats.running }}</span>
        </div>
      </div>
      <div class="plan-actions">
        <button class="btn primary" type="button" @click="openDialog">Generate plan</button>
        <button class="btn" type="button" :disabled="!canExecute" @click="executePlan">
          {{ executeLabel }}
        </button>
      </div>
    </div>
    
    <div v-if="actionError" class="error-text">{{ actionError }}</div>
    
    <div v-if="!plan" class="empty-state">
      <p class="empty">No plan yet. Generate a plan to begin.</p>
      <button class="btn primary" type="button" @click="openDialog">Generate plan</button>
    </div>
    <div v-else class="plan-content">
      <div class="plan-filters">
        <button 
          v-for="filter in ['all', 'pending', 'done', 'running']" 
          :key="filter"
          class="filter-btn"
          :class="{ 'is-active': stepFilter === filter }"
          @click="setStepFilter(filter)"
        >
          {{ filter }} ({{ filter === 'all' ? stepStats.total : (stepStats as any)[filter] }})
        </button>
      </div>
      
      <div class="plan-bulk-actions">
        <button class="btn ghost small" @click="markAllDone" :disabled="stepStats.done === stepStats.total">
          Mark all done
        </button>
        <button class="btn ghost small" @click="resetAllSteps" :disabled="stepStats.pending === stepStats.total">
          Reset all
        </button>
      </div>
      
      <p class="goal">{{ plan.goal }}</p>
      <ul class="step-list">
        <li v-for="(step, index) in filteredSteps" :key="step.id" :data-status="step.status">
          <div class="step-card" :class="{ 'is-selected': selectedStepId === step.id }">
            <div class="step-main" @click="selectStep(step.id)">
              <span class="step-index">{{ String(index + 1).padStart(2, "0") }}</span>
              <div class="step-text">
                <span class="step-title">{{ step.title }}</span>
                <span class="step-status" :data-status="step.status">{{ step.status }}</span>
              </div>
            </div>
            <div class="step-actions">
              <button class="step-toggle" type="button" @click="toggleStep(step.id, step.status)">
                {{ step.status === 'done' ? '↺' : '✓' }}
              </button>
            </div>
          </div>
          <div v-if="selectedStepId === step.id" class="step-details">
            <div class="step-detail-content">
              <p><strong>Step ID:</strong> {{ step.id }}</p>
              <p><strong>Status:</strong> {{ step.status }}</p>
              <p><strong>Title:</strong> {{ step.title }}</p>
              <p v-if="(step as any).description"><strong>Description:</strong> {{ (step as any).description }}</p>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>

  <teleport to="body">
    <div v-if="showDialog" class="plan-dialog" @click.self="closeDialog">
      <div class="plan-dialog__card" role="dialog" aria-modal="true" aria-labelledby="plan-dialog-title">
        <header class="plan-dialog__header">
          <div>
            <p class="eyebrow">Plan</p>
            <h3 id="plan-dialog-title">Generate plan</h3>
          </div>
          <button class="btn ghost" type="button" @click="closeDialog">Close</button>
        </header>
        <div class="plan-dialog__body plan-editor">
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
        <footer class="plan-dialog__footer">
          <button class="btn ghost" type="button" @click="closeDialog">Cancel</button>
          <button class="btn primary" type="button" @click="submitPlan">Generate plan</button>
        </footer>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.plan-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

.plan-header {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.plan-stats {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.08);
  min-width: 60px;
}

.stat-label {
  font-size: 0.65rem;
  letter-spacing: 0.04em;
  color: var(--text-tertiary);
}

.stat-value {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}

.stat-value.success {
  color: var(--status-success);
}

.stat-value.pending {
  color: var(--text-secondary);
}

.stat-value.running {
  color: var(--accent);
}

.plan-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.plan-filters {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.filter-btn {
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.06);
  color: var(--text-secondary);
  font-size: 0.65rem;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: all 0.2s ease;
}

.filter-btn:hover {
  border-color: rgba(var(--accent-rgb), 0.4);
  color: var(--text-primary);
}

.filter-btn.is-active {
  border-color: rgba(var(--accent-rgb), 0.6);
  background: rgba(var(--accent-rgb), 0.15);
  color: var(--accent);
}

.plan-bulk-actions {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}

.btn.small {
  padding: 4px 8px;
  font-size: 0.65rem;
}

.plan-editor {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: 14px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.06);
}

.editor-field {
  display: grid;
  gap: 6px;
  font-size: 0.8rem;
  color: var(--text-secondary);
  letter-spacing: 0.02em;
  font-family: var(--font-body);
}

.editor-field textarea {
  width: 100%;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  padding: 10px 12px;
  background: rgba(var(--line-rgb), 0.05);
  color: var(--text-primary);
  font-family: var(--font-body);
  font-size: 0.8rem;
  resize: vertical;
}

.editor-field textarea:focus {
  outline: none;
  border-color: rgba(var(--accent-rgb), 0.5);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.15);
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
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.06);
  box-shadow: 0 10px 20px rgba(8, 12, 18, 0.18);
  color: var(--text-secondary);
  font-size: 0.86rem;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.2s ease;
}

.step-card:hover {
  border-color: rgba(var(--accent-rgb), 0.35);
  transform: translateY(-1px);
}

.step-card.is-selected {
  border-color: rgba(var(--accent-rgb), 0.5);
  box-shadow: 0 12px 24px rgba(8, 12, 18, 0.22);
}

.step-list li[data-status="done"] .step-card {
  border-color: rgba(var(--status-success-rgb), 0.35);
  background: rgba(var(--status-success-rgb), 0.12);
}

.step-list li[data-status="done"] .step-card::before {
  background: linear-gradient(180deg, rgba(var(--status-success-rgb), 0.95), rgba(var(--accent-rgb), 0.2));
}

.step-card::before {
  content: "";
  position: absolute;
  left: 0;
  top: 12px;
  bottom: 12px;
  width: 2px;
  border-radius: 999px;
  background: rgba(var(--accent-rgb), 0.6);
  opacity: 0.6;
}

.step-main {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  flex: 1;
}

.step-actions {
  display: flex;
  gap: 6px;
}

.step-details {
  margin-top: 8px;
  padding: 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.05);
  animation: slideDown 0.2s ease;
}

.step-detail-content {
  display: grid;
  gap: 8px;
  font-size: 0.8rem;
}

.step-detail-content p {
  margin: 0;
  color: var(--text-secondary);
}

.step-detail-content strong {
  color: var(--text-primary);
}

.step-index {
  font-size: 0.7rem;
  letter-spacing: 0.08em;
  color: var(--accent);
  padding: 4px 8px;
  border-radius: 999px;
  background: rgba(var(--accent-rgb), 0.12);
  border: 1px solid rgba(var(--accent-rgb), 0.3);
}

.step-text {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.step-toggle {
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.06);
  color: var(--text-secondary);
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.step-toggle:hover {
  background: rgba(var(--accent-rgb), 0.16);
  color: var(--accent);
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
  letter-spacing: 0.03em;
  font-size: 0.7rem;
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

.empty-state {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px dashed rgba(var(--line-rgb), 0.3);
  background: rgba(var(--line-rgb), 0.06);
}

.error-text {
  margin: 0;
  color: var(--status-error);
  font-size: 0.75rem;
}

.plan-dialog {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(5, 8, 14, 0.6);
  backdrop-filter: blur(6px);
  z-index: 40;
}

.plan-dialog__card {
  width: min(640px, 92vw);
  padding: 20px 22px;
  border-radius: 18px;
  border: 1px solid rgba(var(--line-rgb), 0.22);
  background: var(--surface, rgba(20, 24, 30, 0.9));
  box-shadow:
    0 24px 60px rgba(0, 0, 0, 0.45),
    inset 0 1px 0 rgba(255, 255, 255, 0.06);
  display: grid;
  gap: 16px;
}

.plan-dialog__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.plan-dialog__header h3 {
  margin: 4px 0 0;
  font-size: 1rem;
  letter-spacing: 0.04em;
  font-family: var(--font-display);
  color: var(--text-primary);
}

.plan-dialog__body {
  display: grid;
  gap: 10px;
}

.plan-dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
}
</style>
