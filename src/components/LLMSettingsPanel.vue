<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ProviderConfig from "./llm/ProviderConfig.vue";
import ConnectionTest from "./llm/ConnectionTest.vue";
import SystemPrompt from "./llm/SystemPrompt.vue";
import ContextStrategy from "./llm/ContextStrategy.vue";
import ToolAllowlist from "./llm/ToolAllowlist.vue";
import AuditSettings from "./llm/AuditSettings.vue";

type ProviderId = "openai" | "anthropic" | "local" | "ollama" | "azure";

const provider = ref<ProviderId>("openai");
const profileName = ref("Default");
const apiKey = ref("");
const baseUrl = ref("");
const DEFAULT_OPENAI_MODEL = "gpt-4o";
const model = ref(DEFAULT_OPENAI_MODEL);
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

type ProviderConfig = {
  apiKey: string;
  baseUrl: string;
  model: string;
};

function buildProviderDefaults(): Record<ProviderId, ProviderConfig> {
  return {
    openai: { apiKey: "", baseUrl: "", model: DEFAULT_OPENAI_MODEL },
    anthropic: { apiKey: "", baseUrl: "", model: "claude-3.5-sonnet" },
    local: { apiKey: "", baseUrl: "", model: "llama3.1:70b" },
    ollama: { apiKey: "", baseUrl: "", model: "llama3.1:70b" },
    azure: { apiKey: "", baseUrl: "", model: "gpt-4o" },
  };
}

const providerDefaults = buildProviderDefaults();
const providerConfigs = ref<Record<ProviderId, ProviderConfig>>({
  ...providerDefaults,
});

function snapshotProviderConfig(): ProviderConfig {
  return {
    apiKey: apiKey.value,
    baseUrl: baseUrl.value,
    model: model.value,
  };
}

function resolveProviderConfig(id: ProviderId): ProviderConfig {
  const stored = providerConfigs.value[id];
  return {
    ...providerDefaults[id],
    ...(stored ?? {}),
  };
}

function applyProviderConfig(id: ProviderId) {
  const config = resolveProviderConfig(id);
  apiKey.value = config.apiKey;
  baseUrl.value = config.baseUrl;
  model.value = config.model;
}

const ollamaModels = ref<string[]>([]);
const openAiModels = ref<string[]>([]);
const openAiFetchStatus = ref<"idle" | "loading" | "ok" | "error">("idle");
const openAiFetchMessage = ref("");
const openAiFetchAt = ref<number | null>(null);

const modelFetchStatus = ref<"idle" | "loading" | "ok" | "error">("idle");
const modelFetchMessage = ref("");
const modelFetchAt = ref<number | null>(null);

const modelTestStatus = ref<"idle" | "running" | "ok" | "error">("idle");
const modelTestMessage = ref("");
const modelTestDetail = ref("");

type TestLog = {
  id: string;
  level: "info" | "success" | "warn" | "error";
  message: string;
  timestamp: number;
  detail?: string;
};

type LlmModelFetchResponse = {
  models: string[];
  sourceUrl: string;
};

const testLogs = ref<TestLog[]>([]);
const testStatus = ref<"idle" | "running" | "ok" | "error">("idle");

const isTesting = computed(() => testStatus.value === "running");

const toolToggles = ref([
  { id: "terminal.exec_interactive", label: "terminal.exec_interactive", enabled: true },
  { id: "terminal.run_command", label: "terminal.run_command", enabled: true },
  { id: "fs.read_file", label: "fs.read_file", enabled: true },
  { id: "fs.write_file", label: "fs.write_file", enabled: true },
  { id: "fs.apply_patch", label: "fs.apply_patch", enabled: true },
  { id: "fs.search", label: "fs.search", enabled: true },
  { id: "git.status", label: "git.status", enabled: true },
  { id: "git.diff", label: "git.diff", enabled: true },
  { id: "tests.run", label: "tests.run", enabled: false },
]);

type LLMProfile = {
  profileName: string;
  provider: ProviderId;
  apiKey: string;
  baseUrl: string;
  model: string;
  providerConfigs?: Partial<Record<ProviderId, ProviderConfig>>;
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

let isHydrating = false;

watch(provider, (next, prev) => {
  if (isHydrating) return;
  if (prev) {
    providerConfigs.value = {
      ...providerConfigs.value,
      [prev]: snapshotProviderConfig(),
    };
  }
  applyProviderConfig(next);
  if (next !== "local" && next !== "ollama") {
    modelFetchStatus.value = "idle";
    modelFetchMessage.value = "";
    modelFetchAt.value = null;
    modelTestStatus.value = "idle";
    modelTestMessage.value = "";
    modelTestDetail.value = "";
    ollamaModels.value = [];
  } else if (prev && prev !== next) {
    ollamaModels.value = [];
    modelFetchStatus.value = "idle";
    modelFetchMessage.value = "";
    modelFetchAt.value = null;
    modelTestStatus.value = "idle";
    modelTestMessage.value = "";
    modelTestDetail.value = "";
  }
  if (prev && prev !== next) {
    resetOpenAiModels();
  }
});

watch(baseUrl, () => {
  if (provider.value === "local" || provider.value === "ollama") {
    ollamaModels.value = [];
    modelFetchStatus.value = "idle";
    modelFetchMessage.value = "";
    modelFetchAt.value = null;
    modelTestStatus.value = "idle";
    modelTestMessage.value = "";
    modelTestDetail.value = "";
  }
  if (provider.value === "openai") {
    resetOpenAiModels();
  }
});

watch(apiKey, () => {
  if (provider.value === "openai") {
    resetOpenAiModels();
  }
});

function resetDefaults() {
  isHydrating = true;
  providerConfigs.value = {
    ...providerDefaults,
  };
  provider.value = "openai";
  applyProviderConfig("openai");
  profileName.value = "Default";
  prompt.value =
    "You are a precise coding agent. Use tools, summarize changes, and avoid unsafe commands.";
  contextPolicy.value = "adaptive";
  memoryMode.value = "session";
  enableCaching.value = true;
  maxTerminalLines.value = 800;
  redactSecrets.value = true;
  auditLogs.value = true;
  ollamaModels.value = [];
  modelFetchStatus.value = "idle";
  modelFetchMessage.value = "";
  modelFetchAt.value = null;
  modelTestStatus.value = "idle";
  modelTestMessage.value = "";
  modelTestDetail.value = "";
  resetOpenAiModels();
  toolToggles.value = toolToggles.value.map((tool) => ({
    ...tool,
    enabled: ["fs.write_file", "tests.run"].includes(tool.id) ? false : true,
  }));
  isHydrating = false;
}

function buildProfile(): LLMProfile {
  const configs: Partial<Record<ProviderId, ProviderConfig>> = {
    ...providerConfigs.value,
    [provider.value]: snapshotProviderConfig(),
  };
  return {
    profileName: profileName.value.trim() || "Default",
    provider: provider.value,
    apiKey: apiKey.value,
    baseUrl: baseUrl.value,
    model: model.value,
    providerConfigs: configs,
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
  isHydrating = true;
  profileName.value = profile.profileName;
  const defaults = buildProviderDefaults();
  const storedConfigs = profile.providerConfigs ?? {};
  const mergedConfigs: Record<ProviderId, ProviderConfig> = {
    ...defaults,
  };
  (Object.keys(defaults) as ProviderId[]).forEach((id) => {
    const stored = storedConfigs[id];
    if (stored) {
      mergedConfigs[id] = { ...defaults[id], ...stored };
    }
  });
  if (profile.provider in mergedConfigs) {
    mergedConfigs[profile.provider] = {
      ...mergedConfigs[profile.provider],
      apiKey: profile.apiKey,
      baseUrl: profile.baseUrl,
      model: profile.model,
    };
  }
  providerConfigs.value = mergedConfigs;
  provider.value = profile.provider;
  applyProviderConfig(profile.provider);
  resetOpenAiModels();
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
  isHydrating = false;
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
  if (provider.value === "ollama") return "";
  return "";
}

function resolveOllamaBaseUrl() {
  if (baseUrl.value.trim()) return normalizeBaseUrl(baseUrl.value);
  if (provider.value === "local") return "http://localhost:11434";
  return "";
}

function resetOpenAiModels() {
  openAiModels.value = [];
  openAiFetchStatus.value = "idle";
  openAiFetchMessage.value = "";
  openAiFetchAt.value = null;
}

async function fetchOpenAiModels() {
  openAiFetchStatus.value = "loading";
  openAiFetchMessage.value = "";
  openAiFetchAt.value = null;

  const base = resolveBaseUrl();
  if (!base) {
    openAiFetchStatus.value = "error";
    openAiFetchMessage.value = "Base URL is required.";
    return;
  }

  if (!apiKey.value.trim()) {
    openAiFetchStatus.value = "error";
    openAiFetchMessage.value = "API key is required.";
    return;
  }

  try {
    const response = (await invoke("llm_fetch_models", {
      request: {
        provider: provider.value,
        baseUrl: base,
        apiKey: apiKey.value,
      },
    })) as LlmModelFetchResponse;
    const uniqueModels = response.models ?? [];
    if (!uniqueModels.length) {
      openAiFetchStatus.value = "error";
      openAiFetchMessage.value = "No models found.";
      return;
    }
    openAiModels.value = uniqueModels;
    openAiFetchStatus.value = "ok";
    openAiFetchMessage.value = `Loaded ${uniqueModels.length} models from ${response.sourceUrl}`;
    openAiFetchAt.value = Date.now();
    if (!uniqueModels.includes(model.value)) {
      model.value = uniqueModels[0];
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    openAiFetchStatus.value = "error";
    openAiFetchMessage.value = message || "Unable to load models.";
  }
}

async function fetchOllamaModels() {
  modelFetchStatus.value = "loading";
  modelFetchMessage.value = "";
  modelFetchAt.value = null;

  const base = resolveOllamaBaseUrl();
  if (!base) {
    modelFetchStatus.value = "error";
    modelFetchMessage.value = "Base URL is required for Ollama.";
    return;
  }

  try {
    const response = (await invoke("llm_fetch_models", {
      request: {
        provider: provider.value,
        baseUrl: base,
        apiKey: apiKey.value,
      },
    })) as LlmModelFetchResponse;
    const uniqueModels = response.models ?? [];
    if (!uniqueModels.length) {
      modelFetchStatus.value = "error";
      modelFetchMessage.value = "No models found.";
      return;
    }
    ollamaModels.value = uniqueModels;
    modelFetchStatus.value = "ok";
    modelFetchMessage.value = `Loaded ${uniqueModels.length} models from ${response.sourceUrl}`;
    modelFetchAt.value = Date.now();
    if (!uniqueModels.includes(model.value)) {
      model.value = uniqueModels[0];
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    modelFetchStatus.value = "error";
    modelFetchMessage.value = message || "Unable to load models.";
  }
}

async function testOllamaModel() {
  modelTestStatus.value = "running";
  modelTestMessage.value = "";
  modelTestDetail.value = "";

  const base = resolveOllamaBaseUrl();
  if (!base) {
    modelTestStatus.value = "error";
    modelTestMessage.value = "Base URL is required for Ollama.";
    return;
  }

  const targetModel = model.value.trim();
  if (!targetModel) {
    modelTestStatus.value = "error";
    modelTestMessage.value = "Select a model to test.";
    return;
  }

  try {
    const response = (await invoke("llm_fetch_models", {
      request: {
        provider: provider.value,
        baseUrl: base,
        apiKey: apiKey.value,
      },
    })) as LlmModelFetchResponse;
    if (response.models.includes(targetModel)) {
      modelTestStatus.value = "ok";
      modelTestMessage.value = `Model "${targetModel}" is available.`;
      return;
    }
  } catch (error) {
    modelTestDetail.value = error instanceof Error ? error.message : String(error);
  }

  modelTestStatus.value = "error";
  modelTestMessage.value = `Model "${targetModel}" not found.`;
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

  if (providerValue !== "local" && providerValue !== "ollama" && !apiKey.value.trim()) {
    pushTestLog("error", "API key is required.");
    testStatus.value = "error";
    return;
  }

  try {
    if (providerValue === "local" || providerValue === "ollama") {
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
        <span class="save-status" :data-status="saveStatus" :title="saveMessage">
          {{ saveStatusLabel }}
        </span>
        <button class="btn primary" type="button" @click="saveProfile">Save profile</button>
      </div>
    </header>

    <div class="settings-grid">
      <section class="card">
        <div class="card-head">
          <h3>Provider & Model</h3>
          <span class="pill">Profile: {{ profileName }}</span>
        </div>
        <ProviderConfig
          :provider="provider"
          :model="model"
          :api-key="apiKey"
          :base-url="baseUrl"
          :ollama-models="ollamaModels"
          :open-ai-models="openAiModels"
          :model-fetch-status="modelFetchStatus"
          :model-fetch-message="modelFetchMessage"
          :model-fetch-at="modelFetchAt"
          :model-test-status="modelTestStatus"
          :model-test-message="modelTestMessage"
          :model-test-detail="modelTestDetail"
          :open-ai-fetch-status="openAiFetchStatus"
          :open-ai-fetch-message="openAiFetchMessage"
          :open-ai-fetch-at="openAiFetchAt"
          @update:provider="provider = $event"
          @update:model="model = $event"
          @update:apiKey="apiKey = $event"
          @update:baseUrl="baseUrl = $event"
          @fetchOllamaModels="fetchOllamaModels"
          @testOllamaModel="testOllamaModel"
          @fetchOpenAiModels="fetchOpenAiModels"
        />
      </section>

      <section class="card wide">
        <ConnectionTest
          :test-logs="testLogs"
          :test-status="testStatus"
          :is-testing="isTesting"
          @testConnection="testConnection"
        />
      </section>

      <section class="card wide">
        <SystemPrompt
          :prompt="prompt"
          @update:prompt="prompt = $event"
        />
      </section>

      <section class="card">
        <ContextStrategy
          :context-policy="contextPolicy"
          :memory-mode="memoryMode"
          :max-terminal-lines="maxTerminalLines"
          :enable-caching="enableCaching"
          @update:contextPolicy="contextPolicy = $event"
          @update:memoryMode="memoryMode = $event"
          @update:maxTerminalLines="maxTerminalLines = $event"
          @update:enableCaching="enableCaching = $event"
        />
      </section>

      <section class="card">
        <ToolAllowlist
          :tool-toggles="toolToggles"
          @update:toolToggles="toolToggles = $event"
        />
      </section>

      <section class="card">
        <AuditSettings
          :audit-logs="auditLogs"
          :redact-secrets="redactSecrets"
          @update:auditLogs="auditLogs = $event"
          @update:redactSecrets="redactSecrets = $event"
        />
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
  padding: 12px 16px;
  border-radius: 16px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
}

.settings-header h2 {
  margin: 6px 0 8px;
  font-size: 1.4rem;
  letter-spacing: 0.02em;
}

.subtitle {
  margin: 0;
  color: var(--text-secondary);
  max-width: 520px;
  line-height: 1.5;
}

.header-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.save-status {
  align-self: center;
  font-size: 0.7rem;
  letter-spacing: 0.03em;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  color: var(--text-secondary);
  background: rgba(var(--line-rgb), 0.06);
}

.save-status[data-status="saved"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.3);
  background: rgba(var(--status-success-rgb), 0.12);
}

.save-status[data-status="error"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.3);
  background: rgba(var(--status-warning-rgb), 0.12);
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.card {
  padding: 18px;
  border-radius: 16px;
  background: rgba(var(--line-rgb), 0.06);
  border: 1px solid rgba(var(--line-rgb), 0.18);
  display: grid;
  gap: 14px;
  position: relative;
  overflow: hidden;
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
  font-size: 0.95rem;
  font-family: var(--font-display);
  letter-spacing: 0.02em;
}

.pill {
  font-size: 0.65rem;
  letter-spacing: 0.03em;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.35);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
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

.hint {
  font-size: 0.78rem;
  color: var(--text-secondary);
  position: relative;
  z-index: 1;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  letter-spacing: 0.03em;
  color: var(--status-success);
}

@media (max-width: 1100px) {
  .settings-grid {
    grid-template-columns: 1fr;
  }

  .card.wide {
    grid-column: auto;
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
