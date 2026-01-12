<script setup lang="ts">
import { computed, ref } from "vue";
import { agentStore } from "../agents/orchestrator";

const { state } = agentStore;
const events = computed(() => state.events ?? []);

const filter = ref("all");
const query = ref("");

const filteredEvents = computed(() => {
  const q = query.value.trim().toLowerCase();
  return events.value.filter((event) => {
    if (filter.value !== "all" && event.type !== filter.value) {
      return false;
    }
    if (!q) return true;
    const payload = JSON.stringify(event.payload ?? {}).toLowerCase();
    return event.type.toLowerCase().includes(q) || payload.includes(q);
  });
});

const eventTypes = computed(() => {
  const types = new Set<string>();
  events.value.forEach((event) => types.add(event.type));
  return ["all", ...Array.from(types).sort()];
});

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString();
}

function summarize(event: { type: string; payload: Record<string, unknown> }) {
  if (event.type === "ToolCallFinished") {
    const summary = event.payload?.summary as string | undefined;
    return summary ? summary.slice(0, 120) : "tool finished";
  }
  if (event.type === "Error") {
    return String(event.payload?.message ?? "error").slice(0, 120);
  }
  if (event.type === "AgentMessage") {
    return String(event.payload?.content ?? "").slice(0, 120);
  }
  return "";
}
</script>

<template>
  <div class="timeline-panel">
    <div class="header">
      <p class="eyebrow">Timeline</p>
      <div class="filters">
        <select v-model="filter" class="field">
          <option v-for="type in eventTypes" :key="type" :value="type">{{ type }}</option>
        </select>
        <input v-model="query" class="field" type="text" placeholder="Search events" />
      </div>
    </div>
    <ul v-if="filteredEvents.length" class="event-list">
      <li v-for="event in filteredEvents" :key="event.id">
        <div>
          <span class="event-type">{{ event.type }}</span>
          <p v-if="summarize(event)" class="event-summary">{{ summarize(event) }}</p>
        </div>
        <span class="event-time">{{ formatTime(event.ts) }}</span>
      </li>
    </ul>
    <p v-else class="empty">No events yet.</p>
  </div>
</template>

<style scoped>
.timeline-panel {
  display: grid;
  gap: 8px;
  color: var(--text-secondary);
}

.header {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.filters {
  display: grid;
  gap: 8px;
  grid-template-columns: 140px minmax(0, 1fr);
}

.field {
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
  color: var(--text-primary);
  font-size: 0.8rem;
}

.event-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 8px;
}

.event-list li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
  font-size: 0.85rem;
}

.event-type {
  color: var(--text-primary);
}

.event-summary {
  margin: 4px 0 0;
  color: var(--text-tertiary);
  font-size: 0.75rem;
}

.event-time {
  color: var(--text-tertiary);
}

.empty {
  margin: 0;
  color: var(--text-tertiary);
}
</style>
