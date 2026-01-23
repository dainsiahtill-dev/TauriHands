<script setup lang="ts">
import LLMSettingsPanel from "../components/LLMSettingsPanel.vue";
</script>

<template>
  <section class="settings-page">
    <LLMSettingsPanel />
  </section>
</template>

<style scoped>
.settings-page {
  height: 100%;
  min-height: 0;
  padding: 12px;
  border-radius: 0;
  background: var(--panel-core-strong);
  border: 1px solid rgba(var(--line-rgb), 0.4);
  box-shadow: 0 0 24px rgba(5, 8, 14, 0.8);
  overflow: hidden;
  clip-path: polygon(
    var(--hud-cut-sm) 0,
    calc(100% - var(--hud-cut-sm)) 0,
    100% var(--hud-cut-sm),
    100% calc(100% - var(--hud-cut-sm)),
    calc(100% - var(--hud-cut-sm)) 100%,
    var(--hud-cut-sm) 100%,
    0 calc(100% - var(--hud-cut-sm)),
    0 var(--hud-cut-sm)
  );
}
</style>
