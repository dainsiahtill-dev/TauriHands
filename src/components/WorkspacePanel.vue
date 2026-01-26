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

type SearchMatch = {
  path: string;
  line: number;
  column: number;
  text: string;
};

type ToolResult = {
  ok: boolean;
  stdout_excerpt?: string | null;
  stderr_excerpt?: string | null;
  artifacts?: { matches?: SearchMatch[] } | null;
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
const searchQuery = ref("");
const searchResults = ref<SearchMatch[]>([]);
const searchError = ref("");
const searchStatus = ref<"idle" | "loading">("idle");
const isSearching = computed(() => searchStatus.value === "loading");

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

async function runSearch() {
  const pattern = searchQuery.value.trim();
  if (!pattern) {
    searchError.value = "Search query is required.";
    searchResults.value = [];
    return;
  }
  searchError.value = "";
  searchStatus.value = "loading";
  try {
    const result = (await invoke("fs_search", {
      request: {
        pattern,
        max_results: 200,
      },
    })) as ToolResult;
    if (!result.ok) {
      searchError.value = result.stderr_excerpt ?? "Search failed.";
      searchResults.value = [];
      return;
    }
    searchResults.value = result.artifacts?.matches ?? [];
  } catch (error) {
    const message = extractErrorMessage(error);
    searchError.value = message || "Unable to run search.";
    searchResults.value = [];
  } finally {
    searchStatus.value = "idle";
  }
}

function formatLocation(match: SearchMatch) {
  if (!match.line) return match.path;
  const column = match.column ? `:${match.column}` : "";
  return `${match.path}:${match.line}${column}`;
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
      <input
        v-model="searchQuery"
        type="text"
        placeholder="Search in workspace"
        @keydown.enter.prevent="runSearch"
      />
      <button class="btn ghost" type="button" @click="runSearch" :disabled="isSearching">
        rg
      </button>
    </div>
    <p v-if="searchError" class="error-text">{{ searchError }}</p>

    <div class="section">
      <div class="section-title">Results</div>
      <p v-if="isSearching" class="empty-text">Searching...</p>
      <p v-else-if="!searchResults.length" class="empty-text">No results.</p>
      <ul v-else class="search-results">
        <li v-for="(match, index) in searchResults" :key="`${match.path}-${match.line}-${index}`">
          <div class="search-location">{{ formatLocation(match) }}</div>
          <div class="search-text">{{ match.text }}</div>
        </li>
      </ul>
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
  font-family: var(--font-display);
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.badge {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 6px 10px;
  border-radius: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.45);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
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

.search-box {
  display: flex;
  gap: 8px;
}

.search-box input {
  flex: 1;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  padding: 10px 12px;
  background: rgba(3, 12, 24, 0.9);
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

.search-box input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.search-results {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
  max-height: 200px;
  overflow: auto;
}

.search-results li {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.35);
  padding: 8px 10px;
  background: rgba(7, 12, 20, 0.75);
  display: grid;
  gap: 4px;
  font-size: 0.75rem;
  color: var(--text-soft);
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

.search-location {
  color: var(--accent);
  font-size: 0.68rem;
  word-break: break-all;
}

.search-text {
  color: var(--text-secondary);
  word-break: break-word;
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
  font-family: var(--font-display);
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
  border-radius: 0;
  border: 1px solid transparent;
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
  font-family: var(--font-display);
}

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
</style>


