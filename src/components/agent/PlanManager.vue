<script setup lang="ts">
import { ref } from "vue";

interface PlanStep {
  id: string;
  title: string;
  status: string;
}

interface Props {
  planSteps: PlanStep[];
  planGoal: string;
}

interface Emits {
  (e: "updatePlan", goal: string, steps: string[], generate?: boolean): void;
  (e: "updatePlanStatus", id: string, status: string): void;
  (e: "update:planGoal", value: string): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const planInput = ref("");

function parsePlanInput(input: string) {
  return input
    .split(/[\n;]+/)
    .map((line) => line.replace(/^[\s\-*\d\.\)\]]+/, "").trim())
    .filter(Boolean);
}

async function addPlan() {
  const items = parsePlanInput(planInput.value);
  if (items.length === 0) return;
  const goal = props.planGoal.trim();
  if (!goal) return;
  const current = props.planSteps.map((step) => step.title);
  await emit("updatePlan", goal, [...current, ...items]);
  planInput.value = "";
}

async function removePlanItem(id: string) {
  const goal = props.planGoal.trim();
  if (!goal) return;
  const next = props.planSteps.filter((step) => step.id !== id).map((step) => step.title);
  await emit("updatePlan", goal, next);
}

async function skipPlanItem(id: string) {
  await emit("updatePlanStatus", id, "skipped");
}

async function retryPlanItem(id: string) {
  await emit("updatePlanStatus", id, "pending");
}

async function clearPlanItems() {
  const goal = props.planGoal.trim();
  if (!goal) return;
  await emit("updatePlan", goal, [], false);
}

async function generatePlanFromGoal() {
  const trimmed = props.planGoal.trim();
  if (!trimmed) return;
  await emit("updatePlan", trimmed, [], true);
}
</script>

<template>
  <div class="plan-manager">
    <div class="section-title">Planner</div>
    <div class="plan-builder">
      <label class="plan-goal">
        <span>Goal</span>
        <textarea
          :value="planGoal"
          @input="$emit('update:planGoal', ($event.target as HTMLTextAreaElement).value)"
          rows="2"
          placeholder="Describe goal to auto-generate plan steps"
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
</template>

<style scoped>
.plan-manager {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: var(--text-secondary);
  font-family: var(--font-display);
}

.plan-builder {
  display: grid;
  gap: 12px;
}

.plan-goal textarea,
.plan-builder textarea {
  width: 100%;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  padding: 10px 12px;
  background: rgba(4, 12, 22, 0.8);
  color: var(--text-primary);
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
  border-radius: 0;
  background: rgba(9, 14, 22, 0.85);
  border: 1px solid rgba(var(--line-rgb), 0.35);
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
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.35);
  color: var(--text-secondary);
  background: rgba(8, 12, 20, 0.75);
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

.plan-item-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
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

.btn.ghost:hover {
  border-color: rgba(var(--accent-rgb), 0.5);
  color: var(--accent);
}
</style>
