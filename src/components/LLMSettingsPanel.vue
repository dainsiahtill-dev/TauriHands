<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";

type ProviderId = "openai" | "anthropic" | "local" | "ollama" | "azure";

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
  { id: "ollama", label: "Ollama (LAN)" },
  { id: "azure", label: "Azure OpenAI" },
];

const modelsByProvider: Record<ProviderId, string[]> = {
  openai: ["gpt-5.1", "gpt-5.2", "gpt-5.2-codex"],
  anthropic: ["claude-3.5-sonnet", "claude-3.5-haiku", "claude-3-opus"],
  local: ["llama3.1:70b", "qwen2.5:32b", "mistral-large"],
  ollama: ["llama3.1:70b", "qwen2.5:32b", "mistral-large"],
  azure: ["gpt-4o", "gpt-4.1", "gpt-35-turbo"],
};

type ProviderConfig = {
  apiKey: string;
  baseUrl: string;
  model: string;
};

function buildProviderDefaults(): Record<ProviderId, ProviderConfig> {
  return {
    openai: { apiKey: "", baseUrl: "", model: modelsByProvider.openai[0] ?? "" },
    anthropic: { apiKey: "", baseUrl: "", model: modelsByProvider.anthropic[0] ?? "" },
    local: { apiKey: "", baseUrl: "", model: modelsByProvider.local[0] ?? "" },
    ollama: { apiKey: "", baseUrl: "", model: modelsByProvider.ollama[0] ?? "" },
    azure: { apiKey: "", baseUrl: "", model: modelsByProvider.azure[0] ?? "" },
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
const modelFetchStatus = ref<"idle" | "loading" | "ok" | "error">("idle");
const modelFetchMessage = ref("");
const modelFetchAt = ref<number | null>(null);

const modelFetchLabel = computed(() => {
  if (modelFetchStatus.value === "loading") return "Loading";
  if (modelFetchStatus.value === "ok") return "Models loaded";
  if (modelFetchStatus.value === "error") return "Load failed";
  return "Idle";
});

const modelTestStatus = ref<"idle" | "running" | "ok" | "error">("idle");
const modelTestMessage = ref("");
const modelTestDetail = ref("");

const modelTestLabel = computed(() => {
  if (modelTestStatus.value === "running") return "Testing";
  if (modelTestStatus.value === "ok") return "Model ready";
  if (modelTestStatus.value === "error") return "Model error";
  return "Idle";
});

const isLocalProvider = computed(() => provider.value === "local" || provider.value === "ollama");
const modelOptions = computed(() => {
  const baseOptions =
    isLocalProvider.value && ollamaModels.value.length
      ? ollamaModels.value
      : modelsByProvider[provider.value] ?? [];
  const current = model.value.trim();
  if (current && !baseOptions.includes(current)) {
    return [current, ...baseOptions];
  }
  return baseOptions;
});
const baseUrlPlaceholder = computed(() => {
  if (provider.value === "ollama") return "http://<LAN-IP>:11434";
  if (provider.value === "local") return "http://localhost:11434";
  if (provider.value === "azure") return "https://{resource}.openai.azure.com";
  return "https://api.openai.com/v1";
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
  const nextModels = modelOptions.value;
  if (!model.value) {
    model.value = nextModels?.[0] ?? "";
  }
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
});

watch(baseUrl, () => {
  if (!isLocalProvider.value) return;
  ollamaModels.value = [];
  modelFetchStatus.value = "idle";
  modelFetchMessage.value = "";
  modelFetchAt.value = null;
  modelTestStatus.value = "idle";
  modelTestMessage.value = "";
  modelTestDetail.value = "";
});

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
  const nextModels = modelOptions.value;
  if (!model.value) {
    model.value = nextModels?.[0] ?? "";
  }
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

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}

const modelFetchTime = computed(() =>
  modelFetchAt.value ? formatTime(modelFetchAt.value) : "",
);

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

function parseOllamaTags(payload: unknown) {
  if (!payload || typeof payload !== "object") return [];
  const record = payload as { models?: Array<{ name?: string }> };
  if (!Array.isArray(record.models)) return [];
  return record.models.map((item) => item.name).filter((name): name is string => Boolean(name));
}

function parseOpenAiModels(payload: unknown) {
  if (!payload || typeof payload !== "object") return [];
  const record = payload as { data?: Array<{ id?: string }> };
  if (!Array.isArray(record.data)) return [];
  return record.data.map((item) => item.id).filter((name): name is string => Boolean(name));
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

  const endpoints = [
    { url: `${base}/api/tags`, parser: parseOllamaTags },
    { url: `${base}/v1/models`, parser: parseOpenAiModels },
  ];

  let lastError = "";
  for (const endpoint of endpoints) {
    try {
      const response = await fetch(endpoint.url, { method: "GET" });
      if (!response.ok) {
        lastError = `HTTP ${response.status} ${response.statusText}`;
        continue;
      }
      const payload = (await response.json()) as unknown;
      const models = endpoint.parser(payload);
      const uniqueModels = Array.from(new Set(models)).sort();
      if (uniqueModels.length > 0) {
        ollamaModels.value = uniqueModels;
        modelFetchStatus.value = "ok";
        modelFetchMessage.value = `Loaded ${uniqueModels.length} models from ${endpoint.url}`;
        modelFetchAt.value = Date.now();
        if (isLocalProvider.value && !uniqueModels.includes(model.value)) {
          model.value = uniqueModels[0];
        }
        return;
      }
      lastError = "No models found.";
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
  }

  modelFetchStatus.value = "error";
  modelFetchMessage.value = lastError || "Unable to load models.";
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
    const showResponse = await fetch(`${base}/api/show`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: targetModel }),
    });
    if (showResponse.ok) {
      modelTestStatus.value = "ok";
      modelTestMessage.value = `Model "${targetModel}" is available.`;
      return;
    }
  } catch (error) {
    modelTestDetail.value = error instanceof Error ? error.message : String(error);
  }

  try {
    const response = await fetch(`${base}/v1/models`, { method: "GET" });
    if (response.ok) {
      const payload = (await response.json()) as unknown;
      const models = parseOpenAiModels(payload);
      if (models.includes(targetModel)) {
        modelTestStatus.value = "ok";
        modelTestMessage.value = `Model "${targetModel}" is available.`;
        return;
      }
    }
  } catch (error) {
    modelTestDetail.value = error instanceof Error ? error.message : String(error);
  }

  try {
    const response = await fetch(`${base}/api/tags`, { method: "GET" });
    if (response.ok) {
      const payload = (await response.json()) as unknown;
      const models = parseOllamaTags(payload);
      if (models.includes(targetModel)) {
        modelTestStatus.value = "ok";
        modelTestMessage.value = `Model "${targetModel}" is available.`;
        return;
      }
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
            <input v-model="baseUrl" type="text" :placeholder="baseUrlPlaceholder" />
          </label>
          <label class="full">
            <span>API Key</span>
            <input v-model="apiKey" type="password" placeholder="sk-..." />
          </label>
        </div>
        <div v-if="isLocalProvider" class="model-tools">
          <div class="model-tools__actions">
            <button
              class="btn ghost"
              type="button"
              :disabled="modelFetchStatus === 'loading'"
              @click="fetchOllamaModels"
            >
              {{ modelFetchStatus === "loading" ? "Loading..." : "Load models" }}
            </button>
            <button
              class="btn ghost"
              type="button"
              :disabled="modelTestStatus === 'running'"
              @click="testOllamaModel"
            >
              {{ modelTestStatus === "running" ? "Testing..." : "Test model" }}
            </button>
            <span class="pill" :data-status="modelFetchStatus">{{ modelFetchLabel }}</span>
            <span class="pill" :data-status="modelTestStatus">{{ modelTestLabel }}</span>
          </div>
          <div v-if="modelFetchMessage" class="hint">{{ modelFetchMessage }}</div>
          <div v-if="modelFetchTime" class="hint">Last sync: {{ modelFetchTime }}</div>
          <div v-if="modelTestMessage" class="hint">{{ modelTestMessage }}</div>
          <pre v-if="modelTestDetail" class="model-detail">{{ modelTestDetail }}</pre>
          <div v-if="ollamaModels.length" class="model-list">
            <button
              v-for="item in ollamaModels"
              :key="item"
              type="button"
              class="model-chip"
              :class="{ active: model === item }"
              @click="model = item"
            >
              {{ item }}
            </button>
          </div>
          <p v-else class="hint">No models loaded yet.</p>
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
  color: var(--text-secondary);
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
  color: var(--text-secondary);
}

.save-status[data-status="saved"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.12);
}

.save-status[data-status="error"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.4);
  background: rgba(var(--status-warning-rgb), 0.12);
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
  background: radial-gradient(circle, rgba(var(--accent-rgb), 0.12), transparent 70%);
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
  border: 1px solid rgba(var(--accent-rgb), 0.3);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
}

.pill[data-status="loading"],
.pill[data-status="running"] {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.5);
  background: rgba(var(--accent-rgb), 0.14);
}

.pill[data-status="ok"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.5);
  background: rgba(var(--status-success-rgb), 0.12);
}

.pill[data-status="error"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.5);
  background: rgba(var(--status-warning-rgb), 0.12);
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
  color: var(--text-secondary);
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
  color: var(--text-primary);
  font-family: inherit;
}

textarea {
  resize: vertical;
  min-height: 140px;
  position: relative;
  z-index: 1;
}

input[type="range"] {
  accent-color: var(--accent);
}

.range-meta {
  font-size: 0.75rem;
  color: var(--text-secondary);
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
  color: var(--text-soft);
}

.switch input {
  accent-color: var(--status-success);
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
  color: var(--text-soft);
}

.toggle-pill input {
  accent-color: var(--accent);
}

.hint {
  font-size: 0.78rem;
  color: var(--text-secondary);
  position: relative;
  z-index: 1;
}

.model-tools {
  display: grid;
  gap: 10px;
  position: relative;
  z-index: 1;
}

.model-tools__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}

.model-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.model-chip {
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: rgba(7, 12, 22, 0.75);
  color: var(--text-soft);
  padding: 6px 10px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  cursor: pointer;
  transition: all 0.2s ease;
}

.model-chip:hover {
  border-color: rgba(var(--accent-rgb), 0.5);
  color: var(--text-primary);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.18);
}

.model-chip.active {
  border-color: rgba(var(--accent-rgb), 0.6);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
  box-shadow: 0 0 14px rgba(var(--accent-rgb), 0.2);
}

.model-detail {
  margin: 0;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(5, 8, 14, 0.7);
  border: 1px solid var(--line);
  font-size: 0.7rem;
  color: var(--text-soft);
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  max-height: 160px;
  overflow: auto;
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
  color: var(--text-secondary);
}

.log-level {
  text-transform: uppercase;
  letter-spacing: 0.12em;
  font-size: 0.6rem;
  color: var(--status-success);
}

.log-message {
  color: var(--text-primary);
}

.log-detail {
  margin: 0;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(5, 8, 14, 0.7);
  border: 1px solid var(--line);
  font-size: 0.7rem;
  color: var(--text-soft);
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
  color: var(--status-success);
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


