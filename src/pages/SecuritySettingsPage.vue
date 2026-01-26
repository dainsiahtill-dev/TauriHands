<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { missionStore } from "../stores/mission";

const router = useRouter();
const { state: missionState, loadActive, saveActive, defaultTaskConfig } = missionStore;

const task = reactive(defaultTaskConfig());
const status = ref<"idle" | "saving" | "saved" | "error">("idle");
const message = ref("");

const workspacePath = computed(() => task.workspace || "Not set");
const taskId = computed(() => task.taskId || "Not set");

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

async function saveSecurity() {
  status.value = "saving";
  message.value = "";
  try {
    const result = await saveActive({ ...task });
    Object.assign(task, result);
    status.value = "saved";
    message.value = "Saved security policy";
  } catch (error) {
    status.value = "error";
    message.value = error instanceof Error ? error.message : String(error);
  }
}

function resetSecurity() {
  const defaults = defaultTaskConfig();
  task.riskPolicy = { ...defaults.riskPolicy };
  status.value = "idle";
  message.value = "Reset to defaults";
}

function openMission() {
  void router.push({ name: "mission" });
}
</script>

<template>
  <div class="settings-page">
    <header class="settings-header">
      <div>
        <p class="eyebrow">Security Settings</p>
        <h2>Task-level guardrails</h2>
        <p class="subtitle">These policies apply to the active task and its tool execution.</p>
      </div>
      <div class="header-actions">
        <span class="status-pill" :data-status="status">{{ message || status }}</span>
        <button class="btn ghost" type="button" @click="openMission">Open Mission</button>
        <button class="btn primary" type="button" @click="saveSecurity" :disabled="status === 'saving'">
          Save
        </button>
      </div>
    </header>

    <section class="card">
      <div class="card-head">
        <h3>Execution policy</h3>
        <span class="pill">Active task</span>
      </div>

      <div class="form-grid">
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
        <button class="btn ghost" type="button" @click="resetSecurity">Reset defaults</button>
        <div class="meta">
          <span>Task: <strong>{{ taskId }}</strong></span>
          <span>Workspace: <strong>{{ workspacePath }}</strong></span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-page {
  height: 100%;
  min-height: 0;
  padding: 16px;
  border-radius: 16px;
  background: var(--surface, #ffffff);
  border: 1px solid rgba(var(--line-rgb), 0.2);
  box-shadow: 0 16px 30px rgba(64, 158, 255, 0.16);
  overflow: auto;
  display: grid;
  gap: 16px;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  padding: 12px 16px;
  border-radius: 16px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.06);
}

.settings-header h2 {
  margin: 6px 0 8px;
  font-size: 1.2rem;
  letter-spacing: 0.02em;
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  max-width: 520px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.status-pill {
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  font-size: 0.7rem;
  letter-spacing: 0.03em;
  color: var(--text-secondary);
}

.status-pill[data-status="saved"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.35);
  background: rgba(var(--status-success-rgb), 0.12);
}

.status-pill[data-status="saving"] {
  color: var(--status-info);
  border-color: rgba(var(--status-info-rgb), 0.35);
  background: rgba(var(--status-info-rgb), 0.12);
}

.status-pill[data-status="error"] {
  color: var(--status-error);
  border-color: rgba(var(--status-error-rgb), 0.35);
  background: rgba(var(--status-error-rgb), 0.12);
}

.card {
  padding: 18px;
  border-radius: 16px;
  background: rgba(var(--line-rgb), 0.06);
  border: 1px solid rgba(var(--line-rgb), 0.18);
  display: grid;
  gap: 14px;
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.card-head h3 {
  margin: 0;
  font-size: 1rem;
  letter-spacing: 0.02em;
}

.pill {
  font-size: 0.65rem;
  letter-spacing: 0.03em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.35);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
}

.form-grid {
  display: grid;
  gap: 12px;
}

.form-grid label {
  display: grid;
  gap: 6px;
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.field {
  padding: 8px 10px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-primary);
  font-size: 0.85rem;
}

.form-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.meta strong {
  color: var(--text-primary);
}
</style>
