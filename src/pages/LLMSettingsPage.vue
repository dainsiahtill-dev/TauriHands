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
  border-radius: 16px;
  background: var(--surface, rgba(20, 24, 30, 0.8));
  border: 1px solid rgba(var(--line-rgb), 0.2);
  box-shadow: 0 16px 30px rgba(8, 12, 18, 0.2);
  overflow: hidden;
}
</style>
