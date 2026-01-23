<script setup lang="ts">
import { computed } from "vue";
import StreamPreview from "../StreamPreview.vue";

interface Message {
  role: string;
  content: string;
}

interface Props {
  agentState: string;
  run: any;
  activeToolCall: any;
  activeToolOutput: string;
  streamContent: string;
  isStreaming: boolean;
  isThinking: boolean;
  lastAssistantMessage: string;
  lastMessage: string;
  llmMessages: Message[];
}

const props = defineProps<Props>();
const visibleMessages = computed(() => props.llmMessages.slice(-20));
</script>

<template>
  <div class="llm-live">
    <div class="section-title">LLM live</div>
    <div class="controls">
      <span class="phase-chip" :data-phase="agentState">{{ agentState }}</span>
      <span class="budget-chip">Turn {{ run?.turn ?? 0 }}</span>
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
    <div class="llm-messages">
      <div
        v-for="(message, index) in visibleMessages"
        :key="`llm-${index}`"
        class="llm-message"
        :data-role="message.role"
      >
        <span class="llm-role">{{ message.role }}</span>
        <p>{{ message.content }}</p>
      </div>
      <p v-if="!visibleMessages.length" class="empty-text">No messages yet.</p>
    </div>
  </div>
</template>

<style scoped>
.llm-live {
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

.llm-preview {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: rgba(8, 12, 20, 0.75);
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

.llm-messages {
  display: grid;
  gap: 10px;
  max-height: 360px;
  overflow: auto;
}

.llm-message {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  padding: 10px 12px;
  background: rgba(8, 12, 20, 0.85);
  display: grid;
  gap: 6px;
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

.llm-message[data-role="assistant"] {
  border-color: rgba(var(--status-success-rgb), 0.35);
}

.llm-message[data-role="user"] {
  border-color: rgba(var(--accent-rgb), 0.35);
}

.llm-message[data-role="system"] {
  border-color: rgba(var(--text-secondary-rgb), 0.35);
}

.llm-role {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  color: var(--text-secondary);
}

.llm-message p {
  margin: 0;
  font-size: 0.8rem;
  white-space: pre-wrap;
}

.summary-label {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-secondary);
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
  background: rgba(8, 12, 20, 0.85);
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

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
</style>
