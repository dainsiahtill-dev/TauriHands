<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { agentStore } from "../agents/orchestrator";

type ToolResult = {
  ok: boolean;
  stdout_excerpt?: string | null;
  stderr_excerpt?: string | null;
};

const { state } = agentStore;
const lastError = computed(() => state.run?.lastError ?? "");

const loading = ref(false);
const error = ref("");
const diffText = ref("");

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    const result = (await invoke("git_diff", { request: { path: null } })) as ToolResult;
    if (!result.ok) {
      error.value = result.stderr_excerpt ?? "Git diff failed.";
      diffText.value = "";
    } else {
      diffText.value = result.stdout_excerpt ?? "";
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    diffText.value = "";
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="diff-panel">
    <div class="header">
      <p class="eyebrow">Patch view</p>
      <button class="btn ghost" type="button" @click="refresh" :disabled="loading">
        Refresh
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>
    <pre v-else-if="diffText" class="diff-output">{{ diffText }}</pre>
    <p v-else class="empty">{{ loading ? "Loading..." : "No diff output." }}</p>
    <p v-if="lastError" class="hint">Last error: {{ lastError }}</p>
  </div>
</template>

<style scoped>
.diff-panel {
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

.diff-output {
  margin: 0;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
  color: var(--text-primary);
  font-size: 0.8rem;
  white-space: pre-wrap;
  max-height: 320px;
  overflow: auto;
}

.empty {
  margin: 0;
  color: var(--text-tertiary);
}

.error {
  margin: 0;
  color: var(--status-error);
}

.hint {
  margin: 0;
  font-size: 0.8rem;
  color: var(--status-warning);
}
</style>

