<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";

type ViewId = "console" | "llm" | "tools" | "security";

const pages: Record<ViewId, { title: string; subtitle: string; description: string }> = {
  console: {
    title: "Console",
    subtitle: "Terminal-first operations",
    description: "Plan, execute, and verify with live PTY sessions and tool calls.",
  },
  llm: {
    title: "LLM Settings",
    subtitle: "Models, safety, and tools",
    description: "Configure providers, prompts, and tool access for deterministic runs.",
  },
  tools: {
    title: "Tool Settings",
    subtitle: "Policies and guardrails",
    description: "Define tool scopes, allowlists, and audit settings.",
  },
  security: {
    title: "Security Settings",
    subtitle: "Sandbox and network",
    description: "Control path access, command policy, and network access.",
  },
};

const navItems: Array<{ id: ViewId; label: string; hint: string }> = [
  { id: "console", label: "Console", hint: "Terminal + Agent timeline" },
  { id: "llm", label: "LLM Settings", hint: "Providers, prompts, tool policy" },
  { id: "tools", label: "Tool Settings", hint: "Policies and guardrails" },
  { id: "security", label: "Security Settings", hint: "Sandbox + network" },
];

const route = useRoute();
const router = useRouter();

const currentView = computed<ViewId>(() => {
  if (route.name === "llm") return "llm";
  if (route.name === "tools") return "tools";
  if (route.name === "security") return "security";
  return "console";
});
const pageMeta = computed(() => pages[currentView.value]);

function goTo(view: ViewId) {
  if (currentView.value === view) return;
  void router.push({ name: view });
}
</script>

<template>
  <div class="shell">
    <div class="shell-main">
      <header class="topbar">
        <div class="brand">
          <div class="brand-mark">TH</div>
          <div class="brand-copy">
            <span class="brand-title">TauriHands</span>
            <span class="brand-sub">{{ pageMeta.subtitle }}</span>
          </div>
        </div>

        <nav class="topbar-tabs">
          <button
            v-for="item in navItems"
            :key="item.id"
            type="button"
            class="tab"
            :class="{ active: currentView === item.id }"
            @click="goTo(item.id)"
          >
            <span class="tab-label">{{ item.label }}</span>
            <span class="tab-hint">{{ item.hint }}</span>
          </button>
        </nav>

        <div class="topbar-status">
          <span class="status-label">{{ pageMeta.title }}</span>
          <span class="status-desc">{{ pageMeta.description }}</span>
        </div>
      </header>

      <main class="content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<style scoped>
.shell {
  min-height: 100vh;
  height: 100vh;
  padding: 20px;
  display: flex;
  flex-direction: column;
}

.shell-main {
  display: flex;
  flex-direction: column;
  gap: 14px;
  flex: 1;
  min-height: 0;
}

.topbar {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 18px;
  align-items: center;
  padding: 16px 18px;
  border-radius: 18px;
  background: var(--panel-strong);
  border: 1px solid var(--line);
  box-shadow: var(--shadow);
  backdrop-filter: blur(14px);
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-mark {
  width: 36px;
  height: 36px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-weight: 700;
  letter-spacing: 0.12em;
  font-size: 0.7rem;
  color: #05060a;
  background: linear-gradient(135deg, rgba(45, 246, 255, 0.9), rgba(182, 255, 75, 0.85));
  box-shadow: 0 0 18px rgba(45, 246, 255, 0.35);
}

.brand-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.brand-title {
  font-size: 0.9rem;
  text-transform: uppercase;
  letter-spacing: 0.22em;
  color: #e6f3ff;
}

.brand-sub {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: #8aa0b7;
}

.topbar-tabs {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(0, 1fr);
  align-items: center;
  gap: 8px;
  padding: 6px;
  border-radius: 999px;
  background: var(--panel-glass);
  border: 1px solid var(--line);
}

.tab {
  border-radius: 999px;
  border: 1px solid transparent;
  padding: 8px 14px;
  display: grid;
  gap: 2px;
  background: transparent;
  color: #9bb0c6;
  cursor: pointer;
  transition: all 0.2s ease;
}

.tab-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
}

.tab-hint {
  font-size: 0.65rem;
  color: rgba(138, 160, 183, 0.8);
}

.tab.active {
  border-color: rgba(45, 246, 255, 0.6);
  color: #2df6ff;
  background: rgba(45, 246, 255, 0.12);
}

.tab:hover {
  border-color: rgba(45, 246, 255, 0.4);
  color: #2df6ff;
}

.topbar-status {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  text-align: right;
}

.status-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: #b6ff4b;
}

.status-desc {
  max-width: 260px;
  font-size: 0.75rem;
  color: #8aa0b7;
}

.content {
  flex: 1;
  min-height: 0;
}

@media (max-width: 1200px) {
  .topbar {
    grid-template-columns: 1fr;
    align-items: flex-start;
  }

  .topbar-status {
    align-items: flex-start;
    text-align: left;
  }

}

@media (max-width: 720px) {
  .shell {
    padding: 14px;
  }

  .topbar-tabs {
    grid-auto-flow: row;
  }
}
</style>
