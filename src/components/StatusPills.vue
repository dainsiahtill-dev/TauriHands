<script setup lang="ts">
type StatusItem = {
  label: string;
  value: string;
  tone?: "info" | "ok" | "warn" | "error";
};

const props = defineProps<{
  items: StatusItem[];
}>();
</script>

<template>
  <div class="status-pills">
    <span
      v-for="item in props.items"
      :key="item.label"
      class="pill"
      :data-tone="item.tone ?? 'info'"
    >
      <strong>{{ item.label }}</strong>
      <span>{{ item.value }}</span>
    </span>
  </div>
</template>

<style scoped>
.status-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.pill {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: rgba(10, 16, 26, 0.7);
  color: var(--text-secondary);
  font-size: 0.72rem;
}

.pill strong {
  font-weight: 600;
  color: var(--text-primary);
}

.pill[data-tone="ok"] {
  border-color: rgba(182, 255, 75, 0.35);
  color: #b6ff4b;
}

.pill[data-tone="warn"] {
  border-color: rgba(255, 184, 77, 0.4);
  color: #ffb84d;
}

.pill[data-tone="error"] {
  border-color: rgba(255, 99, 99, 0.4);
  color: #ff6363;
}
</style>
