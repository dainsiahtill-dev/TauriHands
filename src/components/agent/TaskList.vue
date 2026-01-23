<script setup lang="ts">
interface Task {
  id: string;
  title: string;
  status: string;
}

interface Props {
  tasks: Task[];
}

defineProps<Props>();
</script>

<template>
  <div class="task-list">
    <div class="section-title">Tasks</div>
    <div v-if="tasks.length" class="plan-list">
      <div v-for="task in tasks" :key="task.id" class="plan-item">
        <span class="plan-text">{{ task.title }}</span>
        <span class="plan-status" :data-status="task.status">{{ task.status }}</span>
      </div>
    </div>
    <p v-else class="empty-text">No tasks generated yet.</p>
  </div>
</template>

<style scoped>
.task-list {
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

.plan-list {
  display: grid;
  gap: 10px;
}

.plan-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 0;
  background: rgba(9, 14, 22, 0.85);
  border: 1px solid rgba(var(--line-rgb), 0.35);
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

.plan-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plan-status {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  padding: 4px 8px;
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.35);
  color: var(--text-secondary);
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

.empty-text {
  margin: 0;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
</style>
