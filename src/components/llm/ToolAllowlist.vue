<script setup lang="ts">

interface ToolToggle {
  id: string;
  label: string;
  enabled: boolean;
}

interface Props {
  toolToggles: ToolToggle[];
}

interface Emits {
  (e: "update:toolToggles", value: ToolToggle[]): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

function updateTool(toolId: string, enabled: boolean) {
  const updatedToggles = props.toolToggles.map(tool => 
    tool.id === toolId ? { ...tool, enabled } : tool
  );
  emit('update:toolToggles', updatedToggles);
}
</script>

<template>
  <div class="tool-allowlist">
    <div class="card-head">
      <h3>Tool Allowlist</h3>
      <span class="pill">Least privilege</span>
    </div>
    <div class="tool-grid">
      <label v-for="tool in toolToggles" :key="tool.id" class="toggle-pill">
        <input 
          :checked="tool.enabled" 
          @change="updateTool(tool.id, ($event.target as HTMLInputElement).checked)"
          type="checkbox" 
        />
        <span>{{ tool.label }}</span>
      </label>
    </div>
  </div>
</template>

<style scoped>
.tool-allowlist {
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: relative;
  z-index: 1;
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  position: relative;
  z-index: 1;
}

.card-head h3 {
  margin: 0;
  font-size: 1rem;
  font-family: var(--font-display);
  letter-spacing: 0.02em;
}

.tool-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  position: relative;
  z-index: 1;
}

.toggle-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  font-size: 0.78rem;
  color: var(--text-primary);
}

.toggle-pill input {
  accent-color: var(--accent);
}

.pill {
  font-size: 0.65rem;
  letter-spacing: 0.03em;
  padding: 6px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--accent-rgb), 0.35);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
}

@media (max-width: 1100px) {
  .tool-grid {
    grid-template-columns: 1fr;
  }
}
</style>
