<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { agentStore } from "../agents/orchestrator";

type ChatRole = "user" | "assistant" | "system";

type ChatMessage = {
  id: string;
  role: ChatRole;
  content: string;
  timestamp: number;
};

const { state, initKernelStore, userInput } = agentStore;

const systemMessage: ChatMessage = {
  id: "system-1",
  role: "system",
  content: "Messages are routed to the local kernel. Use Agent panel for execution details.",
  timestamp: Date.now(),
};

const messages = ref<ChatMessage[]>([systemMessage]);
const input = ref("");
const inputRef = ref<HTMLTextAreaElement | null>(null);
const listRef = ref<HTMLDivElement | null>(null);
const sendError = ref("");
const agentState = computed(() => state.run?.agentState ?? "IDLE");
const runMessages = computed(() => state.run?.messages ?? []);
const lastCount = ref(0);
const lastRunId = ref<string | null>(null);

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

async function sendMessage() {
  const value = input.value.trim();
  if (!value) return;
  sendError.value = "";
  input.value = "";
  try {
    await userInput(value);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    sendError.value = message || "Unable to send message.";
  }
}

function resetMessages() {
  messages.value = [systemMessage];
  lastCount.value = 0;
}

function syncMessages() {
  const list = runMessages.value;
  if (list.length < lastCount.value) {
    resetMessages();
  }
  for (let i = lastCount.value; i < list.length; i += 1) {
    const msg = list[i];
    const role = (msg.role as ChatRole) || "user";
    messages.value.push({
      id: `msg-${i}`,
      role,
      content: msg.content,
      timestamp: Date.now(),
    });
  }
  lastCount.value = list.length;
  void nextTick().then(scrollToBottom);
}

onMounted(async () => {
  await initKernelStore();
  lastRunId.value = state.run?.runId ?? null;
  syncMessages();
  window.addEventListener("focus-chat-input", handleFocusInput);
});

onBeforeUnmount(() => {
  window.removeEventListener("focus-chat-input", handleFocusInput);
});

watch(
  () => state.run?.runId ?? null,
  (value) => {
    if (value !== lastRunId.value) {
      lastRunId.value = value;
      resetMessages();
    }
  },
);

watch(
  () => runMessages.value.length,
  () => {
    syncMessages();
  },
);
</script>

<template>
  <div class="chat-panel">
    <header class="chat-header">
      <div>
        <p class="eyebrow">Conversation</p>
        <h3>Agent chat</h3>
      </div>
      <span class="status-pill" :data-state="agentState">{{ agentState }}</span>
    </header>

    <div ref="listRef" class="chat-messages">
      <div v-for="message in messages" :key="message.id" class="chat-message" :data-role="message.role">
        <div class="message-meta">
          <span class="role">{{ message.role }}</span>
          <span class="time">{{ formatTime(message.timestamp) }}</span>
        </div>
        <p>{{ message.content }}</p>
      </div>
    </div>

    <div class="chat-input">
      <textarea
        ref="inputRef"
        v-model="input"
        rows="2"
        placeholder="Type a request for the agent (Ctrl+Enter to send)"
        @keydown.ctrl.enter.prevent="sendMessage"
      ></textarea>
      <div class="chat-actions">
        <span class="hint">Ctrl+Enter to send</span>
        <div class="action-buttons">
          <button class="btn ghost" type="button" disabled title="Planned">
            Insert output
          </button>
          <button class="btn primary" type="button" :disabled="!input.trim()" @click="sendMessage">
            Send
          </button>
        </div>
      </div>
      <p v-if="sendError" class="error-text">{{ sendError }}</p>
    </div>
  </div>
</template>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
  padding: 14px;
  border-radius: 16px;
  background: transparent;
  border: none;
  box-shadow: none;
}

.chat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.chat-header h3 {
  margin: 0;
  font-size: 1.05rem;
}

.status-pill {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(138, 160, 183, 0.3);
  color: #8aa0b7;
  background: rgba(138, 160, 183, 0.12);
}

.status-pill[data-state="RUNNING"] {
  color: #2df6ff;
  border-color: rgba(45, 246, 255, 0.4);
  background: rgba(45, 246, 255, 0.12);
}

.status-pill[data-state="AWAITING_USER"] {
  color: #b6ff4b;
  border-color: rgba(182, 255, 75, 0.4);
  background: rgba(182, 255, 75, 0.12);
}

.status-pill[data-state="ERROR"] {
  color: #ffb84d;
  border-color: rgba(255, 184, 77, 0.4);
  background: rgba(255, 184, 77, 0.12);
}

.chat-messages {
  flex: 1;
  min-height: 0;
  display: grid;
  gap: 10px;
  overflow: auto;
  padding-right: 4px;
  padding-bottom: 6px;
}

.chat-message {
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(8, 12, 20, 0.72);
  border: 1px solid var(--line);
  color: #c7d7ec;
}

.chat-message[data-role="user"] {
  border-color: rgba(45, 246, 255, 0.35);
  box-shadow: 0 0 16px rgba(45, 246, 255, 0.08);
}

.chat-message[data-role="assistant"] {
  border-color: rgba(182, 255, 75, 0.35);
}

.chat-message[data-role="system"] {
  border-color: rgba(138, 160, 183, 0.35);
  color: #9bb0c6;
}

.message-meta {
  display: flex;
  gap: 10px;
  align-items: center;
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: #8aa0b7;
  margin-bottom: 6px;
}

.message-meta .role {
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(9, 14, 22, 0.8);
}

.message-meta .time {
  font-variant-numeric: tabular-nums;
}

.chat-message p {
  margin: 0;
  font-size: 0.85rem;
  line-height: 1.5;
}

.chat-input {
  display: grid;
  gap: 8px;
  padding-top: 4px;
}

.chat-input textarea {
  border-radius: 12px;
  border: 1px solid var(--line);
  padding: 10px 12px;
  background: var(--panel-glass);
  color: #e6f3ff;
  font-family: inherit;
  resize: vertical;
}

.chat-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.action-buttons {
  display: flex;
  gap: 8px;
}

.hint {
  font-size: 0.7rem;
  color: #8aa0b7;
}

.error-text {
  margin: 0;
  font-size: 0.75rem;
  color: #ffb84d;
}

.btn {
  border-radius: 999px;
  border: 1px solid var(--line);
  padding: 8px 14px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  background: var(--panel-glass);
  color: #c7d7ec;
  cursor: pointer;
}

.btn.primary {
  background: linear-gradient(135deg, rgba(45, 246, 255, 0.9), rgba(74, 125, 255, 0.9));
  color: #05060a;
  border-color: transparent;
  box-shadow: 0 0 18px rgba(45, 246, 255, 0.35);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn.ghost:hover {
  border-color: rgba(45, 246, 255, 0.5);
  color: #2df6ff;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: #2df6ff;
}
</style>
