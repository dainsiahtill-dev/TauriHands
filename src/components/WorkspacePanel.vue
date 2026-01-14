<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import WorkspaceTreeNode from "./WorkspaceTreeNode.vue";

type TreeItem = {
  type: "folder" | "file";
  name: string;
  path: string;
  children?: TreeItem[];
};

const workspacePathInput = ref("");
const workspaceError = ref("");
const workspaceLabel = computed(() => {
  if (!workspacePathInput.value) return "Workspace";
  const trimmed = workspacePathInput.value.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]/);
  return parts[parts.length - 1] || trimmed;
});

const treeItems = ref<TreeItem[]>([]);
const treeError = ref("");

async function loadWorkspace() {
  try {
    const root = (await invoke("get_workspace_root")) as string;
    workspacePathInput.value = normalizeDisplayPath(root);
    await loadTree();
  } catch (error) {
    const message = extractErrorMessage(error);
    workspaceError.value = message || "Unable to load workspace root.";
  }
}

async function applyWorkspace() {
  const value = normalizeInputPath(workspacePathInput.value);
  if (!value) {
    workspaceError.value = "Workspace path is required.";
    return;
  }
  workspacePathInput.value = value;
  workspaceError.value = "";
  try {
    const root = (await invoke("set_workspace_root", { root: value })) as string;
    workspacePathInput.value = normalizeDisplayPath(root);
    await loadTree();
  } catch (error) {
    const message = extractErrorMessage(error);
    workspaceError.value = message || "Invalid workspace path.";
  }
}

async function loadTree() {
  treeError.value = "";
  try {
    const items = (await invoke("fs_list_tree", {
      max_depth: 4,
      max_entries: 2000,
      show_hidden: false,
    })) as TreeItem[];
    treeItems.value = items;
  } catch (error) {
    const message = extractErrorMessage(error);
    treeError.value = message || "Unable to load workspace tree.";
  }
}

async function browseWorkspace() {
  workspaceError.value = "";
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select workspace folder",
    });
    if (!selected) return;
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;
    workspacePathInput.value = normalizeDisplayPath(path);
    await applyWorkspace();
  } catch (error) {
    const message = extractErrorMessage(error);
    workspaceError.value = message || "Unable to open folder dialog.";
  }
}

onMounted(() => {
  void loadWorkspace();
});

function normalizeDisplayPath(path: string) {
  if (path.startsWith("\\\\?\\")) {
    const withoutPrefix = path.slice(4);
    if (withoutPrefix.startsWith("UNC\\")) {
      return `\\\\${withoutPrefix.slice(4)}`;
    }
    return withoutPrefix;
  }
  return path;
}

function normalizeInputPath(value: string) {
  const trimmed = value.trim();
  const unquoted = stripWrappingQuotes(trimmed);
  return normalizeDisplayPath(unquoted);
}

function stripWrappingQuotes(value: string) {
  if (value.length < 2) return value;
  const first = value[0];
  const last = value[value.length - 1];
  if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
    return value.slice(1, -1);
  }
  return value;
}

function extractErrorMessage(error: unknown) {
  if (typeof error === "string") return error;
  if (error && typeof error === "object") {
    const record = error as Record<string, unknown>;
    if (typeof record.message === "string") return record.message;
    if (typeof record.error === "string") return record.error;
  }
  if (error instanceof Error) return error.message;
  return "";
}
</script>

<template>
  <div class="workspace-panel">
    <div class="panel-header">
      <div>
        <p class="eyebrow">Workspace</p>
        <h3>{{ workspaceLabel }}</h3>
      </div>
      <span class="badge">sandboxed</span>
    </div>

    <div class="workspace-select">
      <input
        v-model="workspacePathInput"
        type="text"
        placeholder="Workspace path"
        @keydown.enter.prevent="applyWorkspace"
      />
      <div class="workspace-actions">
        <button class="btn ghost" type="button" @click="applyWorkspace">Set</button>
        <button class="btn ghost" type="button" @click="browseWorkspace">Browse</button>
      </div>
    </div>
    <p v-if="workspaceError" class="error-text">{{ workspaceError }}</p>

    <div class="search-box">
      <input type="text" placeholder="Search (planned)" disabled />
      <button class="btn ghost" type="button" disabled>rg</button>
    </div>

    <div class="section">
      <div class="section-title">Tree</div>
      <p v-if="treeError" class="error-text">{{ treeError }}</p>
      <p v-else-if="!treeItems.length" class="empty-text">No files found.</p>
      <ul class="tree">
        <WorkspaceTreeNode v-for="item in treeItems" :key="item.path" :item="item" />
      </ul>
    </div>
  </div>
</template>

<style scoped>
.workspace-panel {
  display: flex;
  flex-direction: column;
  gap: 20px;
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 14px;
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
}

.badge {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.4);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
}

.search-box {
  display: flex;
  gap: 8px;
}

.search-box input {
  flex: 1;
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: var(--panel-glass);
  color: var(--text-primary);
}

.search-box input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.workspace-select {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.workspace-select input {
  flex: 1;
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: var(--panel-glass);
  color: var(--text-primary);
}

.workspace-actions {
  display: inline-flex;
  gap: 8px;
  flex: 0 0 auto;
}

.error-text {
  margin: 0;
  font-size: 0.7rem;
  color: var(--status-warning);
}

.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
}

.section-title {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: var(--text-secondary);
}

.tree {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 10px;
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding-right: 4px;
  font-size: 0.85rem;
  color: var(--text-soft);
}

.status {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid transparent;
}

.status[data-status="modified"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.4);
  background: rgba(var(--status-warning-rgb), 0.12);
}

.status[data-status="added"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.12);
}

.status[data-status="clean"] {
  color: var(--text-secondary);
  border-color: rgba(var(--text-secondary-rgb), 0.3);
  background: rgba(var(--text-secondary-rgb), 0.12);
}

.btn {
  border-radius: 10px;
  border: 1px solid var(--line);
  padding: 8px 10px;
  font-size: 0.7rem;
  background: var(--panel-glass);
  color: var(--text-secondary);
  cursor: pointer;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: var(--status-success);
}

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
</style>


