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
  border-radius: 22px;
  background: var(--panel-strong);
  border: 1px solid var(--line);
  box-shadow: 0 0 24px rgba(5, 8, 14, 0.8);
  overflow: hidden;
}
</style>
