<script setup lang="ts">
type Tab = {
  id: string;
  label: string;
};

const props = defineProps<{
  tabs: Tab[];
  active: string;
}>();

const emit = defineEmits<{
  (e: "select", id: string): void;
}>();

function selectTab(id: string) {
  emit("select", id);
}
</script>

<template>
  <div class="workbench-tabs">
    <button
      v-for="tab in props.tabs"
      :key="tab.id"
      type="button"
      class="tab"
      :class="{ active: tab.id === props.active }"
      @click="selectTab(tab.id)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>

<style scoped>
.workbench-tabs {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  padding: 10px 12px;
  border-radius: 14px;
  background: var(--panel);
  border: 1px solid var(--line);
  box-shadow: var(--shadow);
}

.tab {
  border: 1px solid transparent;
  padding: 6px 12px;
  border-radius: 999px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.8rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  cursor: pointer;
}

.tab.active {
  border-color: rgba(45, 246, 255, 0.4);
  color: #2df6ff;
  background: rgba(45, 246, 255, 0.1);
}
</style>
