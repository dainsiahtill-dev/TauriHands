<script setup lang="ts">
interface Log {
  id: string;
  level: string;
  message: string;
  timestamp: number;
}

interface Props {
  logs: Log[];
  latestErrorId: string;
}

defineProps<Props>();

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString();
}
</script>

<template>
  <div class="log-viewer">
    <div class="section-title">Logs</div>
    <div class="logs">
      <div
        v-for="log in logs"
        :key="log.id"
        class="log-row"
        :data-level="log.level"
        :data-latest="log.id === latestErrorId"
      >
        <span class="log-time">{{ formatTime(log.timestamp) }}</span>
        <span class="log-level">{{ log.level }}</span>
        <span class="log-message">{{ log.message }}</span>
      </div>
      <p v-if="!logs.length" class="empty-text">No logs yet.</p>
    </div>
  </div>
</template>

<style scoped>
.log-viewer {
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

.logs {
  display: grid;
  gap: 8px;
}

.log-row {
  display: grid;
  grid-template-columns: auto auto 1fr;
  gap: 8px;
  align-items: center;
  padding: 6px 10px;
  border-radius: 10px;
  background: rgba(8, 12, 20, 0.8);
  border: 1px solid var(--line);
  font-size: 0.75rem;
}

.log-row[data-latest="true"] {
  border-color: rgba(var(--status-warning-rgb), 0.6);
  box-shadow: 0 0 16px rgba(var(--status-warning-rgb), 0.15);
}

.log-time {
  color: var(--text-secondary);
}

.log-level {
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 0.6rem;
}

.log-message {
  color: var(--text-primary);
}

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
</style>
