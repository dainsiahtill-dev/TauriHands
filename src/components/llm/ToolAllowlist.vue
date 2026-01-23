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
  font-size: 1.1rem;
  font-family: var(--font-display);
  text-transform: uppercase;
  letter-spacing: 0.18em;
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
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.4);
  background: rgba(3, 12, 24, 0.85);
  font-size: 0.78rem;
  color: var(--text-soft);
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

.toggle-pill input {
  accent-color: var(--accent);
}

.pill {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.14em;
  padding: 6px 10px;
  border-radius: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.4);
  color: var(--accent);
  background: rgba(var(--accent-rgb), 0.12);
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

@media (max-width: 1100px) {
  .tool-grid {
    grid-template-columns: 1fr;
  }
}
</style>
