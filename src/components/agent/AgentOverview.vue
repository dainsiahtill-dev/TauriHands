<script setup lang="ts">
import StreamPreview from "../StreamPreview.vue";

interface Props {
  agentState: string;
  budgetLabel: string;
  summary: {
    steps: number;
    tasks: number;
    tools: number;
    logs: number;
    messages: number;
  };
  activeToolCall: any;
  activeToolOutput: string;
  streamContent: string;
  isStreaming: boolean;
  isThinking: boolean;
  lastAssistantMessage: string;
  lastMessage: string;
  isAwaiting: boolean;
  showError: boolean;
  errorMessage: string;
}

defineProps<Props>();
</script>

<template>
  <div class="agent-overview">
    <div class="section-title">Status</div>
    <div class="controls">
      <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
      <span class="budget-chip">Steps {{ budgetLabel }}</span>
    </div>
    <div class="summary-grid">
      <div class="summary-card">
        <span class="summary-label">Plan steps</span>
        <strong>{{ summary.steps }}</strong>
      </div>
      <div class="summary-card">
        <span class="summary-label">Tasks</span>
        <strong>{{ summary.tasks }}</strong>
      </div>
      <div class="summary-card">
        <span class="summary-label">Tool calls</span>
        <strong>{{ summary.tools }}</strong>
      </div>
      <div class="summary-card">
        <span class="summary-label">Logs</span>
        <strong>{{ summary.logs }}</strong>
      </div>
    </div>
    <div class="llm-preview">
      <div class="llm-preview-header">
        <span class="summary-label">Live activity</span>
        <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
      </div>
      <div v-if="activeToolCall" class="llm-preview-block">
        <span class="llm-preview-title">Running tool</span>
        <p class="llm-preview-text">
          {{ activeToolCall.tool }} · {{ activeToolCall.detail }}
        </p>
        <pre v-if="activeToolOutput" class="llm-preview-output">{{ activeToolOutput }}</pre>
      </div>
      <div v-else-if="isStreaming" class="llm-preview-block">
        <span class="llm-preview-title">Streaming</span>
        <StreamPreview :content="streamContent" />
      </div>
      <p v-else-if="isThinking" class="llm-preview-text">LLM is selecting next action...</p>
      <p v-else class="llm-preview-text">
        {{ lastAssistantMessage || lastMessage || "No messages yet." }}
      </p>
    </div>
    <p v-if="isAwaiting" class="awaiting-hint">Waiting for input in chat to continue.</p>
    <div v-if="showError" class="error-card">
      <div class="error-title">Last error</div>
      <pre class="error-detail">{{ errorMessage }}</pre>
    </div>
  </div>
</template>

<style scoped>
.agent-overview {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: var(--text-secondary);
  font-family: var(--font-display);
}

.controls {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  align-items: center;
}

.summary-grid {
  display: grid;
  gap: 10px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.summary-card {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  padding: 10px 12px;
  background: rgba(var(--line-rgb), 0.08);
  display: grid;
  gap: 6px;
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

.summary-label {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-secondary);
}

.summary-card strong {
  font-size: 1rem;
  color: var(--text-primary);
}

.llm-preview {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: rgba(var(--line-rgb), 0.08);
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

.llm-preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.llm-preview-text {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-soft);
  white-space: pre-wrap;
  max-height: 120px;
  overflow: auto;
}

.llm-preview-block {
  display: grid;
  gap: 6px;
}

.llm-preview-title {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-secondary);
}

.llm-preview-output {
  margin: 0;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  padding: 8px 10px;
  background: rgba(5, 8, 14, 0.75);
  color: var(--text-soft);
  font-size: 0.7rem;
  font-family: var(--font-body);
  white-space: pre-wrap;
  max-height: 140px;
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

.awaiting-hint {
  margin: 0;
  font-size: 0.75rem;
  color: var(--status-success);
}

.error-card {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 0;
  border: 1px solid rgba(var(--status-warning-rgb), 0.5);
  background: rgba(var(--status-warning-rgb), 0.12);
  display: grid;
  gap: 6px;
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

.error-title {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--status-warning);
}

.error-detail {
  margin: 0;
  max-height: 160px;
  overflow: auto;
  font-size: 0.75rem;
  white-space: pre-wrap;
  color: var(--status-warning);
  font-family: var(--font-body);
}

.phase-chip,
.budget-chip {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 6px 10px;
  border-radius: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.35);
  color: var(--text-soft);
  background: rgba(var(--line-rgb), 0.1);
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
</style>
