<script setup lang="ts">
interface Props {
  contextPolicy: string;
  memoryMode: string;
  maxTerminalLines: number;
  enableCaching: boolean;
}

interface Emits {
  (e: "update:contextPolicy", value: string): void;
  (e: "update:memoryMode", value: string): void;
  (e: "update:maxTerminalLines", value: number): void;
  (e: "update:enableCaching", value: boolean): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();
</script>

<template>
  <div class="context-strategy">
    <div class="card-head">
      <h3>Context Strategy</h3>
      <span class="pill">Policy {{ contextPolicy }}</span>
    </div>
    <div class="field-grid">
      <label>
        <span>Context policy</span>
        <select :value="contextPolicy" @change="$emit('update:contextPolicy', ($event.target as HTMLSelectElement).value)">
          <option value="adaptive">Adaptive</option>
          <option value="terminal-first">Terminal first</option>
          <option value="code-first">Code first</option>
          <option value="summary-first">Summary first</option>
        </select>
      </label>
      <label>
        <span>Memory</span>
        <select :value="memoryMode" @change="$emit('update:memoryMode', ($event.target as HTMLSelectElement).value)">
          <option value="session">Session</option>
          <option value="workspace">Workspace</option>
          <option value="off">Off</option>
        </select>
      </label>
      <label>
        <span>Terminal cap (lines)</span>
        <input 
          :value="maxTerminalLines" 
          @input="$emit('update:maxTerminalLines', parseInt(($event.target as HTMLInputElement).value) || 800)"
          type="number" 
          min="200" 
          step="100" 
        />
      </label>
      <label class="switch">
        <input 
          :checked="enableCaching" 
          @change="$emit('update:enableCaching', ($event.target as HTMLInputElement).checked)"
          type="checkbox" 
        />
        <span>Enable response caching</span>
      </label>
    </div>
  </div>
</template>

<style scoped>
.context-strategy {
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

.field-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  position: relative;
  z-index: 1;
}

.field-grid label {
  display: grid;
  gap: 6px;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-secondary);
  font-family: var(--font-display);
}

.field-grid label.switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 0.8rem;
  color: var(--text-soft);
}

select,
input {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  padding: 10px 12px;
  background: rgba(3, 12, 24, 0.9);
  color: var(--text-primary);
  font-family: inherit;
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

.switch input {
  accent-color: var(--status-success);
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
  .field-grid {
    grid-template-columns: 1fr;
  }
}
</style>
