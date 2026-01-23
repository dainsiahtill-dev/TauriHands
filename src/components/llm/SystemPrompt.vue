<script setup lang="ts">
interface Props {
  prompt: string;
}

interface Emits {
  (e: "update:prompt", value: string): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();
</script>

<template>
  <div class="system-prompt">
    <div class="card-head">
      <h3>System Prompt</h3>
      <span class="pill">Tokens tracked</span>
    </div>
    <textarea 
      :value="prompt" 
      @input="$emit('update:prompt', ($event.target as HTMLTextAreaElement).value)"
      rows="6"
    ></textarea>
    <div class="hint">
      Keep system prompt short and precise. Avoid leaking secrets or environment details.
    </div>
  </div>
</template>

<style scoped>
.system-prompt {
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

textarea {
  border-radius: 0;
  border: 1px solid rgba(var(--line-rgb), 0.45);
  padding: 10px 12px;
  background: rgba(3, 12, 24, 0.9);
  color: var(--text-primary);
  font-family: inherit;
  resize: vertical;
  min-height: 140px;
  position: relative;
  z-index: 1;
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

.hint {
  font-size: 0.78rem;
  color: var(--text-secondary);
  position: relative;
  z-index: 1;
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
</style>
