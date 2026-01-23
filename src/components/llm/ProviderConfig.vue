<script setup lang="ts">
import { computed, ref, watch } from "vue";

type ProviderId = "openai" | "anthropic" | "local" | "ollama" | "azure";

interface ProviderConfig {
  apiKey: string;
  baseUrl: string;
  model: string;
}

interface Props {
  provider: ProviderId;
  model: string;
  apiKey: string;
  baseUrl: string;
  ollamaModels: string[];
  openAiModels: string[];
  modelFetchStatus: "idle" | "loading" | "ok" | "error";
  modelFetchMessage: string;
  modelFetchAt: number | null;
  modelTestStatus: "idle" | "running" | "ok" | "error";
  modelTestMessage: string;
  modelTestDetail: string;
  openAiFetchStatus: "idle" | "loading" | "ok" | "error";
  openAiFetchMessage: string;
  openAiFetchAt: number | null;
}

interface Emits {
  (e: "update:provider", value: ProviderId): void;
  (e: "update:model", value: string): void;
  (e: "update:apiKey", value: string): void;
  (e: "update:baseUrl", value: string): void;
  (e: "fetchOllamaModels"): void;
  (e: "testOllamaModel"): void;
  (e: "fetchOpenAiModels"): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const providerOptions = [
  { id: "openai" as const, label: "OpenAI" },
  { id: "anthropic" as const, label: "Anthropic" },
  { id: "local" as const, label: "Local (Ollama/LM Studio)" },
  { id: "ollama" as const, label: "Ollama (LAN)" },
  { id: "azure" as const, label: "Azure OpenAI" },
];

const modelsByProvider: Record<ProviderId, string[]> = {
  openai: [],
  anthropic: ["claude-3.5-sonnet", "claude-3.5-haiku", "claude-3-opus"],
  local: ["llama3.1:70b", "qwen2.5:32b", "mistral-large"],
  ollama: ["llama3.1:70b", "qwen2.5:32b", "mistral-large"],
  azure: ["gpt-4o", "gpt-4.1", "gpt-35-turbo"],
};

const isOpenAiProvider = computed(() => props.provider === "openai");
const isLocalProvider = computed(() => props.provider === "local" || props.provider === "ollama");

const modelOptions = computed(() => {
  let baseOptions = modelsByProvider[props.provider] ?? [];
  if (isLocalProvider.value && props.ollamaModels.length) {
    baseOptions = props.ollamaModels;
  } else if (isOpenAiProvider.value && props.openAiModels.length) {
    baseOptions = props.openAiModels;
  }
  const current = props.model.trim();
  if (current && !baseOptions.includes(current)) {
    return [current, ...baseOptions];
  }
  return baseOptions;
});

const useModelSelect = computed(
  () => !isOpenAiProvider.value || props.openAiModels.length > 0,
);

const baseUrlPlaceholder = computed(() => {
  if (props.provider === "ollama") return "http://<LAN-IP>:11434";
  if (props.provider === "local") return "http://localhost:11434";
  if (props.provider === "azure") return "https://{resource}.openai.azure.com";
  return "https://api.openai.com/v1";
});

const modelFetchLabel = computed(() => {
  if (props.modelFetchStatus === "loading") return "Loading";
  if (props.modelFetchStatus === "ok") return "Models loaded";
  if (props.modelFetchStatus === "error") return "Load failed";
  return "Idle";
});

const modelTestLabel = computed(() => {
  if (props.modelTestStatus === "running") return "Testing";
  if (props.modelTestStatus === "ok") return "Model ready";
  if (props.modelTestStatus === "error") return "Model error";
  return "Idle";
});

const openAiFetchLabel = computed(() => {
  if (props.openAiFetchStatus === "loading") return "Loading";
  if (props.openAiFetchStatus === "ok") return "Models loaded";
  if (props.openAiFetchStatus === "error") return "Load failed";
  return "Idle";
});

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}

const modelFetchTime = computed(() =>
  props.modelFetchAt ? formatTime(props.modelFetchAt) : "",
);

const openAiFetchTime = computed(() =>
  props.openAiFetchAt ? formatTime(props.openAiFetchAt) : "",
);
</script>

<template>
  <div class="provider-config">
    <div class="field-grid">
      <label>
        <span>Provider</span>
        <select :value="provider" @change="$emit('update:provider', ($event.target as HTMLSelectElement).value as ProviderId)">
          <option v-for="item in providerOptions" :key="item.id" :value="item.id">
            {{ item.label }}
          </option>
        </select>
      </label>
      <label>
        <span>Model</span>
        <select v-if="useModelSelect" :value="model" @change="$emit('update:model', ($event.target as HTMLSelectElement).value)">
          <option v-for="item in modelOptions" :key="item" :value="item">
            {{ item }}
          </option>
        </select>
        <input
          v-else
          :value="model"
          @input="$emit('update:model', ($event.target as HTMLInputElement).value)"
          type="text"
          placeholder="Enter OpenAI model ID"
        />
      </label>
      <label class="full">
        <span>Base URL</span>
        <input 
          :value="baseUrl" 
          @input="$emit('update:baseUrl', ($event.target as HTMLInputElement).value)"
          type="text" 
          :placeholder="baseUrlPlaceholder" 
        />
      </label>
      <label class="full">
        <span>API Key</span>
        <input 
          :value="apiKey" 
          @input="$emit('update:apiKey', ($event.target as HTMLInputElement).value)"
          type="password" 
          placeholder="sk-..." 
        />
      </label>
    </div>
    
    <!-- Local Provider Tools -->
    <div v-if="isLocalProvider" class="model-tools">
      <div class="model-tools__actions">
        <button
          class="btn ghost"
          type="button"
          :disabled="modelFetchStatus === 'loading'"
          @click="$emit('fetchOllamaModels')"
        >
          {{ modelFetchStatus === "loading" ? "Loading..." : "Load models" }}
        </button>
        <button
          class="btn ghost"
          type="button"
          :disabled="modelTestStatus === 'running'"
          @click="$emit('testOllamaModel')"
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
          @click="$emit('update:model', item)"
        >
          {{ item }}
        </button>
      </div>
      <p v-else class="hint">No models loaded yet.</p>
    </div>
    
    <!-- OpenAI Provider Tools -->
    <div v-if="isOpenAiProvider" class="model-tools">
      <div class="model-tools__actions">
        <button
          class="btn ghost"
          type="button"
          :disabled="openAiFetchStatus === 'loading'"
          @click="$emit('fetchOpenAiModels')"
        >
          {{ openAiFetchStatus === "loading" ? "Loading..." : "Load OpenAI models" }}
        </button>
        <span class="pill" :data-status="openAiFetchStatus">{{ openAiFetchLabel }}</span>
      </div>
      <div v-if="openAiFetchMessage" class="hint">{{ openAiFetchMessage }}</div>
      <div v-if="openAiFetchTime" class="hint">Last sync: {{ openAiFetchTime }}</div>
      <p v-if="!openAiModels.length && !openAiFetchMessage" class="hint">
        No models loaded yet.
      </p>
    </div>
  </div>
</template>

<style scoped>
.provider-config {
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: relative;
  z-index: 1;
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.field-grid label {
  display: grid;
  gap: 6px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-secondary);
  font-family: var(--font-display);
}

.field-grid label.full {
  grid-column: span 2;
}

select,
input {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  padding: 10px 12px;
  background: rgba(3, 12, 24, 0.9);
  color: var(--text-primary);
  font-family: inherit;
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
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  background: rgba(7, 12, 22, 0.8);
  color: var(--text-soft);
  padding: 6px 10px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  cursor: pointer;
  transition: all 0.2s ease;
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
  border-radius: 0;
  background: rgba(5, 8, 14, 0.75);
  border: 1px solid rgba(var(--line-rgb), 0.4);
  font-size: 0.7rem;
  color: var(--text-soft);
  font-family: var(--font-body);
  white-space: pre-wrap;
  max-height: 160px;
  overflow: auto;
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

.hint {
  font-size: 0.78rem;
  color: var(--text-secondary);
  position: relative;
  z-index: 1;
}

.pill {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  padding: 6px 10px;
  border-radius: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.4);
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

@media (max-width: 1100px) {
  .field-grid {
    grid-template-columns: 1fr;
  }

  .field-grid label.full {
    grid-column: auto;
  }
}
</style>
