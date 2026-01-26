<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { agentStore } from "../agents/orchestrator";
import PanelShell from "./PanelShell.vue";

type ChatRole = "user" | "assistant" | "system";

type ChatMessage = {
  id: string;
  role: ChatRole;
  content: string;
  timestamp: number;
  toolCalls?: ChatToolCall[];
};

type ChatToolCall = {
  id: string;
  tool: string;
  detail: string;
  status: "running" | "ok" | "error";
  output?: string;
  summary?: string | null;
};

const { state, initKernelStore, userInput, stop } = agentStore;

const systemMessage: ChatMessage = {
  id: "system-1",
  role: "system",
  content: "Messages are routed to the local kernel. Use Agent panel for execution details.",
  timestamp: Date.now(),
};

const input = ref("");
const inputRef = ref<HTMLTextAreaElement | null>(null);
const listRef = ref<HTMLDivElement | null>(null);
const sendError = ref("");
const isAtBottom = ref(true);
const agentState = computed(() => state.run?.agentState ?? "IDLE");
const isAwaiting = computed(() => agentState.value === "AWAITING_USER");
const chatEntries = computed(() => state.chatEntries ?? []);
const displayEntries = computed<ChatMessage[]>(() => [systemMessage, ...chatEntries.value]);
const latestEventId = computed(() => state.events[0]?.id ?? "");

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}

function scrollToBottom() {
  const el = listRef.value;
  if (!el) return;
  el.scrollTop = el.scrollHeight;
  isAtBottom.value = true;
}

function handleFocusInput() {
  inputRef.value?.focus();
}

function updateScrollState() {
  const el = listRef.value;
  if (!el) return;
  const threshold = 24;
  isAtBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - threshold;
}

async function handleContinue() {
  sendError.value = "";
  try {
    await userInput("继续");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    sendError.value = message || "Unable to continue.";
  }
}

async function handleStop() {
  sendError.value = "";
  try {
    await stop();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    sendError.value = message || "Unable to stop.";
  }
}

function resolveChatPayload(value: string, forceChatOnly: boolean) {
  if (forceChatOnly) {
    return { content: value, chatOnly: true };
  }
  const trimmed = value.trim();
  if (trimmed.toLowerCase().startsWith("/chat")) {
    const content = trimmed.slice(5).trimStart();
    return { content, chatOnly: true };
  }
  return { content: value, chatOnly: false };
}

async function sendMessage(forceChatOnly = false) {
  const value = input.value.trim();
  if (!value) return;
  sendError.value = "";
  const payload = resolveChatPayload(value, forceChatOnly);
  if (!payload.content.trim()) {
    sendError.value = "Chat message cannot be empty.";
    return;
  }
  input.value = "";
  try {
    await userInput(payload.content, payload.chatOnly ? { chatOnly: true } : undefined);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    sendError.value = message || "Unable to send message.";
  }
}

function statusClass(status: ChatToolCall["status"]) {
  if (status === "ok") return "tool-status tool-status--ok";
  if (status === "error") return "tool-status tool-status--error";
  return "tool-status tool-status--running";
}

onMounted(async () => {
  await initKernelStore();
  window.addEventListener("focus-chat-input", handleFocusInput);
  await nextTick();
  scrollToBottom();
  listRef.value?.addEventListener("scroll", updateScrollState);
  updateScrollState();
});

onBeforeUnmount(() => {
  window.removeEventListener("focus-chat-input", handleFocusInput);
  listRef.value?.removeEventListener("scroll", updateScrollState);
});

watch(
  () => latestEventId.value,
  () => {
    void nextTick().then(scrollToBottom);
  },
);
</script>

<template>
  <PanelShell title="Chat / Agent Interaction" subtitle="Conversation stream" :no-padding="true">
    <template #actions>
      <span class="state-chip" :data-state="agentState">{{ agentState }}</span>
    </template>

    <div class="chat-panel">
      <div class="conversation-header">
        <div>
          <p class="conversation-eyebrow">Execution loop</p>
          <p class="conversation-title">Plan -> Act -> Observe</p>
        </div>
        <div class="conversation-meta">
          <span class="conversation-meta__label">Session</span>
          <span class="conversation-meta__value">{{ agentState }}</span>
        </div>
      </div>

      <div v-if="isAwaiting" class="conversation-alert">
        <div>
          <p class="alert-title">Action required</p>
          <p class="alert-text">The agent is waiting for confirmation or input.</p>
        </div>
        <div class="alert-actions">
          <button class="btn primary" type="button" @click="handleContinue">Continue</button>
          <button class="btn ghost" type="button" @click="handleStop">Stop</button>
        </div>
      </div>

      <div ref="listRef" class="conversation-content">
        <div
          v-for="message in displayEntries"
          :key="message.id"
          class="message"
          :data-role="message.role"
        >
          <div class="message-avatar">
            <span v-if="message.role === 'assistant'">AI</span>
            <span v-else-if="message.role === 'user'">U</span>
            <span v-else>SYS</span>
          </div>

          <div class="message-body">
            <div class="message-meta">
              <span class="message-role">{{ message.role }}</span>
              <span class="message-time">{{ formatTime(message.timestamp) }}</span>
            </div>

            <div class="message-content">
              {{ message.content }}
            </div>

            <div v-if="message.toolCalls && message.toolCalls.length" class="message-tools">
              <details
                v-for="call in message.toolCalls"
                :key="call.id"
                class="tool-card"
                :open="call.status === 'running'"
              >
                <summary class="tool-summary">
                  <span class="tool-name">{{ call.tool }}</span>
                  <span class="tool-chip" :class="statusClass(call.status)">
                    {{ call.status }}
                  </span>
                  <span class="tool-detail-text">{{ call.detail }}</span>
                </summary>
                <div class="tool-body">
                  <pre v-if="call.output">{{ call.output }}</pre>
                  <p v-else class="tool-empty">No output yet.</p>
                  <p v-if="call.summary" class="tool-summary-text">{{ call.summary }}</p>
                </div>
              </details>
            </div>
          </div>
        </div>
        <button
          v-if="!isAtBottom"
          type="button"
          class="conversation-scroll"
          @click="scrollToBottom"
        >
          Bottom
        </button>
      </div>

      <div class="prompt-input">
        <div class="prompt-input__field">
          <textarea
            ref="inputRef"
            v-model="input"
            rows="2"
            placeholder="Type a request for the agent (/chat for chat-only)"
            class="prompt-input__textarea"
            @keydown.ctrl.enter.prevent="sendMessage"
          ></textarea>
        </div>
        <div class="prompt-input__actions">
          <button class="btn ghost" type="button" :disabled="!input.trim()" @click="sendMessage(true)">
            Chat only
          </button>
          <button class="btn primary" type="button" :disabled="!input.trim()" @click="sendMessage()">
            Send
          </button>
        </div>
        <div class="prompt-input__meta">
          <span>Ctrl+Enter to send - /chat for chat-only</span>
          <span>Orchestrator: <span class="text-status-success">Online</span></span>
        </div>
        <p v-if="sendError" class="prompt-input__error">{{ sendError }}</p>
      </div>
    </div>
  </PanelShell>
</template>

<style scoped>
summary::-webkit-details-marker {
  display: none;
}

summary::marker {
  content: "";
}

.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  gap: 12px;
}

.state-chip {
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  font-size: 0.65rem;
  letter-spacing: 0.04em;
  color: var(--text-secondary);
}

.state-chip[data-state="RUNNING"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.12);
}

.state-chip[data-state="AWAITING_USER"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.4);
  background: rgba(var(--status-warning-rgb), 0.12);
}

.state-chip[data-state="ERROR"] {
  color: var(--status-error);
  border-color: rgba(var(--status-error-rgb), 0.4);
  background: rgba(var(--status-error-rgb), 0.12);
}

.state-chip[data-state="PAUSED"] {
  color: var(--status-info);
  border-color: rgba(var(--status-info-rgb), 0.4);
  background: rgba(var(--status-info-rgb), 0.12);
}

.conversation-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 14px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
}

.conversation-eyebrow {
  margin: 0 0 4px;
  font-size: 0.65rem;
  color: var(--text-tertiary);
  letter-spacing: 0.04em;
}

.conversation-title {
  margin: 0;
  font-size: 0.95rem;
  color: var(--text-primary);
  font-weight: 600;
}

.conversation-meta {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.08);
  font-size: 0.7rem;
  color: var(--text-secondary);
}

.conversation-meta__value {
  font-weight: 600;
  color: var(--text-primary);
}

.conversation-alert {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 14px;
  border: 1px solid rgba(var(--status-warning-rgb), 0.35);
  background: rgba(var(--status-warning-rgb), 0.1);
}

.alert-title {
  margin: 0 0 4px;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-primary);
}

.alert-text {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.alert-actions {
  display: flex;
  gap: 8px;
}

.conversation-content {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
  border-radius: 16px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.04);
}

.conversation-scroll {
  align-self: flex-end;
  position: sticky;
  bottom: 12px;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.4);
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent);
  font-size: 0.7rem;
  cursor: pointer;
}

.message {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}

.message[data-role="user"] {
  flex-direction: row-reverse;
}

.message-avatar {
  height: 32px;
  width: 32px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  display: grid;
  place-items: center;
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--text-primary);
}

.message[data-role="assistant"] .message-avatar {
  border-color: rgba(var(--accent-rgb), 0.4);
  color: var(--accent);
}

.message[data-role="system"] .message-avatar {
  border-color: rgba(var(--status-warning-rgb), 0.4);
  color: var(--status-warning);
}

.message-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 75%;
}

.message[data-role="user"] .message-body {
  align-items: flex-end;
}

.message-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.65rem;
  color: var(--text-tertiary);
}

.message[data-role="user"] .message-meta {
  justify-content: flex-end;
}

.message-role {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.16);
  background: rgba(var(--line-rgb), 0.06);
  color: var(--text-secondary);
  text-transform: capitalize;
}

.message-time {
  font-size: 0.65rem;
  color: var(--text-tertiary);
}

.message-content {
  padding: 10px 14px;
  border-radius: 14px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-primary);
  font-size: 0.85rem;
  line-height: 1.6;
  white-space: pre-wrap;
}

.message[data-role="user"] .message-content {
  border-color: rgba(var(--accent-rgb), 0.35);
  background: rgba(var(--accent-rgb), 0.16);
}

.message[data-role="system"] .message-content {
  background: rgba(var(--line-rgb), 0.05);
  color: var(--text-secondary);
}

.message-tools {
  display: grid;
  gap: 8px;
}

.tool-card {
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.16);
  background: rgba(var(--line-rgb), 0.06);
  overflow: hidden;
}

.tool-summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  font-size: 0.7rem;
  color: var(--text-secondary);
  cursor: pointer;
  list-style: none;
}

.tool-name {
  font-weight: 600;
  color: var(--accent);
}

.tool-chip {
  padding: 2px 6px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.tool-status {
  letter-spacing: 0.06em;
}

.tool-status--ok {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.4);
}

.tool-status--error {
  color: var(--status-error);
  border-color: rgba(var(--status-error-rgb), 0.4);
}

.tool-status--running {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.tool-detail-text {
  color: var(--text-tertiary);
}

.tool-body {
  border-top: 1px solid rgba(var(--line-rgb), 0.14);
  padding: 10px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.tool-body pre {
  margin: 0 0 8px;
  white-space: pre-wrap;
  font-family: var(--font-body);
  font-size: 0.75rem;
  color: var(--text-primary);
  background: rgba(var(--line-rgb), 0.08);
  border: 1px solid rgba(var(--line-rgb), 0.16);
  border-radius: 10px;
  padding: 8px;
}

.tool-empty {
  margin: 0;
  color: var(--text-tertiary);
}

.tool-summary-text {
  margin: 6px 0 0;
  color: var(--text-tertiary);
}

.prompt-input {
  border-radius: 16px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
  padding: 12px 14px;
  display: grid;
  gap: 10px;
}

.prompt-input__field {
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.05);
  padding: 8px 10px;
}

.prompt-input__textarea {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 0.85rem;
  resize: none;
  outline: none;
}

.prompt-input__textarea::placeholder {
  color: var(--text-tertiary);
}

.prompt-input__actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  flex-wrap: wrap;
}

.prompt-input__meta {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  font-size: 0.65rem;
  color: var(--text-tertiary);
}

.prompt-input__error {
  margin: 0;
  font-size: 0.7rem;
  color: var(--status-warning);
}
</style>


