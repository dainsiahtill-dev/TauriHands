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
}

function handleFocusInput() {
  inputRef.value?.focus();
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

function avatarClass(role: ChatRole) {
  if (role === "assistant") return "chat-avatar chat-avatar--assistant";
  if (role === "user") return "chat-avatar chat-avatar--user";
  return "chat-avatar chat-avatar--system";
}

function bubbleClass(role: ChatRole) {
  if (role === "user") return "chat-bubble chat-bubble--user";
  if (role === "assistant") return "chat-bubble chat-bubble--assistant";
  return "chat-bubble chat-bubble--system";
}

function statusClass(status: ChatToolCall["status"]) {
  if (status === "ok") return "tool-status tool-status--ok";
  if (status === "error") return "tool-status tool-status--error";
  return "tool-status tool-status--running";
}

onMounted(async () => {
  await initKernelStore();
  window.addEventListener("focus-chat-input", handleFocusInput);
  void nextTick().then(scrollToBottom);
});

onBeforeUnmount(() => {
  window.removeEventListener("focus-chat-input", handleFocusInput);
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
      <div class="chat-banner">
        <span>Execution Loop</span>
        <strong>Plan -> Act -> Observe</strong>
      </div>
      <div
        v-if="isAwaiting"
        class="mx-4 mt-2 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-accent-lime/40 bg-accent-lime/10 px-4 py-3"
      >
        <div>
          <p class="text-[11px] uppercase tracking-[0.2em] text-accent-lime">Action required</p>
          <p class="text-sm text-text-main">The agent is waiting for confirmation or input.</p>
        </div>
        <div class="flex gap-2">
          <button class="btn primary" type="button" @click="handleContinue">Continue</button>
          <button class="btn ghost" type="button" @click="handleStop">Stop</button>
        </div>
      </div>

      <div ref="listRef" class="chat-list">
        <div
          v-for="message in displayEntries"
          :key="message.id"
          class="chat-entry"
          :class="message.role === 'user' ? 'is-user' : ''"
        >
          <div
            :class="avatarClass(message.role)"
          >
            <span v-if="message.role === 'assistant'">AI</span>
            <span v-else-if="message.role === 'user'">U</span>
            <span v-else>SYS</span>
          </div>

          <div class="chat-stack">
            <div
              class="chat-meta"
              :class="message.role === 'user' ? 'is-user' : ''"
            >
              <span class="chat-role">
                {{ message.role }}
              </span>
              <span class="chat-time">{{ formatTime(message.timestamp) }}</span>
            </div>

            <div :class="bubbleClass(message.role)">
              {{ message.content }}
            </div>

            <div v-if="message.toolCalls && message.toolCalls.length" class="space-y-2">
              <details
                v-for="call in message.toolCalls"
                :key="call.id"
                class="tool-detail"
                :open="call.status === 'running'"
              >
                <summary
                  class="tool-summary"
                >
                  <span class="text-accent">{{ call.tool }}</span>
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
      </div>

      <div class="chat-input">
        <div class="chat-input__box">
          <textarea
            ref="inputRef"
            v-model="input"
            rows="2"
            placeholder="Type a request for the agent (/chat for chat-only)"
            class="chat-input__field"
            @keydown.ctrl.enter.prevent="sendMessage"
          ></textarea>
          <button class="btn ghost" type="button" :disabled="!input.trim()" @click="sendMessage(true)">
            Chat only
          </button>
          <button class="btn primary" type="button" :disabled="!input.trim()" @click="sendMessage()">
            Send
          </button>
        </div>
        <div class="chat-input__meta">
          <span>Ctrl+Enter to send • /chat for chat-only</span>
          <span>Orchestrator: <span class="text-status-success">Online</span></span>
        </div>
        <p v-if="sendError" class="mt-2 text-xs text-status-warning">{{ sendError }}</p>
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
  background: rgba(6, 12, 22, 0.6);
  position: relative;
  overflow: hidden;
}

.chat-panel::before {
  content: "";
  position: absolute;
  inset: 0;
  background:
    repeating-linear-gradient(0deg, rgba(255, 255, 255, 0.035) 0 1px, transparent 1px 5px);
  opacity: 0.08;
  pointer-events: none;
}

.chat-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-bottom: 1px solid rgba(var(--line-rgb), 0.28);
  background: linear-gradient(135deg, rgba(5, 12, 24, 0.95), rgba(2, 8, 16, 0.9));
  box-shadow: inset 0 0 14px rgba(var(--accent-rgb), 0.12);
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.22em;
  color: var(--text-tertiary);
  position: relative;
  z-index: 1;
  font-family: var(--font-display);
}

.chat-banner strong {
  color: var(--accent);
  font-weight: 600;
}

.state-chip {
  padding: 4px 10px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  background: rgba(8, 14, 26, 0.85);
  font-size: 0.55rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: var(--text-secondary);
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

.state-chip[data-state="RUNNING"] {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.5);
}

.state-chip[data-state="AWAITING_USER"] {
  color: var(--accent-2);
  border-color: rgba(var(--accent-2-rgb), 0.5);
}

.state-chip[data-state="ERROR"] {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.5);
}

.state-chip[data-state="PAUSED"] {
  color: var(--status-info);
  border-color: rgba(var(--status-info-rgb), 0.5);
}

.chat-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: relative;
  z-index: 1;
}

.chat-entry {
  display: flex;
  gap: 12px;
}

.chat-entry.is-user {
  flex-direction: row-reverse;
}

.chat-avatar {
  height: 36px;
  width: 36px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: linear-gradient(135deg, rgba(12, 22, 40, 0.9), rgba(7, 12, 22, 0.85));
  display: grid;
  place-items: center;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--text-primary);
}

.chat-avatar--assistant {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.5);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.25);
}

.chat-avatar--user {
  border-color: rgba(var(--line-rgb), 0.45);
}

.chat-avatar--system {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.5);
}

.chat-stack {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: 80%;
}

.chat-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.55rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: var(--text-tertiary);
}

.chat-meta.is-user {
  justify-content: flex-end;
}

.chat-role {
  padding: 2px 8px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: rgba(7, 12, 22, 0.6);
  color: var(--text-secondary);
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

.chat-time {
  font-family: var(--font-body);
  font-size: 0.6rem;
  color: var(--text-tertiary);
}

.chat-bubble {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  padding: 10px 14px;
  font-size: 0.85rem;
  line-height: 1.5;
  background: rgba(7, 12, 22, 0.78);
  color: var(--text-soft);
  box-shadow: inset 0 0 12px rgba(var(--accent-rgb), 0.08);
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

.chat-bubble--user {
  border-color: rgba(var(--accent-rgb), 0.5);
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--text-primary);
  box-shadow: 0 0 18px rgba(var(--accent-rgb), 0.18);
}

.chat-bubble--assistant {
  background: rgba(9, 16, 30, 0.85);
  color: var(--text-primary);
}

.chat-bubble--system {
  background: rgba(8, 14, 26, 0.6);
  color: var(--text-secondary);
}

.tool-detail {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.35);
  background: rgba(7, 12, 22, 0.75);
  overflow: hidden;
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

.tool-summary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  font-size: 0.55rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: var(--text-tertiary);
  cursor: pointer;
  list-style: none;
}

.tool-chip {
  padding: 2px 6px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  font-size: 0.55rem;
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

.tool-status {
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.tool-status--ok {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.5);
}

.tool-status--error {
  color: var(--status-warning);
  border-color: rgba(var(--status-warning-rgb), 0.5);
}

.tool-status--running {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.5);
}

.tool-detail-text {
  text-transform: none;
  letter-spacing: 0;
  color: var(--text-secondary);
  font-size: 0.7rem;
}

.tool-body {
  border-top: 1px solid rgba(var(--line-rgb), 0.24);
  padding: 10px;
  font-size: 0.7rem;
  color: var(--text-secondary);
}

.tool-body pre {
  margin: 0 0 8px;
  white-space: pre-wrap;
  font-family: var(--font-body);
  font-size: 0.7rem;
  color: var(--text-primary);
  background: rgba(5, 8, 14, 0.7);
  border: 1px solid rgba(var(--line-rgb), 0.24);
  border-radius: 0;
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

.chat-input {
  border-top: 1px solid rgba(var(--line-rgb), 0.3);
  background: rgba(8, 14, 26, 0.8);
  padding: 12px 16px 14px;
  position: relative;
  z-index: 1;
}

.chat-input__box {
  display: flex;
  align-items: flex-end;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  background: rgba(7, 12, 22, 0.75);
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
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

.chat-input__box:focus-within {
  border-color: rgba(var(--accent-rgb), 0.6);
  box-shadow: 0 0 18px rgba(var(--accent-rgb), 0.18);
}

.chat-input__field {
  flex: 1;
  min-height: 44px;
  resize: none;
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-size: 0.85rem;
  outline: none;
}

.chat-input__field::placeholder {
  color: var(--text-tertiary);
}

.chat-input__meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 8px;
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-tertiary);
}
</style>
