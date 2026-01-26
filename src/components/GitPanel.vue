<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { agentStore } from "../agents/orchestrator";

type ToolResult = {
  ok: boolean;
  stdout_excerpt?: string | null;
  stderr_excerpt?: string | null;
  exit_code?: number | null;
};

const { state } = agentStore;
const runId = computed(() => state.run?.runId ?? "");

const loading = ref(false);
const error = ref("");
const entries = ref<Array<{ status: string; path: string }>>([]);

function parseStatus(output: string) {
  return output
    .split("\n")
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => ({
      status: line.slice(0, 2).trim() || "--",
      path: line.slice(3).trim(),
    }))
    .filter((item) => item.path);
}

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    const result = (await invoke("git_status")) as ToolResult;
    if (!result.ok) {
      error.value = result.stderr_excerpt ?? "Git status failed.";
      entries.value = [];
    } else {
      entries.value = parseStatus(result.stdout_excerpt ?? "");
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="git-panel">
    <div class="header">
      <p class="eyebrow">Repository</p>
      <button class="btn ghost" type="button" @click="refresh" :disabled="loading">
        Refresh
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
    <ul v-else-if="entries.length" class="status-list">
      <li v-for="entry in entries" :key="entry.path">
        <span class="status">{{ entry.status }}</span>
        <span class="path">{{ entry.path }}</span>
      </li>
    </ul>
    <p v-else class="empty">{{ loading ? "Loading..." : "Working tree clean." }}</p>
    <p v-if="runId" class="hint">Active run: {{ runId.slice(0, 8) }}</p>
  </div>
</template>

<style scoped>
.git-panel {
  display: grid;
  gap: 8px;
  color: var(--text-secondary);
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.status-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 8px;
}

.status-list li {
  display: grid;
  grid-template-columns: 32px 1fr;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  font-size: 0.85rem;
}

.status {
  color: var(--accent);
  text-transform: uppercase;
  font-weight: 600;
}

.path {
  color: var(--text-primary);
  word-break: break-all;
}

.error {
  margin: 0;
  color: var(--status-error);
}

.empty {
  margin: 0;
  color: var(--text-tertiary);
}

.hint {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-secondary);
}
</style>

