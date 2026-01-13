<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { agentStore } from "../agents/orchestrator";

const { state } = agentStore;
const events = computed(() => state.events ?? []);
const focusedId = computed(() => state.timelineFocusId ?? "");

const filter = ref("all");
const query = ref("");
const listRef = ref<HTMLUListElement | null>(null);

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
  if (event.type === "JudgeResult") {
    const result = event.payload?.result as { status?: string; reasons?: string[] } | undefined;
    if (!result) return "judge result";
    const reason = result.reasons?.[0] ?? "";
    return `${result.status ?? "unknown"}${reason ? `: ${reason}` : ""}`.slice(0, 120);
  }
  if (event.type === "Error") {
    return String(event.payload?.message ?? "error").slice(0, 120);
  }
  if (event.type === "AgentMessage") {
    return String(event.payload?.content ?? "").slice(0, 120);
  }
  return "";
}

function scrollToFocused() {
  const id = focusedId.value;
  if (!id || !listRef.value) return;
  const target = listRef.value.querySelector(`[data-event-id="${id}"]`);
  if (target && target instanceof HTMLElement) {
    target.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
}

watch(
  () => [focusedId.value, filteredEvents.value.length],
  () => {
    void nextTick().then(scrollToFocused);
  },
);
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
    <ul v-if="filteredEvents.length" ref="listRef" class="event-list">
      <li
        v-for="event in filteredEvents"
        :key="event.id"
        :data-event-id="event.id"
        :class="{ focused: event.id === focusedId }"
      >
        <div class="event-body">
          <span class="event-type">{{ event.type }}</span>
          <template v-if="event.type === 'JudgeResult' && event.payload?.result">
            <div class="judge-result">
              <div class="judge-header">
                <span class="judge-chip" :data-status="event.payload.result.status">
                  {{ event.payload.result.status }}
                </span>
                <span v-if="event.payload.result.reasons?.length" class="judge-reasons">
                  {{ event.payload.result.reasons.join(" | ") }}
                </span>
              </div>
              <details
                v-for="check in event.payload.result.checks ?? []"
                :key="check.id"
                class="judge-check"
              >
                <summary>
                  <span class="judge-check-type">{{ check.type }}</span>
                  <span class="judge-check-status" :data-status="check.status">
                    {{ check.status }}
                  </span>
                  <span v-if="check.reason" class="judge-check-reason">{{ check.reason }}</span>
                </summary>
                <div v-if="check.evidence?.length" class="judge-evidence">
                  <pre v-for="(item, index) in check.evidence" :key="index">{{ item }}</pre>
                </div>
                <p v-else class="judge-empty">No evidence.</p>
              </details>
            </div>
          </template>
          <p v-else-if="summarize(event)" class="event-summary">{{ summarize(event) }}</p>
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

.event-list li.focused {
  border-color: rgba(45, 246, 255, 0.55);
  box-shadow: 0 0 18px rgba(45, 246, 255, 0.15);
}

.event-body {
  display: grid;
  gap: 6px;
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

.judge-result {
  display: grid;
  gap: 6px;
  font-size: 0.78rem;
}

.judge-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.judge-chip {
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(138, 160, 183, 0.3);
  color: #8aa0b7;
  background: rgba(138, 160, 183, 0.12);
}

.judge-chip[data-status="pass"] {
  color: #b6ff4b;
  border-color: rgba(182, 255, 75, 0.4);
  background: rgba(182, 255, 75, 0.12);
}

.judge-chip[data-status="fail"] {
  color: #ffb84d;
  border-color: rgba(255, 184, 77, 0.4);
  background: rgba(255, 184, 77, 0.12);
}

.judge-chip[data-status="pending"] {
  color: #2df6ff;
  border-color: rgba(45, 246, 255, 0.35);
  background: rgba(45, 246, 255, 0.12);
}

.judge-reasons {
  color: #ffb84d;
  font-size: 0.72rem;
  word-break: break-word;
}

.judge-check {
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(5, 8, 14, 0.6);
  overflow: hidden;
}

.judge-check summary {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: #9bb0c6;
  list-style: none;
}

.judge-check summary::-webkit-details-marker {
  display: none;
}

.judge-check-type {
  color: #2df6ff;
}

.judge-check-status {
  padding: 2px 6px;
  border-radius: 999px;
  border: 1px solid var(--line);
  font-size: 0.55rem;
  color: #c7d7ec;
}

.judge-check-status[data-status="pass"] {
  color: #b6ff4b;
  border-color: rgba(182, 255, 75, 0.4);
  background: rgba(182, 255, 75, 0.12);
}

.judge-check-status[data-status="fail"] {
  color: #ffb84d;
  border-color: rgba(255, 184, 77, 0.4);
  background: rgba(255, 184, 77, 0.12);
}

.judge-check-status[data-status="pending"] {
  color: #2df6ff;
  border-color: rgba(45, 246, 255, 0.35);
  background: rgba(45, 246, 255, 0.12);
}

.judge-check-reason {
  text-transform: none;
  letter-spacing: 0;
  color: #8aa0b7;
  font-size: 0.65rem;
}

.judge-evidence {
  display: grid;
  gap: 6px;
  padding: 6px 8px 8px;
  border-top: 1px solid var(--line);
}

.judge-evidence pre {
  margin: 0;
  font-size: 0.68rem;
  color: #c7d7ec;
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  max-height: 140px;
  overflow: auto;
  background: rgba(5, 8, 14, 0.7);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 6px;
}

.judge-empty {
  margin: 0;
  padding: 6px 8px 8px;
  font-size: 0.7rem;
  color: var(--text-tertiary);
}

.empty {
  margin: 0;
  color: var(--text-tertiary);
}
</style>
