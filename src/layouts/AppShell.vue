<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { agentStore } from "../agents/orchestrator";
import CyberScene from "../components/CyberScene.vue";

type ViewId = "console" | "llm" | "tools" | "security";

const router = useRouter();
const route = useRoute();

const { state, initKernelStore, userInput, stop } = agentStore;
const run = computed(() => state.run);
const agentState = computed(() => run.value?.agentState ?? "IDLE");
const needsAttention = computed(() => agentState.value === "AWAITING_USER");
const attentionError = ref("");
const runId = computed(() => run.value?.runId ?? "-");
const shortRunId = computed(() => (runId.value ? runId.value.slice(0, 8) : "-"));
const workspacePath = computed(() => run.value?.toolContext?.cwd ?? "Not set");
const sessionId = computed(() => run.value?.toolContext?.sessionId ?? "none");
const workspaceName = computed(() => {
  if (!workspacePath.value || workspacePath.value === "Not set") return "Not set";
  const parts = workspacePath.value.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? workspacePath.value;
});
const budget = computed(() => run.value?.budget);
const budgetLabel = computed(() =>
  budget.value ? `${budget.value.usedSteps}/${budget.value.maxSteps}` : "-",
);
const budgetPercent = computed(() => {
  const current = budget.value;
  if (!current || current.maxSteps === 0) return 0;
  return Math.min(100, Math.round((current.usedSteps / current.maxSteps) * 100));
});
const toolCalls = computed(() => state.toolCalls ?? []);
const activeTool = computed(() => toolCalls.value.find((call) => call.status === "running"));
const toolStats = computed(() => {
  const stats = { total: 0, running: 0, ok: 0, error: 0 };
  for (const call of toolCalls.value) {
    stats.total += 1;
    if (call.status === "running") stats.running += 1;
    if (call.status === "ok") stats.ok += 1;
    if (call.status === "error") stats.error += 1;
  }
  return stats;
});
const successPercent = computed(() => {
  if (!toolStats.value.total) return 0;
  return Math.round((toolStats.value.ok / toolStats.value.total) * 100);
});
const isStreaming = computed(() => state.llmStream.active);
const streamPreview = computed(() => state.llmStream.content.slice(0, 120));
const powerPercent = computed(() => Math.max(0, 100 - budgetPercent.value));
const strategyLabel = computed(() => (run.value?.autoRun ? "Auto-pilot" : "Guided"));

const agentStateClass = computed(() => {
  switch (agentState.value) {
    case "RUNNING":
      return "text-accent";
    case "AWAITING_USER":
      return "text-accent-lime";
    case "ERROR":
      return "text-status-warning";
    case "PAUSED":
      return "text-status-info";
    default:
      return "text-text-muted";
  }
});

const currentView = computed<ViewId>(() => {
  if (route.name === "llm") return "llm";
  if (route.name === "tools") return "tools";
  if (route.name === "security") return "security";
  return "console";
});

const showSecondarySidebar = computed(() => currentView.value === "console");

const IconCore = {
  template:
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><circle cx="12" cy="12" r="8"></circle><path d="M12 4v2M12 18v2M4 12h2M18 12h2"></path></svg>',
};
const IconConsole = {
  template:
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"></rect><path d="m7 9 3 3-3 3"></path><path d="M13 15h4"></path></svg>',
};
const IconBrain = {
  template:
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 4a3 3 0 0 0-3 3v1a3 3 0 0 0-2 2.8 3 3 0 0 0 1 2.2 3 3 0 0 0 1 5.8h4"></path><path d="M15 4a3 3 0 0 1 3 3v1a3 3 0 0 1 2 2.8 3 3 0 0 1-1 2.2 3 3 0 0 1-1 5.8h-4"></path><path d="M9 7h6M9 12h6M9 17h6"></path></svg>',
};
const IconTools = {
  template:
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14.7 6.3a4 4 0 0 0-5.4 5.4l-5.3 5.3a2 2 0 0 0 2.8 2.8l5.3-5.3a4 4 0 0 0 5.4-5.4l-2 2-3-3 2-2Z"></path></svg>',
};
const IconShield = {
  template:
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3 20 7v5c0 5-3.5 8.5-8 9-4.5-.5-8-4-8-9V7l8-4Z"></path></svg>',
};
const IconSearch = {
  template:
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.5-3.5"></path></svg>',
};

const activities: Array<{
  id: ViewId;
  label: string;
  hint: string;
  icon: typeof IconConsole;
}> = [
  { id: "console", label: "Console", hint: "Cockpit", icon: IconConsole },
  { id: "llm", label: "LLM", hint: "Models", icon: IconBrain },
  { id: "tools", label: "Tools", hint: "Policies", icon: IconTools },
  { id: "security", label: "Security", hint: "Sandbox", icon: IconShield },
];

function navigateTo(view: ViewId) {
  if (currentView.value === view) return;
  void router.push({ name: view });
}

async function handleContinue() {
  attentionError.value = "";
  try {
    await userInput("继续");
  } catch (error) {
    attentionError.value = error instanceof Error ? error.message : "Unable to continue.";
  }
}

async function handleStop() {
  attentionError.value = "";
  try {
    await stop();
  } catch (error) {
    attentionError.value = error instanceof Error ? error.message : "Unable to stop.";
  }
}

onMounted(() => {
  void initKernelStore();
});
</script>

<template>
  <div class="app-shell">
    <CyberScene />
    <div class="scanline" aria-hidden="true"></div>
    <div class="app-backdrop"></div>
    <div class="app-frame">
      <div class="frame-rim" aria-hidden="true"></div>
      <div class="frame-lights" aria-hidden="true"></div>
      <header class="app-header">
        <div class="app-header__left">
          <div class="brand-badge">
            <component :is="IconCore" class="h-5 w-5" />
          </div>
          <div class="header-block">
            <p class="brand-title">System Core</p>
            <h1 class="brand-subtitle">HandsFlow - AI Development Agent</h1>
            <div class="header-chip">
              Workspace <strong>{{ workspaceName }}</strong>
            </div>
          </div>
        </div>

        <div class="app-header__center">
          <div class="header-block center">
            <div class="header-title">System Core Online</div>
            <div class="header-search">
              <component :is="IconSearch" class="h-3 w-3 text-accent" />
              Search / Command
            </div>
            <div class="core-status">
              <span class="core-pulse"></span>
              Core Online
            </div>
          </div>
        </div>

        <div class="app-header__right">
          <div class="meta-card">
            <span>Model</span>
            <strong>GPT-4</strong>
            <div class="meta-pill">Power {{ powerPercent }}%</div>
          </div>
          <div class="meta-card">
            <span>Strategy</span>
            <strong>{{ strategyLabel }}</strong>
            <div class="meta-pill">Active</div>
          </div>
        </div>
      </header>

      <div class="app-body" :class="{ 'with-rail': showSecondarySidebar }">
        <aside class="primary-nav">
          <button
            v-for="item in activities"
            :key="item.id"
            type="button"
            class="nav-button"
            :class="{ 'is-active': currentView === item.id }"
            @click="navigateTo(item.id)"
          >
            <component :is="item.icon" />
            <span>{{ item.label }}</span>
          </button>
        </aside>

        <aside v-if="showSecondarySidebar" class="system-rail">
          <div class="system-rail__header">System</div>
          <div class="system-card">
            <div class="header-chip">
              Core <strong :class="agentStateClass">{{ agentState }}</strong>
            </div>
            <div class="text-xs text-text-dim">{{ workspacePath }}</div>
            <div class="system-row">
              <span>Run</span>
              <strong>{{ shortRunId }}</strong>
            </div>
            <div class="system-row">
              <span>Session</span>
              <strong>{{ sessionId }}</strong>
            </div>
            <div class="system-row">
              <span>Budget</span>
              <strong>{{ budgetLabel }}</strong>
            </div>
            <div class="h-1.5 rounded-full bg-bg-active">
              <div
                class="h-full rounded-full bg-accent shadow-glow shadow-accent/60"
                :style="{ width: `${budgetPercent}%` }"
              ></div>
            </div>
          </div>

          <div class="system-card">
            <span class="text-[11px] uppercase tracking-[0.28em] text-text-dim">Live Signal</span>
            <div v-if="activeTool">
              <strong class="text-accent">{{ activeTool.tool }}</strong>
              <div class="text-text-muted">{{ activeTool.detail }}</div>
            </div>
            <div v-else-if="isStreaming">
              <strong class="text-accent">LLM streaming</strong>
              <div class="text-text-muted">{{ streamPreview || "..." }}</div>
            </div>
            <div v-else class="text-text-dim">Awaiting command.</div>
          </div>

          <div class="system-card">
            <span class="text-[11px] uppercase tracking-[0.28em] text-text-dim">Tool Activity</span>
            <div class="system-list">
              <div class="system-row">
                <span>Total</span>
                <strong>{{ toolStats.total }}</strong>
              </div>
              <div class="system-row">
                <span>Running</span>
                <strong>{{ toolStats.running }}</strong>
              </div>
              <div class="system-row">
                <span>OK</span>
                <strong>{{ toolStats.ok }}</strong>
              </div>
              <div class="system-row">
                <span>Error</span>
                <strong>{{ toolStats.error }}</strong>
              </div>
            </div>
            <div class="h-1.5 rounded-full bg-bg-active">
              <div
                class="h-full rounded-full bg-accent-lime shadow-glow shadow-accent/60"
                :style="{ width: `${successPercent}%` }"
              ></div>
            </div>
          </div>
        </aside>

        <main class="app-main">
          <section class="app-main__viewport">
            <router-view v-slot="{ Component }">
              <transition name="fade" mode="out-in">
                <component :is="Component" class="animate-rise" />
              </transition>
            </router-view>
          </section>
        </main>
      </div>

      <footer class="app-footer">
        <div class="header-chip">
          Status <strong :class="agentStateClass">{{ agentState }}</strong>
        </div>
        <div class="header-chip">
          Run <strong>{{ shortRunId }}</strong>
        </div>
        <div class="header-chip">
          Workspace <strong>{{ workspaceName }}</strong>
        </div>
        <div>Tauri v2.0.0</div>
      </footer>
    </div>

    <div v-if="needsAttention" class="attention-overlay" role="dialog" aria-live="assertive">
      <div class="attention-card">
        <p class="attention-title">Awaiting confirmation</p>
        <p class="attention-text">The agent paused and needs your input to continue.</p>
        <div class="attention-actions">
          <button class="btn primary attention-btn" type="button" @click="handleContinue">Continue</button>
          <button class="btn ghost attention-btn" type="button" @click="handleStop">Stop</button>
        </div>
        <p v-if="attentionError" class="attention-error">{{ attentionError }}</p>
      </div>
    </div>
  </div>
</template>

