<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ToolAllowlist from "../components/llm/ToolAllowlist.vue";

type ProviderConfig = {
  apiKey: string;
  baseUrl: string;
  model: string;
};

type LLMProfile = {
  profileName: string;
  provider: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  providerConfigs?: Record<string, ProviderConfig>;
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

type ToolToggle = {
  id: string;
  label: string;
  enabled: boolean;
};

const DEFAULT_OPENAI_MODEL = "gpt-4o";

const defaultToolToggles: ToolToggle[] = [
  { id: "terminal.exec_interactive", label: "terminal.exec_interactive", enabled: true },
  { id: "terminal.run_command", label: "terminal.run_command", enabled: true },
  { id: "fs.read_file", label: "fs.read_file", enabled: true },
  { id: "fs.write_file", label: "fs.write_file", enabled: false },
  { id: "fs.apply_patch", label: "fs.apply_patch", enabled: true },
  { id: "fs.search", label: "fs.search", enabled: true },
  { id: "git.status", label: "git.status", enabled: true },
  { id: "git.diff", label: "git.diff", enabled: true },
  { id: "tests.run", label: "tests.run", enabled: false },
];

function buildProviderDefaults(): Record<string, ProviderConfig> {
  return {
    openai: { apiKey: "", baseUrl: "", model: DEFAULT_OPENAI_MODEL },
    anthropic: { apiKey: "", baseUrl: "", model: "claude-3.5-sonnet" },
    local: { apiKey: "", baseUrl: "", model: "llama3.1:70b" },
    ollama: { apiKey: "", baseUrl: "", model: "llama3.1:70b" },
    azure: { apiKey: "", baseUrl: "", model: "gpt-4o" },
  };
}

function defaultProfile(): LLMProfile {
  return {
    profileName: "Default",
    provider: "openai",
    apiKey: "",
    baseUrl: "",
    model: DEFAULT_OPENAI_MODEL,
    providerConfigs: buildProviderDefaults(),
    temperature: 0.2,
    topP: 0.9,
    maxTokens: 2048,
    contextWindow: 128000,
    streamResponses: true,
    toolCalling: true,
    safetyMode: true,
    retries: 2,
    concurrency: 2,
    prompt: "You are a precise coding agent. Use tools, summarize changes, and avoid unsafe commands.",
    contextPolicy: "adaptive",
    memoryMode: "session",
    enableCaching: true,
    maxTerminalLines: 800,
    redactSecrets: true,
    auditLogs: true,
    toolToggles: defaultToolToggles.map(({ id, enabled }) => ({ id, enabled })),
  };
}

const activeProfile = ref<LLMProfile>(defaultProfile());
const toolToggles = ref<ToolToggle[]>([...defaultToolToggles]);
const status = ref<"idle" | "loading" | "saving" | "saved" | "error">("idle");
const message = ref("");

function mergeToggles(stored: Array<{ id: string; enabled: boolean }> | undefined) {
  if (!stored || stored.length === 0) {
    return [...defaultToolToggles];
  }
  const map = new Map(stored.map((tool) => [tool.id, tool.enabled]));
  return defaultToolToggles.map((tool) => ({
    ...tool,
    enabled: map.get(tool.id) ?? tool.enabled,
  }));
}

async function loadProfile() {
  status.value = "loading";
  message.value = "";
  try {
    const profile = (await invoke("llm_get_profile")) as LLMProfile | null;
    if (profile) {
      activeProfile.value = profile;
      toolToggles.value = mergeToggles(profile.toolToggles);
      status.value = "saved";
      message.value = `Loaded ${profile.profileName}`;
    } else {
      activeProfile.value = defaultProfile();
      toolToggles.value = [...defaultToolToggles];
      status.value = "idle";
      message.value = "No profile stored yet. Using defaults.";
    }
  } catch (error) {
    status.value = "error";
    message.value = error instanceof Error ? error.message : String(error);
  }
}

async function saveProfile() {
  status.value = "saving";
  message.value = "";
  try {
    activeProfile.value.toolToggles = toolToggles.value.map((tool) => ({
      id: tool.id,
      enabled: tool.enabled,
    }));
    const saved = (await invoke("llm_save_profile", {
      profile: activeProfile.value,
    })) as LLMProfile;
    activeProfile.value = saved;
    toolToggles.value = mergeToggles(saved.toolToggles);
    status.value = "saved";
    message.value = `Saved ${saved.profileName}`;
  } catch (error) {
    status.value = "error";
    message.value = error instanceof Error ? error.message : String(error);
  }
}

onMounted(() => {
  void loadProfile();
});
</script>

<template>
  <div class="settings-page">
    <header class="settings-header">
      <div>
        <p class="eyebrow">Tool Settings</p>
        <h2>Global tool permissions</h2>
        <p class="subtitle">Enable only the tools you want the agent to use.</p>
      </div>
      <div class="header-actions">
        <span class="status-pill" :data-status="status">{{ message || status }}</span>
        <button class="btn primary" type="button" @click="saveProfile" :disabled="status === 'saving'">
          Save
        </button>
      </div>
    </header>

    <section class="card">
      <ToolAllowlist :tool-toggles="toolToggles" @update:toolToggles="toolToggles = $event" />
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
}
</style>
