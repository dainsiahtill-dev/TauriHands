<script setup lang="ts">
interface Props {
  auditLogs: boolean;
  redactSecrets: boolean;
}

interface Emits {
  (e: "update:auditLogs", value: boolean): void;
  (e: "update:redactSecrets", value: boolean): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();
</script>

<template>
  <div class="audit-settings">
    <div class="card-head">
      <h3>Audit & Redaction</h3>
      <span class="pill">Compliance</span>
    </div>
    <div class="toggle-row">
      <label class="switch">
        <input 
          :checked="auditLogs" 
          @change="$emit('update:auditLogs', ($event.target as HTMLInputElement).checked)"
          type="checkbox" 
        />
        <span>Append-only audit log</span>
      </label>
      <label class="switch">
        <input 
          :checked="redactSecrets" 
          @change="$emit('update:redactSecrets', ($event.target as HTMLInputElement).checked)"
          type="checkbox" 
        />
        <span>Redact secrets in logs</span>
      </label>
    </div>
    <div class="hint">
      Logs are stored locally and can be exported for review. Redaction masks API keys.
    </div>
  </div>
</template>

<style scoped>
.audit-settings {
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

.toggle-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px 16px;
  position: relative;
  z-index: 1;
}

.switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 0.8rem;
  color: var(--text-soft);
}

.switch input {
  accent-color: var(--status-success);
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
