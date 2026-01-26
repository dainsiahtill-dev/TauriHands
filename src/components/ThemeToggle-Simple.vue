<script setup lang="ts">
import { useTheme } from '../composables/useTheme'

const { isDark, toggleTheme } = useTheme()
</script>

<template>
  <button 
    class="btn btn--ghost btn--sm"
    @click="toggleTheme"
    :title="isDark ? 'Switch to light theme' : 'Switch to dark theme'"
  >
    <svg v-if="isDark" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-9M12 3v18m9-9h-9" />
      <circle cx="12" cy="12" r="3" />
    </svg>
    <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 0L12 17.646l-6.646-6.646A9 9 0 012.646 3.646L12 6.354l6.646 6.646A9 9 0 0115.354 20.354L12 17.646l6.646-6.646z" />
    </svg>
  </button>
</template>
