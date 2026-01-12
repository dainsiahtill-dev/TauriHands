<script setup lang="ts">
import AppShell from "./layouts/AppShell.vue";
</script>

<template>
  <AppShell />
</template>

<style>
@import url("https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&family=Space+Grotesk:wght@400;500;600;700&display=swap");

:root {
  color-scheme: dark;
  font-family: "Space Grotesk", sans-serif;
  font-size: 16px;
  line-height: 1.5;
  background-color: #05060a;
  color: #e6f3ff;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  --bg: #05060a;
  --bg-elevated: rgba(7, 11, 18, 0.96);
  --panel: rgba(10, 15, 26, 0.92);
  --panel-strong: rgba(12, 18, 32, 0.96);
  --panel-glass: rgba(8, 12, 20, 0.7);
  --line: rgba(255, 255, 255, 0.08);
  --muted: #8aa0b7;
  --accent: #2df6ff;
  --accent-2: #b6ff4b;
  --accent-3: #ff4fa2;
  --glow: rgba(45, 246, 255, 0.25);
  --shadow: 0 24px 60px rgba(2, 4, 8, 0.55);
  --text-primary: #e6f3ff;
  --text-secondary: #9bb0c6;
  --text-tertiary: #6f8298;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-height: 100vh;
  background:
    radial-gradient(circle at 12% 12%, rgba(45, 246, 255, 0.2), transparent 45%),
    radial-gradient(circle at 88% 8%, rgba(255, 79, 162, 0.16), transparent 40%),
    radial-gradient(circle at 30% 92%, rgba(182, 255, 75, 0.12), transparent 48%),
    linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.03) 1px, transparent 1px),
    #05060a;
  background-size: auto, auto, auto, 48px 48px, 48px 48px, auto;
  background-attachment: fixed;
  color: inherit;
}

#app {
  min-height: 100vh;
}

.eyebrow {
  margin: 0;
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: var(--text-tertiary);
}

.btn {
  border-radius: 10px;
  border: 1px solid var(--line);
  padding: 8px 10px;
  font-size: 0.75rem;
  background: var(--panel-glass);
  color: var(--text-secondary);
  cursor: pointer;
}

.btn.primary {
  background: linear-gradient(135deg, rgba(45, 246, 255, 0.9), rgba(74, 125, 255, 0.9));
  color: #05060a;
  border-color: transparent;
  box-shadow: 0 0 18px rgba(45, 246, 255, 0.4);
}

.btn.ghost:hover {
  border-color: rgba(45, 246, 255, 0.5);
  color: var(--accent);
}

.btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
