<script setup lang="ts">
import { computed } from "vue";

interface TestLog {
  id: string;
  level: "info" | "success" | "warn" | "error";
  message: string;
  timestamp: number;
  detail?: string;
}

interface Props {
  testLogs: TestLog[];
  testStatus: "idle" | "running" | "ok" | "error";
  isTesting: boolean;
}

interface Emits {
  (e: "testConnection"): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const testStatusLabel = computed(() => {
  switch (props.testStatus) {
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

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}
</script>

<template>
  <div class="connection-test">
    <div class="card-head">
      <h3>Connection test</h3>
      <span class="pill" :data-status="testStatus">{{ testStatusLabel }}</span>
    </div>
    <div class="test-actions">
      <button class="btn ghost" type="button" :disabled="isTesting" @click="$emit('testConnection')">
        {{ isTesting ? "Testing..." : "Test connection" }}
      </button>
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
  </div>
</template>

<style scoped>
.connection-test {
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: relative;
  z-index: 1;
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
  font-family: var(--font-display);
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.test-actions {
  display: flex;
  gap: 8px;
  position: relative;
  z-index: 1;
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
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: rgba(4, 12, 22, 0.8);
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
  border-radius: 0;
  background: rgba(5, 8, 14, 0.75);
  border: 1px solid rgba(var(--line-rgb), 0.4);
  font-size: 0.7rem;
  color: var(--text-soft);
  font-family: var(--font-body);
  white-space: pre-wrap;
  max-height: 220px;
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
</style>
