<script setup lang="ts">
import PanelShell from "../components/PanelShell.vue";
</script>

<template>
  <div class="settings-page">
    <PanelShell title="Tool Settings" subtitle="Policies">
      <p class="empty">Tool policies are not configured yet.</p>
    </PanelShell>
  </div>
</template>

<style scoped>
.settings-page {
  height: 100%;
  padding: 16px;
}

.empty {
  color: var(--text-tertiary);
  margin: 0;
}
</style>
