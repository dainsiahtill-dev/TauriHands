<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "xterm/css/xterm.css";
import { terminalStore } from "../stores/terminal";

type TerminalOutputEvent = {
  session_id: string;
  data_base64: string;
};

type TerminalSessionInfo = {
  id: string;
  title: string;
  cwd: string;
  shell: string;
  cols: number;
  rows: number;
  created_at_ms: number;
  is_alive: boolean;
};

type TerminalReplayResponse = {
  data_base64: string;
};

type SessionState = "starting" | "live" | "error";

type TerminalSession = TerminalSessionInfo & {
  state: SessionState;
  hasUnread: boolean;
};

const terminalRef = ref<HTMLDivElement | null>(null);
const sessions = ref<TerminalSession[]>([]);
const activeSessionId = ref<string | null>(null);
const activeSession = computed(() =>
  sessions.value.find((session) => session.id === activeSessionId.value),
);
const status = computed<"idle" | SessionState>(
  () => activeSession.value?.state ?? "idle",
);
const capturedOutput = ref("");
const textDecoder = new TextDecoder();

let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let unlisten: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;
let sessionCounter = 1;
const dragSessionId = ref<string | null>(null);

function cssVar(name: string, fallback: string) {
  if (typeof window === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function cssRgbVar(name: string, fallback: string) {
  if (typeof window === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function nextTitle(base?: string) {
  if (base && base.trim()) return base.trim();
  const title = `Session ${sessionCounter}`;
  sessionCounter += 1;
  return title;
}

function markSessionUnread(sessionId: string) {
  const session = sessions.value.find((item) => item.id === sessionId);
  if (session) {
    session.hasUnread = true;
  }
}

async function createSession(options?: { shell?: string; cwd?: string; title?: string }) {
  if (!term) return null;
  const title = nextTitle(options?.title);
  try {
    const info = (await invoke("terminal_create_session", {
      shell: options?.shell ?? null,
      shell_args: null,
      cwd: options?.cwd ?? null,
      cols: term.cols || 120,
      rows: term.rows || 30,
      title,
    })) as TerminalSessionInfo;
    const session: TerminalSession = {
      ...info,
      state: "live",
      hasUnread: false,
    };
    sessions.value.push(session);
    setActiveSession(session.id);
    return session;
  } catch (error) {
    term.write("\r\n[terminal error] Unable to start PTY session.\r\n");
    return null;
  }
}

function setActiveSession(id: string) {
  if (activeSessionId.value === id) return;
  activeSessionId.value = id;
  terminalStore.setActiveSessionId(id);
  const session = sessions.value.find((item) => item.id === id);
  if (session) {
    session.hasUnread = false;
  }
  capturedOutput.value = "";
  term?.reset();
  void replayOutput();
  resizeTerminal();
}

async function sendInput(data: string) {
  if (!activeSessionId.value) return;
  const bytes = new TextEncoder().encode(data);
  await invoke("terminal_write", {
    session_id: activeSessionId.value,
    data_base64: bytesToBase64(bytes),
  });
}

async function closeSession(sessionId: string) {
  await invoke("terminal_kill", { session_id: sessionId });
  const remaining = sessions.value.filter((item) => item.id !== sessionId);
  sessions.value = remaining;
  if (activeSessionId.value === sessionId) {
    if (remaining.length > 0) {
      setActiveSession(remaining[0].id);
    } else {
      activeSessionId.value = null;
      terminalStore.setActiveSessionId(null);
      term?.reset();
      await createSession();
    }
  }
}

async function createNewSession() {
  await createSession();
}

function renameActive() {
  const session = activeSession.value;
  if (!session) return;
  const next = window.prompt("Rename session", session.title);
  if (next && next.trim()) {
    const title = next.trim();
    session.title = title;
    void invoke("terminal_set_title", { session_id: session.id, title });
  }
}

async function cloneActive() {
  const session = activeSession.value;
  if (!session) return;
  await createSession({
    shell: session.shell,
    cwd: session.cwd,
    title: `Copy of ${session.title}`,
  });
}

async function replayOutput() {
  if (!activeSessionId.value || !term) return;
  const response = (await invoke("terminal_replay", {
    session_id: activeSessionId.value,
    max_bytes: 24000,
  })) as TerminalReplayResponse;
  const bytes = base64ToBytes(response.data_base64);
  term.reset();
  term.write(textDecoder.decode(bytes));
}

async function syncOrder() {
  const order = sessions.value.map((session) => session.id);
  try {
    const normalized = (await invoke("terminal_set_order", {
      session_ids: order,
    })) as string[];
    sessions.value = normalized
      .map((id) => sessions.value.find((session) => session.id === id))
      .filter((session): session is TerminalSession => Boolean(session));
  } catch (error) {
    // Ignore sync failures; the UI state still reflects the user's order.
  }
}

function reorderSessions(dragId: string, targetId: string) {
  if (dragId === targetId) return;
  const list = [...sessions.value];
  const fromIndex = list.findIndex((session) => session.id === dragId);
  const toIndex = list.findIndex((session) => session.id === targetId);
  if (fromIndex < 0 || toIndex < 0) return;
  const [moved] = list.splice(fromIndex, 1);
  list.splice(toIndex, 0, moved);
  sessions.value = list;
  void syncOrder();
}

function onDragStart(sessionId: string, event: DragEvent) {
  dragSessionId.value = sessionId;
  event.dataTransfer?.setData("text/plain", sessionId);
}

function onDrop(targetId: string) {
  if (!dragSessionId.value) return;
  reorderSessions(dragSessionId.value, targetId);
  dragSessionId.value = null;
}

function onDragEnd() {
  dragSessionId.value = null;
}

async function captureOutput() {
  if (!activeSessionId.value) return;
  const response = (await invoke("terminal_replay", {
    session_id: activeSessionId.value,
    max_bytes: 8000,
  })) as TerminalReplayResponse;
  capturedOutput.value = textDecoder.decode(base64ToBytes(response.data_base64));
}

function resizeTerminal() {
  if (!term || !fitAddon) return;
  fitAddon.fit();
  if (!activeSessionId.value) return;
  void invoke("terminal_resize", {
    session_id: activeSessionId.value,
    cols: term.cols,
    rows: term.rows,
  });
}

async function loadExistingSessions() {
  try {
    const existing = (await invoke("terminal_list_sessions")) as TerminalSessionInfo[];
    if (existing.length === 0) {
      await createSession();
      return;
    }
    sessions.value = existing.map((info, index) => ({
      ...info,
      title: info.title || nextTitle(`Session ${index + 1}`),
      state: info.is_alive ? "live" : "error",
      hasUnread: false,
    }));
    sessionCounter = sessions.value.length + 1;
    setActiveSession(sessions.value[0].id);
  } catch (error) {
    await createSession();
  }
}

onMounted(async () => {
  if (!terminalRef.value) return;
  const textPrimary = cssVar("--text-primary", "#e6f1ff");
  const accent = cssVar("--accent", "#36f6ff");
  const accentRgb = cssRgbVar("--accent-rgb", "54, 246, 255");
  term = new Terminal({
    cursorBlink: true,
    fontFamily: '"JetBrains Mono", monospace',
    fontSize: 13,
    lineHeight: 1.2,
    theme: {
      background: "rgba(7, 11, 20, 0.95)",
      foreground: textPrimary,
      cursor: accent,
      selection: `rgba(${accentRgb}, 0.25)`,
    },
  });
  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalRef.value);
  fitAddon.fit();

  term.onData((data) => {
    void sendInput(data);
  });

  unlisten = await listen("terminal-output", (event) => {
    const payload = event.payload as TerminalOutputEvent;
    if (payload.session_id !== activeSessionId.value) {
      markSessionUnread(payload.session_id);
      return;
    }
    const bytes = base64ToBytes(payload.data_base64);
    term?.write(textDecoder.decode(bytes));
  });

  await loadExistingSessions();

  resizeObserver = new ResizeObserver(() => resizeTerminal());
  resizeObserver.observe(terminalRef.value);
  window.addEventListener("resize", resizeTerminal);
});

onBeforeUnmount(() => {
  if (resizeObserver && terminalRef.value) {
    resizeObserver.unobserve(terminalRef.value);
  }
  window.removeEventListener("resize", resizeTerminal);
  if (unlisten) {
    unlisten();
  }
  term?.dispose();
});
</script>

<template>
  <div class="terminal-panel">
    <div class="terminal-header">
      <div class="terminal-tabs">
        <div
          v-for="session in sessions"
          :key="session.id"
          class="terminal-tab"
          :class="{
            active: session.id === activeSessionId,
            dragging: session.id === dragSessionId,
          }"
          draggable="true"
          @dragstart="onDragStart(session.id, $event)"
          @dragover.prevent
          @drop="onDrop(session.id)"
          @dragend="onDragEnd"
        >
          <button class="tab-main" type="button" @click="setActiveSession(session.id)">
            <span class="tab-name">{{ session.title }}</span>
            <span v-if="session.hasUnread" class="tab-unread"></span>
          </button>
          <button class="tab-close" type="button" @click="closeSession(session.id)">×</button>
        </div>
        <button class="tab-add" type="button" @click="createNewSession">+</button>
      </div>
      <div class="terminal-actions">
        <span class="tab-status" :data-state="status">{{ status }}</span>
        <button class="btn ghost" type="button" @click="renameActive">Rename</button>
        <button class="btn ghost" type="button" @click="cloneActive">Clone</button>
        <button class="btn ghost" type="button" @click="replayOutput">Replay</button>
        <button class="btn accent" type="button" @click="captureOutput">Capture</button>
      </div>
    </div>
    <div ref="terminalRef" class="terminal-body"></div>
    <div v-if="capturedOutput" class="terminal-capture">
      <div class="capture-head">
        <span>Captured output</span>
        <button class="btn ghost" type="button" @click="capturedOutput = ''">Clear</button>
      </div>
      <pre>{{ capturedOutput }}</pre>
    </div>
  </div>
</template>

<style scoped>
.terminal-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
  padding: 12px;
}

.terminal-header {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.terminal-tabs {
  display: flex;
  gap: 8px;
  align-items: center;
  overflow-x: auto;
  padding-bottom: 2px;
}

.terminal-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border-radius: 0;
  background: rgba(4, 12, 22, 0.7);
  border: 1px solid rgba(var(--line-rgb), 0.4);
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

.terminal-tab.active {
  border-color: rgba(var(--accent-rgb), 0.5);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.2);
}

.terminal-tab.dragging {
  opacity: 0.6;
  border-style: dashed;
}

.tab-main {
  border: none;
  background: transparent;
  color: var(--text-soft);
  font-size: 0.8rem;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  cursor: pointer;
}

.tab-name {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-unread {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--accent);
  box-shadow: 0 0 8px rgba(var(--accent-rgb), 0.6);
}

.tab-close {
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 0.9rem;
  line-height: 1;
  padding: 2px 6px;
}

.tab-close:hover {
  color: var(--accent-3);
}

.tab-add {
  border-radius: 0;
  border: 1px dashed rgba(var(--accent-rgb), 0.4);
  background: rgba(2, 10, 20, 0.4);
  color: var(--accent);
  font-size: 1rem;
  padding: 2px 10px;
  cursor: pointer;
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

.tab-status {
  font-size: 0.7rem;
  padding: 4px 10px;
  border-radius: 0;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  border: 1px solid transparent;
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

.tab-status[data-state="live"] {
  color: var(--status-success);
  border-color: rgba(var(--status-success-rgb), 0.5);
  background: rgba(var(--status-success-rgb), 0.15);
}

.tab-status[data-state="starting"] {
  color: var(--accent);
  border-color: rgba(var(--accent-rgb), 0.5);
  background: rgba(var(--accent-rgb), 0.12);
}

.tab-status[data-state="error"] {
  color: var(--accent-3);
  border-color: rgba(var(--accent-3-rgb), 0.5);
  background: rgba(var(--accent-3-rgb), 0.12);
}

.tab-status[data-state="idle"] {
  color: var(--text-secondary);
  border-color: rgba(var(--text-secondary-rgb), 0.4);
  background: rgba(var(--text-secondary-rgb), 0.12);
}

.terminal-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  align-items: center;
}

.terminal-body {
  flex: 1;
  min-height: 220px;
  border-radius: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  box-shadow: inset 0 0 22px rgba(var(--accent-rgb), 0.1);
}

.terminal-capture {
  padding: 14px 16px;
  border-radius: 0;
  background: rgba(3, 10, 20, 0.85);
  border: 1px solid rgba(var(--accent-rgb), 0.35);
  font-size: 0.85rem;
  color: var(--text-soft);
  max-height: 180px;
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

.capture-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  color: var(--text-secondary);
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
}

.terminal-capture pre {
  margin: 0;
  white-space: pre-wrap;
  font-family: var(--font-body);
}

.btn {
  border-radius: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.5);
  padding: 8px 14px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  background: linear-gradient(135deg, rgba(3, 12, 24, 0.95), rgba(2, 8, 16, 0.85));
  color: var(--text-primary);
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

.btn:hover {
  border-color: rgba(var(--accent-rgb), 0.7);
  color: var(--accent);
}

.btn.accent {
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.95), rgba(var(--status-info-rgb), 0.8));
  color: var(--bg);
  border-color: transparent;
  box-shadow: 0 0 18px rgba(var(--accent-rgb), 0.35);
}

.btn.ghost {
  background: rgba(4, 12, 22, 0.7);
}
@media (max-width: 900px) {
  .terminal-actions {
    flex-wrap: wrap;
  }
}
</style>


