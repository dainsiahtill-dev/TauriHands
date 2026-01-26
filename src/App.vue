<script setup lang="ts">
import AppShell from "./layouts/AppShell-Element.vue";
import { useTheme } from "./composables/useTheme";

// Theme is automatically initialized when useTheme is called
useTheme();
</script>

<template>
  <AppShell />
</template>
