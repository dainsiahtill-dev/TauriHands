<script setup lang="ts">
type EvidenceItem = {
  label: string;
  value: string;
};

const props = defineProps<{
  items: EvidenceItem[];
  emptyText?: string;
}>();
</script>

<template>
  <ul class="evidence-list">
    <li v-for="item in props.items" :key="item.label">
      <span class="label">{{ item.label }}</span>
      <span class="value">{{ item.value }}</span>
    </li>
    <li v-if="!props.items.length" class="empty">
      {{ props.emptyText ?? "No evidence yet." }}
    </li>
  </ul>
</template>

<style scoped>
.evidence-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.evidence-list li {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: rgba(8, 12, 20, 0.7);
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.evidence-list .label {
  color: var(--text-primary);
}

.evidence-list .empty {
  justify-content: center;
  color: var(--text-tertiary);
  border-style: dashed;
}
</style>
