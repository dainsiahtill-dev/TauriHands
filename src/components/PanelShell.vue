<script setup lang="ts">
const props = defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<template>
  <section class="panel-shell">
    <header class="panel-header">
      <div>
        <p class="eyebrow">{{ props.subtitle }}</p>
        <h3>{{ props.title }}</h3>
      </div>
      <div class="panel-actions">
        <slot name="actions" />
      </div>
    </header>
    <div class="panel-body">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.panel-shell {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
  border-radius: 16px;
  background: var(--panel);
  border: 1px solid var(--line);
  box-shadow: var(--shadow);
  height: 100%;
  min-height: 0;
}

.panel-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.panel-header h3 {
  margin: 0;
  font-size: 1rem;
}

.panel-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.panel-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
