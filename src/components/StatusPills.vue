<script setup lang="ts">
import { computed, ref } from "vue";

type StatusItem = {
  label: string;
  value: string;
  tone?: "info" | "ok" | "warn" | "error";
  detail?: string;
};

const props = defineProps<{
  items: StatusItem[];
}>();

const activeLabel = ref("");

const activeDetail = computed(() =>
  props.items.find((item) => item.label === activeLabel.value && item.detail),
);

function toggleDetail(item: StatusItem) {
  if (!item.detail) return;
  activeLabel.value = activeLabel.value === item.label ? "" : item.label;
}
</script>

<template>
  <div class="status-pills">
    <template v-for="item in props.items" :key="item.label">
      <button
        v-if="item.detail"
        type="button"
        class="pill pill-button"
        :data-tone="item.tone ?? 'info'"
        :data-active="activeLabel === item.label"
        :aria-expanded="activeLabel === item.label"
        @click="toggleDetail(item)"
      >
        <strong>{{ item.label }}</strong>
        <span>{{ item.value }}</span>
      </button>
      <span v-else class="pill" :data-tone="item.tone ?? 'info'">
        <strong>{{ item.label }}</strong>
        <span>{{ item.value }}</span>
      </span>
    </template>
  </div>
  <div v-if="activeDetail" class="pill-detail">
    <div class="pill-detail__label">{{ activeDetail.label }} details</div>
    <pre class="pill-detail__content">{{ activeDetail.detail }}</pre>
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
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.72rem;
}

.pill strong {
  font-weight: 600;
  color: var(--text-primary);
  font-family: var(--font-display);
}

.pill-button {
  cursor: pointer;
  transition: border-color 0.2s ease, box-shadow 0.2s ease, color 0.2s ease;
}

.pill-button:hover {
  border-color: rgba(var(--accent-rgb), 0.45);
  color: var(--text-primary);
  box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.18);
}

.pill-button[data-active="true"] {
  border-color: rgba(var(--status-warning-rgb), 0.6);
  box-shadow: 0 0 14px rgba(var(--status-warning-rgb), 0.2);
}

.pill-detail {
  margin-top: 8px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--status-warning-rgb), 0.45);
  background: rgba(var(--status-warning-rgb), 0.1);
  display: grid;
  gap: 6px;
}

.pill-detail__label {
  font-size: 0.65rem;
  letter-spacing: 0.06em;
  color: var(--status-warning);
  font-family: var(--font-display);
}

.pill-detail__content {
  margin: 0;
  max-height: 180px;
  overflow: auto;
  font-size: 0.75rem;
  white-space: pre-wrap;
  color: var(--text-soft);
  font-family: var(--font-body);
}

.pill[data-tone="ok"] {
  border-color: rgba(var(--status-success-rgb), 0.35);
  color: var(--status-success);
}

.pill[data-tone="warn"] {
  border-color: rgba(var(--status-warning-rgb), 0.4);
  color: var(--status-warning);
}

.pill[data-tone="error"] {
  border-color: rgba(255, 99, 99, 0.4);
  color: var(--status-error);
}
</style>


