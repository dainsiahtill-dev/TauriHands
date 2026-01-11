<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";

type ProviderId = "openai" | "anthropic" | "local" | "azure";

const provider = ref<ProviderId>("openai");
const profileName = ref("Default");
const apiKey = ref("");
const baseUrl = ref("");
const model = ref("gpt-4o");
const prompt = ref(
  "You are a precise coding agent. Use tools, summarize changes, and avoid unsafe commands.",
);
const contextPolicy = ref("adaptive");
const memoryMode = ref("session");
const enableCaching = ref(true);
const maxTerminalLines = ref(800);
const redactSecrets = ref(true);
const auditLogs = ref(true);

const DEFAULT_RUNTIME = {
  temperature: 0.2,
  topP: 0.9,
  maxTokens: 2048,
  contextWindow: 128000,
  streamResponses: true,
  toolCalling: true,
  safetyMode: true,
  retries: 2,
  concurrency: 2,
};

const providerOptions = [
  { id: "openai", label: "OpenAI" },
  { id: "anthropic", label: "Anthropic" },
  { id: "local", label: "Local (Ollama/LM Studio)" },
  { id: "azure", label: "Azure OpenAI" },
];

const modelsByProvider: Record<ProviderId, string[]> = {
  openai: ["gpt-5.1", "gpt-5.2", "gpt-5.2-codex"],
  anthropic: ["claude-3.5-sonnet", "claude-3.5-haiku", "claude-3-opus"],
  local: ["llama3.1:70b", "qwen2.5:32b", "mistral-large"],
  azure: ["gpt-4o", "gpt-4.1", "gpt-35-turbo"],
};

const modelOptions = computed(() => modelsByProvider[provider.value] ?? []);

watch(provider, (next) => {
  const nextModels = modelsByProvider[next];
  model.value = nextModels?.[0] ?? "";
});

const toolToggles = ref([
  { id: "terminal.exec_interactive", label: "terminal.exec_interactive", enabled: true },
  { id: "terminal.run_command", label: "terminal.run_command", enabled: true },
  { id: "fs.read_file", label: "fs.read_file", enabled: true },
  { id: "fs.write_file", label: "fs.write_file", enabled: false },
  { id: "fs.apply_patch", label: "fs.apply_patch", enabled: true },
  { id: "fs.search", label: "fs.search", enabled: true },
  { id: "git.status", label: "git.status", enabled: true },
  { id: "git.diff", label: "git.diff", enabled: true },
  { id: "tests.run", label: "tests.run", enabled: false },
]);

type TestLog = {
  id: string;
  level: "info" | "success" | "warn" | "error";
  message: string;
  timestamp: number;
  detail?: string;
};

const testLogs = ref<TestLog[]>([]);
const testStatus = ref<"idle" | "running" | "ok" | "error">("idle");

const testStatusLabel = computed(() => {
  switch (testStatus.value) {
    case "running":
      return "Testing";
    case "ok":
      return "Connected";
    case "error":
      return "Error";
    default:
      return "Idle";
  }
});

const isTesting = computed(() => testStatus.value === "running");

type LLMProfile = {
  profileName: string;
  provider: ProviderId;
  apiKey: string;
  baseUrl: string;
  model: string;
  temperature: number;
  topP: number;
  maxTokens: number;
  contextWindow: number;
  streamResponses: boolean;
  toolCalling: boolean;
  safetyMode: boolean;
  retries: number;
  concurrency: number;
  prompt: string;
  contextPolicy: string;
  memoryMode: string;
  enableCaching: boolean;
  maxTerminalLines: number;
  redactSecrets: boolean;
  auditLogs: boolean;
  toolToggles: Array<{ id: string; enabled: boolean }>;
};

const saveStatus = ref<"idle" | "saving" | "saved" | "error">("idle");
const saveMessage = ref("");
const saveStatusLabel = computed(() => {
  switch (saveStatus.value) {
    case "saving":
      return "Saving";
    case "saved":
      return "Saved";
    case "error":
      return "Save failed";
    default:
      return "Not saved";
  }
});

function resetDefaults() {
  provider.value = "openai";
  profileName.value = "Default";
  apiKey.value = "";
  baseUrl.value = "";
  model.value = modelsByProvider.openai[0];
  prompt.value =
    "You are a precise coding agent. Use tools, summarize changes, and avoid unsafe commands.";
  contextPolicy.value = "adaptive";
  memoryMode.value = "session";
  enableCaching.value = true;
  maxTerminalLines.value = 800;
  redactSecrets.value = true;
  auditLogs.value = true;
  toolToggles.value = toolToggles.value.map((tool) => ({
    ...tool,
    enabled: ["fs.write_file", "tests.run"].includes(tool.id) ? false : true,
  }));
}

function buildProfile(): LLMProfile {
  return {
    profileName: profileName.value.trim() || "Default",
    provider: provider.value,
    apiKey: apiKey.value,
    baseUrl: baseUrl.value,
    model: model.value,
    temperature: DEFAULT_RUNTIME.temperature,
    topP: DEFAULT_RUNTIME.topP,
    maxTokens: DEFAULT_RUNTIME.maxTokens,
    contextWindow: DEFAULT_RUNTIME.contextWindow,
    streamResponses: DEFAULT_RUNTIME.streamResponses,
    toolCalling: DEFAULT_RUNTIME.toolCalling,
    safetyMode: DEFAULT_RUNTIME.safetyMode,
    retries: DEFAULT_RUNTIME.retries,
    concurrency: DEFAULT_RUNTIME.concurrency,
    prompt: prompt.value,
    contextPolicy: contextPolicy.value,
    memoryMode: memoryMode.value,
    enableCaching: enableCaching.value,
    maxTerminalLines: maxTerminalLines.value,
    redactSecrets: redactSecrets.value,
    auditLogs: auditLogs.value,
    toolToggles: toolToggles.value.map((tool) => ({ id: tool.id, enabled: tool.enabled })),
  };
}

function applyProfile(profile: LLMProfile) {
  profileName.value = profile.profileName;
  provider.value = profile.provider;
  apiKey.value = profile.apiKey;
  baseUrl.value = profile.baseUrl;
  const models = modelsByProvider[profile.provider] ?? [];
  model.value = models.includes(profile.model) ? profile.model : models[0] ?? "";
  prompt.value = profile.prompt;
  contextPolicy.value = profile.contextPolicy;
  memoryMode.value = profile.memoryMode;
  enableCaching.value = profile.enableCaching;
  maxTerminalLines.value = profile.maxTerminalLines;
  redactSecrets.value = profile.redactSecrets;
  auditLogs.value = profile.auditLogs;
  const togglesById = new Map(profile.toolToggles.map((tool) => [tool.id, tool.enabled]));
  toolToggles.value = toolToggles.value.map((tool) => ({
    ...tool,
    enabled: togglesById.get(tool.id) ?? tool.enabled,
  }));
}

async function loadProfile() {
  try {
    const profile = (await invoke("llm_get_profile")) as LLMProfile | null;
    if (profile) {
      applyProfile(profile);
      saveStatus.value = "saved";
      saveMessage.value = `Loaded ${profile.profileName}`;
    } else {
      saveStatus.value = "idle";
      saveMessage.value = "No profile stored yet.";
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    saveStatus.value = "error";
    saveMessage.value = message || "Unable to load profile.";
  }
}

async function saveProfile() {
  saveStatus.value = "saving";
  saveMessage.value = "";
  try {
    const profile = buildProfile();
    const saved = (await invoke("llm_save_profile", { profile })) as LLMProfile;
    applyProfile(saved);
    saveStatus.value = "saved";
    saveMessage.value = `Saved ${profile.profileName}`;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    saveStatus.value = "error";
    saveMessage.value = message || "Unable to save profile.";
  }
}

function makeLogId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
}

function pushTestLog(level: TestLog["level"], message: string, detail?: string) {
  testLogs.value.unshift({
    id: makeLogId("test"),
    level,
    message,
    timestamp: Date.now(),
    detail,
  });
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}

function normalizeBaseUrl(value: string) {
  return value.trim().replace(/\/+$/, "");
}

function withV1(base: string) {
  return base.endsWith("/v1") ? base : `${base}/v1`;
}

function resolveBaseUrl() {
  if (baseUrl.value.trim()) return normalizeBaseUrl(baseUrl.value);
  if (provider.value === "openai") return "https://api.openai.com/v1";
  if (provider.value === "anthropic") return "https://api.anthropic.com/v1";
  if (provider.value === "local") return "http://localhost:11434";
  return "";
}

async function testConnection() {
  testLogs.value = [];
  testStatus.value = "running";

  const providerValue = provider.value;
  pushTestLog("info", `Testing ${providerValue} connection`);

  const base = resolveBaseUrl();
  if (!base) {
    pushTestLog("error", "Base URL is required.");
    testStatus.value = "error";
    return;
  }

  if (providerValue !== "local" && !apiKey.value.trim()) {
    pushTestLog("error", "API key is required.");
    testStatus.value = "error";
    return;
  }

  try {
    if (providerValue === "local") {
      const candidates = [`${base}/api/tags`, `${base}/v1/models`];
      for (const url of candidates) {
        pushTestLog("info", `Requesting ${url}`);
        const response = await fetch(url, { method: "GET" });
        const body = await response.text();
        pushTestLog(
          response.ok ? "success" : "warn",
          `HTTP ${response.status} ${response.statusText}`,
          body.slice(0, 1200),
        );
        if (response.ok) {
          testStatus.value = "ok";
          return;
        }
      }
      testStatus.value = "error";
      return;
    }

    const url =
      providerValue === "azure" ? base : `${withV1(base)}/models`;
    pushTestLog("info", `Requesting ${url}`);

    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (providerValue === "anthropic") {
      headers["x-api-key"] = apiKey.value.trim();
      headers["anthropic-version"] = "2023-06-01";
    } else if (providerValue === "azure") {
      headers["api-key"] = apiKey.value.trim();
    } else {
      headers["Authorization"] = `Bearer ${apiKey.value.trim()}`;
    }

    const response = await fetch(url, {
      method: "GET",
      headers,
    });
    const body = await response.text();
    pushTestLog(
      response.ok ? "success" : "error",
      `HTTP ${response.status} ${response.statusText}`,
      body.slice(0, 1200),
    );
    testStatus.value = response.ok ? "ok" : "error";
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    pushTestLog("error", "Request failed", message);
    testStatus.value = "error";
  }
}

onMounted(() => {
  void loadProfile();
});
</script>

<template>
  <div class="llm-settings">
    <header class="settings-header">
      <div>
        <p class="eyebrow">LLM Settings</p>
        <h2>Model routing, safety, and tool governance</h2>
        <p class="subtitle">
          Configure providers, tokens, and tool access for deterministic agent execution.
        </p>
      </div>
      <div class="header-actions">
        <button class="btn ghost" type="button" :disabled="isTesting" @click="testConnection">
          {{ isTesting ? "Testing..." : "Test connection" }}
        </button>
        <button class="btn primary" type="button" @click="saveProfile">Save profile</button>
        <span class="save-status" :data-status="saveStatus" :title="saveMessage">
          {{ saveStatusLabel }}
        </span>
      </div>
    </header>

    <div class="settings-grid">
      <section class="card">
        <div class="card-head">
          <h3>Provider & Model</h3>
          <span class="pill">Profile: {{ profileName }}</span>
        </div>
        <div class="field-grid">
          <label>
            <span>Provider</span>
            <select v-model="provider">
              <option v-for="item in providerOptions" :key="item.id" :value="item.id">
                {{ item.label }}
              </option>
            </select>
          </label>
          <label>
            <span>Model</span>
            <select v-model="model">
              <option v-for="item in modelOptions" :key="item" :value="item">
                {{ item }}
              </option>
            </select>
          </label>
          <label class="full">
            <span>Base URL</span>
            <input v-model="baseUrl" type="text" placeholder="https://api.openai.com/v1" />
          </label>
          <label class="full">
            <span>API Key</span>
            <input v-model="apiKey" type="password" placeholder="sk-..." />
          </label>
        </div>
      </section>

      <section class="card wide">
        <div class="card-head">
          <h3>Connection test</h3>
          <span class="pill" :data-status="testStatus">{{ testStatusLabel }}</span>
        </div>
        <div v-if="testLogs.length" class="test-logs">
          <div
            v-for="log in testLogs"
            :key="log.id"
            class="test-log"
            :data-level="log.level"
          >
            <div class="test-log-main">
              <span class="log-time">{{ formatTime(log.timestamp) }}</span>
              <span class="log-level">{{ log.level }}</span>
              <span class="log-message">{{ log.message }}</span>
            </div>
            <pre v-if="log.detail" class="log-detail">{{ log.detail }}</pre>
          </div>
        </div>
        <p v-else class="hint">No test output yet.</p>
      </section>

      <section class="card wide">
        <div class="card-head">
          <h3>System Prompt</h3>
          <span class="pill">Tokens tracked</span>
        </div>
        <textarea v-model="prompt" rows="6"></textarea>
        <div class="hint">
          Keep the system prompt short and precise. Avoid leaking secrets or environment details.
        </div>
      </section>

      <section class="card">
        <div class="card-head">
          <h3>Context Strategy</h3>
          <span class="pill">Policy {{ contextPolicy }}</span>
        </div>
        <div class="field-grid">
          <label>
            <span>Context policy</span>
            <select v-model="contextPolicy">
              <option value="adaptive">Adaptive</option>
              <option value="terminal-first">Terminal first</option>
              <option value="code-first">Code first</option>
              <option value="summary-first">Summary first</option>
            </select>
          </label>
          <label>
            <span>Memory</span>
            <select v-model="memoryMode">
              <option value="session">Session</option>
              <option value="workspace">Workspace</option>
              <option value="off">Off</option>
            </select>
          </label>
          <label>
            <span>Terminal cap (lines)</span>
            <input v-model.number="maxTerminalLines" type="number" min="200" step="100" />
          </label>
          <label class="switch">
            <input v-model="enableCaching" type="checkbox" />
            <span>Enable response caching</span>
          </label>
        </div>
      </section>

      <section class="card">
        <div class="card-head">
          <h3>Tool Allowlist</h3>
          <span class="pill">Least privilege</span>
        </div>
        <div class="tool-grid">
          <label v-for="tool in toolToggles" :key="tool.id" class="toggle-pill">
            <input v-model="tool.enabled" type="checkbox" />
            <span>{{ tool.label }}</span>
          </label>
        </div>
      </section>

      <section class="card">
        <div class="card-head">
          <h3>Audit & Redaction</h3>
          <span class="pill">Compliance</span>
        </div>
        <div class="toggle-row">
          <label class="switch">
            <input v-model="auditLogs" type="checkbox" />
            <span>Append-only audit log</span>
          </label>
          <label class="switch">
            <input v-model="redactSecrets" type="checkbox" />
            <span>Redact secrets in logs</span>
          </label>
        </div>
        <div class="hint">
          Logs are stored locally and can be exported for review. Redaction masks API keys.
        </div>
      </section>
    </div>

    <footer class="settings-footer">
      <button class="btn ghost" type="button" @click="resetDefaults">Reset defaults</button>
      <div class="footer-right">
        <span class="hint">Changes are stored locally until you export or sync.</span>
        <button class="btn primary" type="button" @click="saveProfile">Apply settings</button>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.llm-settings {
  display: flex;
  flex-direction: column;
  gap: 24px;
  height: 100%;
  overflow: auto;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
}

.settings-header h2 {
  margin: 6px 0 8px;
  font-size: 1.6rem;
}

.subtitle {
  margin: 0;
  color: #8aa0b7;
  max-width: 520px;
}

.header-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.save-status {
  align-self: center;
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid var(--line);
  color: #9bb0c6;
}

.save-status[data-status="saved"] {
  color: #b6ff4b;
  border-color: rgba(182, 255, 75, 0.4);
  background: rgba(182, 255, 75, 0.12);
}

.save-status[data-status="error"] {
  color: #ffb84d;
  border-color: rgba(255, 184, 77, 0.4);
  background: rgba(255, 184, 77, 0.12);
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.card {
  padding: 18px;
  border-radius: 18px;
  background: var(--panel);
  border: 1px solid var(--line);
  display: grid;
  gap: 14px;
  position: relative;
  overflow: hidden;
}

.card::after {
  content: "";
  position: absolute;
  inset: -50% -50% auto auto;
  width: 240px;
  height: 240px;
  background: radial-gradient(circle, rgba(45, 246, 255, 0.12), transparent 70%);
  opacity: 0.6;
  pointer-events: none;
}

.card.wide {
  grid-column: span 2;
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  position: relative;
  z-index: 1;
}

.card-head h3 {
  margin: 0;
  font-size: 1.1rem;
}

.pill {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(45, 246, 255, 0.3);
  color: #2df6ff;
  background: rgba(45, 246, 255, 0.12);
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  position: relative;
  z-index: 1;
}

.field-grid label {
  display: grid;
  gap: 6px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: #9bb0c6;
}

.field-grid label.full {
  grid-column: span 2;
}

select,
input,
textarea {
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: var(--panel-glass);
  color: #e6f3ff;
  font-family: inherit;
}

textarea {
  resize: vertical;
  min-height: 140px;
  position: relative;
  z-index: 1;
}

input[type="range"] {
  accent-color: #2df6ff;
}

.range-meta {
  font-size: 0.75rem;
  color: #9bb0c6;
}

.toggle-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 16px;
  position: relative;
  z-index: 1;
}

.switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 0.8rem;
  color: #c7d7ec;
}

.switch input {
  accent-color: #b6ff4b;
}

.tool-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  position: relative;
  z-index: 1;
}

.toggle-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: var(--panel-glass);
  font-size: 0.78rem;
  color: #c7d7ec;
}

.toggle-pill input {
  accent-color: #2df6ff;
}

.hint {
  font-size: 0.78rem;
  color: #8aa0b7;
  position: relative;
  z-index: 1;
}

.test-logs {
  display: grid;
  gap: 10px;
  position: relative;
  z-index: 1;
}

.test-log {
  display: grid;
  gap: 6px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: var(--panel-glass);
}

.test-log-main {
  display: grid;
  grid-template-columns: auto auto 1fr;
  gap: 8px;
  align-items: center;
  font-size: 0.75rem;
}

.log-time {
  color: #9bb0c6;
}

.log-level {
  text-transform: uppercase;
  letter-spacing: 0.12em;
  font-size: 0.6rem;
  color: #b6ff4b;
}

.log-message {
  color: #e6f3ff;
}

.log-detail {
  margin: 0;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(5, 8, 14, 0.7);
  border: 1px solid var(--line);
  font-size: 0.7rem;
  color: #c7d7ec;
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  max-height: 220px;
  overflow: auto;
}

.settings-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding-top: 6px;
}

.footer-right {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: #b6ff4b;
}

@media (max-width: 1100px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }

  .card.wide {
    grid-column: auto;
  }

  .field-grid {
    grid-template-columns: 1fr;
  }

  .field-grid label.full {
    grid-column: auto;
  }

  .tool-grid {
    grid-template-columns: 1fr;
  }

  .settings-header {
    flex-direction: column;
  }

  .settings-footer {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
