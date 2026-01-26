<script setup lang="ts">
interface ToolCall {
  id: string;
  tool: string;
  detail: string;
  status: string;
  summary?: string | null;
}

interface Props {
  toolCalls: ToolCall[];
}

defineProps<Props>();
</script>

<template>
  <div class="tool-call-list">
    <div class="section-title">Tool calls</div>
    <div class="tool-call" v-for="call in toolCalls" :key="call.id">
      <div>
        <strong>{{ call.tool }}</strong>
        <p>{{ call.detail }}</p>
        <p v-if="call.summary || call.status === 'error'" class="tool-summary">
          {{ call.summary || "Error without details." }}
        </p>
      </div>
      <span class="chip" :data-status="call.status">{{ call.status }}</span>
    </div>
    <p v-if="!toolCalls.length" class="empty-text">No tool calls yet.</p>
  </div>
</template>

<style scoped>
.tool-call-list {
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

.tool-call {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
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

.tool-summary {
  margin: 6px 0 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
  white-space: pre-wrap;
}

.chip {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 4px 8px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.3);
  color: var(--text-soft);
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

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
</style>
