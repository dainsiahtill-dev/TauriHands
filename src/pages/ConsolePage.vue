<script setup lang="ts">
import { computed, ref } from "vue";
import WorkbenchTabs from "../components/WorkbenchTabs.vue";
import PanelShell from "../components/PanelShell.vue";
import MissionPanel from "../components/MissionPanel.vue";
import PlanPanel from "../components/PlanPanel.vue";
import LoopPanel from "../components/LoopPanel.vue";
import TerminalPanel from "../components/TerminalPanel.vue";
import DiffPanel from "../components/DiffPanel.vue";
import GitPanel from "../components/GitPanel.vue";
import TimelinePanel from "../components/TimelinePanel.vue";
import { agentStore } from "../agents/orchestrator";

const tabs = [
  { id: "mission", label: "Mission" },
  { id: "plan", label: "Plan" },
  { id: "loop", label: "Loop" },
  { id: "terminal", label: "Terminal" },
  { id: "diff", label: "Diff" },
  { id: "git", label: "Git" },
  { id: "timeline", label: "Timeline" },
];

const focusPanel = ref("loop");
const rightTab = ref("timeline");

const { state } = agentStore;
const run = computed(() => state.run);
const agentState = computed(() => run.value?.agentState ?? "IDLE");
const runId = computed(() => run.value?.runId ?? "-");
const shortRunId = computed(() => (runId.value ? runId.value.slice(0, 8) : "-"));
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

const rightPanelMap = {
  diff: DiffPanel,
  git: GitPanel,
  timeline: TimelinePanel,
};

const rightPanelComponent = computed(() => rightPanelMap[rightTab.value as keyof typeof rightPanelMap]);
const rightPanelTitle = computed(() => {
  switch (rightTab.value) {
    case "diff":
      return "Diff";
    case "git":
      return "Git";
    default:
      return "Timeline";
  }
});

function handleSelectTab(id: string) {
  focusPanel.value = id;
  if (id === "diff" || id === "git" || id === "timeline") {
    rightTab.value = id;
  }
}
</script>

<template>
  <div class="cockpit">
    <div class="cockpit-header">
      <div class="cockpit-title">
        <p class="eyebrow">System</p>
        <h2>Mission Cockpit</h2>
      </div>
      <WorkbenchTabs :tabs="tabs" :active="focusPanel" @select="handleSelectTab" />
    </div>

    <div class="cockpit-grid">
      <section class="rail left-rail">
        <PanelShell title="Workspace" subtitle="Mission control" :class="{ focused: focusPanel === 'mission' }">
          <MissionPanel />
        </PanelShell>
        <PanelShell title="Plan" subtitle="Execution checklist" :class="{ focused: focusPanel === 'plan' }">
          <PlanPanel />
        </PanelShell>
      </section>

      <section class="rail center-rail">
        <PanelShell title="Execution Loop" subtitle="Plan -> Act -> Observe" :class="{ focused: focusPanel === 'loop' }">
          <LoopPanel />
        </PanelShell>
      </section>

      <section class="rail right-rail">
        <PanelShell :title="rightPanelTitle" subtitle="Evidence layer" :class="{ focused: focusPanel === rightTab }">
          <template #actions>
            <div class="hud-tabs">
              <button
                v-for="tab in ['diff', 'git', 'timeline']"
                :key="tab"
                type="button"
                class="hud-tab"
                :class="{ 'is-active': rightTab === tab }"
                @click="rightTab = tab"
              >
                {{ tab }}
              </button>
            </div>
          </template>
          <component :is="rightPanelComponent" />
        </PanelShell>

        <PanelShell title="Telemetry" subtitle="System traffic">
          <div class="telemetry">
            <div class="telemetry-row">
              <span class="telemetry-label">Core status</span>
              <span class="telemetry-value" :class="agentStateClass">{{ agentState }}</span>
            </div>
            <div class="telemetry-row">
              <span>Run</span>
              <span class="telemetry-value text-text-main">{{ shortRunId }}</span>
            </div>
            <div class="telemetry-row">
              <span>Budget</span>
              <span class="telemetry-value text-text-main">{{ budgetLabel }}</span>
            </div>
            <div class="telemetry-bar">
              <div class="telemetry-bar__fill" :style="{ width: `${budgetPercent}%` }"></div>
            </div>
            <div class="telemetry-grid">
              <div class="telemetry-card">
                <span>Total</span>
                <strong>{{ toolStats.total }}</strong>
              </div>
              <div class="telemetry-card">
                <span>Running</span>
                <strong>{{ toolStats.running }}</strong>
              </div>
              <div class="telemetry-card">
                <span>OK</span>
                <strong>{{ toolStats.ok }}</strong>
              </div>
              <div class="telemetry-card">
                <span>Error</span>
                <strong>{{ toolStats.error }}</strong>
              </div>
            </div>
            <div class="telemetry-bar slim">
              <div class="telemetry-bar__fill success" :style="{ width: `${successPercent}%` }"></div>
            </div>
            <div class="telemetry-feed">
              <div v-if="activeTool">
                <p class="feed-title">{{ activeTool.tool }}</p>
                <p class="feed-detail">{{ activeTool.detail }}</p>
              </div>
              <div v-else-if="isStreaming">
                <p class="feed-title">LLM streaming</p>
                <p class="feed-detail">{{ streamPreview || "..." }}</p>
              </div>
              <p v-else class="feed-idle">No activity yet.</p>
            </div>
          </div>
        </PanelShell>
      </section>

      <section class="rail bottom-rail">
        <PanelShell title="Terminal" subtitle="Command console" :class="{ focused: focusPanel === 'terminal' }" :no-padding="true">
          <TerminalPanel />
        </PanelShell>
      </section>
    </div>
  </div>
</template>

<style scoped>
.cockpit {
  display: grid;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.cockpit-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border-radius: 16px;
  border: 1px solid rgba(var(--accent-rgb), 0.35);
  background: rgba(2, 10, 20, 0.85);
  backdrop-filter: blur(12px);
  box-shadow:
    inset 0 1px 0 var(--bevel-light),
    inset 0 -12px 20px var(--bevel-dark),
    inset 0 0 18px rgba(var(--accent-rgb), 0.12),
    0 18px 34px var(--depth-shadow);
  position: relative;
}

.cockpit-header::after {
  content: "";
  position: absolute;
  left: 12px;
  right: 12px;
  top: 0;
  height: 2px;
  background: linear-gradient(90deg, transparent, rgba(var(--accent-rgb), 0.6), transparent);
}

.cockpit-title {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cockpit-title h2 {
  margin: 0;
  font-size: 0.95rem;
  text-transform: uppercase;
  letter-spacing: 0.28em;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-primary);
  text-shadow:
    0 0 12px rgba(var(--accent-rgb), 0.45),
    0 0 24px rgba(var(--accent-3-rgb), 0.2);
}

.cockpit-grid {
  display: grid;
  grid-template-columns: minmax(260px, 320px) minmax(0, 1fr) minmax(280px, 360px);
  grid-template-rows: minmax(0, 1fr) minmax(200px, 260px);
  grid-template-areas:
    "left center right"
    "bottom bottom bottom";
  gap: 14px;
  height: 100%;
  display: grid;
  min-height: 0;
  overflow: hidden;
  position: relative;
  transform-style: preserve-3d;
}

.cockpit-grid::before {
  content: "";
  position: absolute;
  inset: 0;
  background:
    radial-gradient(circle at 50% 35%, rgba(var(--accent-rgb), 0.08), transparent 45%),
    radial-gradient(circle at 88% 70%, rgba(var(--accent-3-rgb), 0.08), transparent 40%);
  pointer-events: none;
  z-index: 0;
}

.rail {
  display: grid;
  gap: 14px;
  min-height: 0;
  overflow: hidden;
  position: relative;
  z-index: 1;
  transform-style: preserve-3d;
}

.left-rail,
.right-rail {
  grid-auto-rows: minmax(0, 1fr);
}

.left-rail {
  grid-area: left;
  --panel-depth: 10px;
  --fog-opacity: 0.14;
}

.center-rail {
  grid-area: center;
  grid-auto-rows: minmax(0, 1fr);
  --panel-depth: 26px;
  --fog-opacity: 0.06;
}

.right-rail {
  grid-area: right;
  grid-template-rows: minmax(0, 1fr) minmax(0, 0.9fr);
  --panel-depth: 12px;
  --fog-opacity: 0.12;
}

.bottom-rail {
  grid-area: bottom;
  min-height: 0;
  --panel-depth: 18px;
  --fog-opacity: 0.1;
}

.focused {
  box-shadow: 0 0 0 1px rgba(var(--accent-rgb), 0.5), 0 0 26px rgba(var(--accent-rgb), 0.2);
  transform: translateZ(calc(var(--panel-depth, 16px) + 12px));
}

.telemetry {
  display: grid;
  gap: 10px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.telemetry-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  color: var(--text-tertiary);
}

.telemetry-label {
  color: var(--text-tertiary);
}

.telemetry-value {
  font-size: 0.7rem;
  letter-spacing: 0.12em;
  text-transform: none;
  font-family: "JetBrains Mono", monospace;
}

.telemetry-bar {
  height: 6px;
  border-radius: 999px;
  background: rgba(var(--line-rgb), 0.2);
  overflow: hidden;
}

.telemetry-bar.slim {
  height: 4px;
}

.telemetry-bar__fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, rgba(var(--accent-rgb), 0.7), rgba(var(--accent-3-rgb), 0.6));
  box-shadow: 0 0 14px rgba(var(--accent-rgb), 0.35);
}

.telemetry-bar__fill.success {
  background: linear-gradient(90deg, rgba(var(--accent-2-rgb), 0.8), rgba(var(--accent-rgb), 0.6));
}

.telemetry-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.telemetry-card {
  border-radius: 10px;
  border: 1px solid rgba(var(--accent-rgb), 0.3);
  background: rgba(2, 10, 20, 0.8);
  backdrop-filter: blur(12px);
  box-shadow:
    inset 0 1px 0 var(--bevel-light),
    inset 0 -10px 16px var(--bevel-dark),
    inset 0 0 12px rgba(var(--accent-rgb), 0.08),
    0 10px 20px var(--depth-shadow);
  padding: 8px 10px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  font-size: 0.6rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  color: var(--text-tertiary);
}

.telemetry-card strong {
  font-size: 0.75rem;
  color: var(--text-primary);
}

.telemetry-feed {
  border-top: 1px solid rgba(var(--line-rgb), 0.24);
  padding-top: 8px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.feed-title {
  margin: 0 0 4px;
  color: var(--accent);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.16em;
  text-shadow: 0 0 10px rgba(var(--accent-rgb), 0.4);
}

.feed-detail {
  margin: 0;
  color: var(--text-soft);
  font-size: 0.75rem;
  word-break: break-word;
}

.feed-idle {
  margin: 0;
  color: var(--text-tertiary);
}

@media (max-width: 1200px) {
  .cockpit-grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto;
    grid-template-areas:
      "left"
      "center"
      "right"
      "bottom";
    overflow: auto;
  }

  .right-rail {
    grid-template-rows: auto;
  }
}
</style>

