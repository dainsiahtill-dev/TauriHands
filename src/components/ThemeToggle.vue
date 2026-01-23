<script setup lang="ts">
import { useTheme } from '../composables/useTheme';

const { currentTheme, currentThemeId, isDark, availableThemes, setTheme, toggleTheme } = useTheme();

const handleThemeChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  setTheme(target.value);
};
</script>

<template>
  <div class="theme-toggle">
    <button 
      class="theme-toggle-btn"
      @click="toggleTheme"
      :title="isDark ? 'Switch to light theme' : 'Switch to dark theme'"
    >
      <svg v-if="isDark" class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-9M12 3v18m9-9h-9" />
        <circle cx="12" cy="12" r="3" />
      </svg>
      <svg v-else class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 0L12 17.646l-6.646-6.646A9 9 0 012.646 3.646L12 6.354l6.646 6.646A9 9 0 0115.354 20.354L12 17.646l6.646-6.646z" />
      </svg>
    </button>
    
    <select 
      class="theme-select"
      :value="currentThemeId"
      @change="handleThemeChange"
      title="Select theme"
    >
      <option 
        v-for="theme in availableThemes" 
        :key="theme.id" 
        :value="theme.id"
      >
        {{ theme.name }}
      </option>
    </select>
  </div>
</template>

<style scoped>
.theme-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
}

.theme-toggle-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: var(--border-radius-md);
  background: var(--surface);
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.2s ease;
}

.theme-toggle-btn:hover {
  background: var(--primary);
  border-color: var(--primary);
  color: var(--bg);
}

.theme-toggle-btn:active {
  transform: scale(0.95);
}

.theme-icon {
  width: 16px;
  height: 16px;
  transition: transform 0.2s ease;
}

.theme-toggle-btn:hover .theme-icon {
  transform: rotate(20deg);
}

.theme-select {
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--border-radius-md);
  background: var(--surface);
  color: var(--text-primary);
  font-family: var(--font-secondary);
  font-size: 0.75rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.theme-select:hover {
  border-color: var(--primary);
}

.theme-select:focus {
  outline: none;
  border-color: var(--primary);
  box-shadow: 0 0 0 2px var(--primary);
}

/* Dark theme specific styles */
[data-theme-mode="dark"] .theme-toggle-btn {
  box-shadow: 0 0 8px rgba(var(--primary-rgb), 0.2);
}

/* Light theme specific styles */
[data-theme-mode="light"] .theme-toggle-btn {
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .theme-toggle {
    gap: 6px;
  }

  .theme-toggle-btn {
    width: 28px;
    height: 28px;
  }

  .theme-icon {
    width: 14px;
    height: 14px;
  }

  .theme-select {
    font-size: 0.7rem;
    padding: 4px 6px;
  }
}

@media (max-width: 480px) {
  .theme-select {
    display: none;
  }
}
</style>
