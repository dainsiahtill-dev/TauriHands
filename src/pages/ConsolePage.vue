<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
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

const route = useRoute();
const router = useRouter();

const focusPanel = ref("loop");
const rightTab = ref("timeline");
const leftPanelCollapsed = ref(false);
const missionSectionCollapsed = ref(false);
const planSectionCollapsed = ref(false);
const rightPanelCollapsed = ref(false);
const bottomPanelCollapsed = ref(false);

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

const singleViewConfig = computed(() => {
  switch (route.name) {
    case "mission":
      return { title: "Mission", subtitle: "Workspace & task setup", panel: MissionPanel, noPadding: false };
    case "plan":
      return { title: "Plan", subtitle: "Execution checklist", panel: PlanPanel, noPadding: false };
    case "loop":
      return { title: "Loop", subtitle: "Run control & chat", panel: LoopPanel, noPadding: false };
    case "terminal":
      return { title: "Terminal", subtitle: "Command console", panel: TerminalPanel, noPadding: true };
    case "diff":
      return { title: "Diff", subtitle: "Working tree changes", panel: DiffPanel, noPadding: false };
    case "git":
      return { title: "Git", subtitle: "Repository status", panel: GitPanel, noPadding: false };
    case "timeline":
      return { title: "Timeline", subtitle: "Event stream", panel: TimelinePanel, noPadding: false };
    default:
      return null;
  }
});

const isSingleView = computed(() => route.name !== "console" && !!singleViewConfig.value);

const gridStyle = computed(() => {
  const rightSize = rightPanelCollapsed.value ? "0px" : "minmax(260px, 360px)";
  const bottomSize = bottomPanelCollapsed.value ? "0px" : "minmax(200px, 260px)";
  return {
    gridTemplateColumns: `minmax(240px, 320px) minmax(0, 1fr) ${rightSize}`,
    gridTemplateRows: `minmax(0, 1fr) ${bottomSize}`,
    gridTemplateAreas: "\"left center right\" \"bottom bottom bottom\"",
  };
});

const cockpitFocusMap: Record<string, string> = {
  console: "loop",
  mission: "mission",
  plan: "plan",
  loop: "loop",
  terminal: "terminal",
  diff: "diff",
  git: "git",
  timeline: "timeline",
};

function syncFromRoute(name: string | undefined) {
  const routeName = name ?? "console";
  const nextFocus = cockpitFocusMap[routeName] ?? "loop";
  focusPanel.value = nextFocus;
  if (routeName === "console") {
    rightTab.value = "timeline";
    return;
  }
  if (routeName === "diff" || routeName === "git" || routeName === "timeline") {
    rightTab.value = routeName;
  }
}

function handleSelectTab(id: string) {
  if (route.name === id) return;
  void router.push({ name: id });
}

function selectRightTab(tab: string) {
  if (rightTab.value === tab) return;
  rightTab.value = tab;
  if (route.name !== tab) {
    void router.push({ name: tab });
  }
}

function toggleLeftPanel() {
  leftPanelCollapsed.value = !leftPanelCollapsed.value;
}

function toggleMissionSection() {
  missionSectionCollapsed.value = !missionSectionCollapsed.value;
}

function togglePlanSection() {
  planSectionCollapsed.value = !planSectionCollapsed.value;
}

function toggleRightPanel() {
  rightPanelCollapsed.value = !rightPanelCollapsed.value;
}

function toggleBottomPanel() {
  bottomPanelCollapsed.value = !bottomPanelCollapsed.value;
}

function openLeftPanel(section: "mission" | "plan") {
  if (leftPanelCollapsed.value) {
    leftPanelCollapsed.value = false;
  }
  if (section === "mission") {
    missionSectionCollapsed.value = false;
  }
  if (section === "plan") {
    planSectionCollapsed.value = false;
  }
  focusPanel.value = section;
}

function goToConsole() {
  void router.push({ name: "console" });
}

watch(
  () => route.name,
  (name) => {
    syncFromRoute(name as string | undefined);
  },
  { immediate: true },
);
</script>

<template>
  <div class="cockpit">
    <div class="cockpit-header">
      <div class="cockpit-title">
        <p class="eyebrow">System</p>
        <h2>Mission Cockpit</h2>
      </div>
      <WorkbenchTabs v-if="!isSingleView" :tabs="tabs" :active="focusPanel" @select="handleSelectTab" />
      <div class="cockpit-actions">
        <button v-if="isSingleView" class="header-toggle" type="button" @click="goToConsole">
          Open cockpit
        </button>
        <template v-else>
          <button class="header-toggle" type="button" @click="toggleRightPanel">
            {{ rightPanelCollapsed ? "Show side" : "Hide side" }}
          </button>
          <button class="header-toggle" type="button" @click="toggleBottomPanel">
            {{ bottomPanelCollapsed ? "Show terminal" : "Hide terminal" }}
          </button>
        </template>
      </div>
    </div>

    <div v-if="isSingleView && singleViewConfig" class="single-view">
      <PanelShell
        :title="singleViewConfig.title"
        :subtitle="singleViewConfig.subtitle"
        :no-padding="singleViewConfig.noPadding"
        class="single-panel"
      >
        <component :is="singleViewConfig.panel" />
      </PanelShell>
    </div>

    <div v-else class="cockpit-grid" :style="gridStyle">
      <section class="rail left-rail" :class="{ 'is-collapsed': leftPanelCollapsed }">
        <div class="left-rail__header">
          <button class="collapse-toggle" @click="toggleLeftPanel" :class="{ 'is-collapsed': leftPanelCollapsed }">
            <span class="toggle-icon">{{ leftPanelCollapsed ? ">>" : "<<" }}</span>
          </button>
          <span class="rail-title">Control Panel</span>
        </div>
        <div v-show="leftPanelCollapsed" class="left-rail__mini">
          <button class="mini-icon" type="button" @click="openLeftPanel('mission')">M</button>
          <button class="mini-icon" type="button" @click="openLeftPanel('plan')">P</button>
        </div>
        
        <div v-show="!leftPanelCollapsed" class="left-rail__content">
          <PanelShell title="Workspace" subtitle="Mission control" :class="{ focused: focusPanel === 'mission' }">
            <template #actions>
              <button class="section-toggle" @click="toggleMissionSection" :class="{ 'is-collapsed': missionSectionCollapsed }">
                {{ missionSectionCollapsed ? "+" : "-" }}
              </button>
            </template>
            <div
              class="collapsible-section"
              :class="{ 'is-collapsed': missionSectionCollapsed }"
              :aria-hidden="missionSectionCollapsed"
            >
              <MissionPanel />
            </div>
          </PanelShell>
          
          <PanelShell title="Plan" subtitle="Execution checklist" :class="{ focused: focusPanel === 'plan' }">
            <template #actions>
              <button class="section-toggle" @click="togglePlanSection" :class="{ 'is-collapsed': planSectionCollapsed }">
                {{ planSectionCollapsed ? "+" : "-" }}
              </button>
            </template>
            <div
              class="collapsible-section"
              :class="{ 'is-collapsed': planSectionCollapsed }"
              :aria-hidden="planSectionCollapsed"
            >
              <PlanPanel />
            </div>
          </PanelShell>
        </div>
      </section>

      <section class="rail center-rail">
        <PanelShell title="Execution Loop" subtitle="Plan -> Act -> Observe" :class="{ focused: focusPanel === 'loop' }">
          <LoopPanel />
        </PanelShell>
      </section>

      <section class="rail right-rail" :class="{ 'is-collapsed': rightPanelCollapsed }">
        <PanelShell :title="rightPanelTitle" subtitle="Evidence layer" :class="{ focused: focusPanel === rightTab }">
          <template #actions>
            <div class="hud-tabs">
              <button
                v-for="tab in ['diff', 'git', 'timeline']"
                :key="tab"
                type="button"
                class="hud-tab"
                :class="{ 'is-active': rightTab === tab }"
                @click="selectRightTab(tab)"
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

      <section class="rail bottom-rail" :class="{ 'is-collapsed': bottomPanelCollapsed }">
        <PanelShell title="Terminal" subtitle="Command console" :class="{ focused: focusPanel === 'terminal' }" :no-padding="true">
          <TerminalPanel />
        </PanelShell>
      </section>
    </div>
  </div>
</template>

<style scoped>
.cockpit {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
  min-height: 0;
}

.cockpit-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 16px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
  box-shadow: 0 12px 24px rgba(8, 12, 18, 0.18);
}

.cockpit-title {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cockpit-title h2 {
  margin: 0;
  font-size: 1rem;
  letter-spacing: 0.02em;
  font-family: var(--font-display);
  color: var(--text-primary);
}

.cockpit-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

.header-toggle {
  padding: 6px 12px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.7rem;
  letter-spacing: 0.04em;
  cursor: pointer;
  transition: all 0.2s ease;
}

.header-toggle:hover {
  border-color: rgba(var(--accent-rgb), 0.35);
  color: var(--text-primary);
  box-shadow: 0 8px 16px rgba(var(--accent-rgb), 0.12);
}

.single-view {
  flex: 1;
  min-height: 0;
  display: grid;
}

.single-panel {
  height: 100%;
}


.cockpit-grid {
  display: grid;
  transition: grid-template-columns 0.25s ease, grid-template-rows 0.25s ease;
  gap: 14px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}


.rail {
  display: grid;
  gap: 14px;
  min-height: 0;
  overflow: hidden;
}

.left-rail,
.right-rail {
  grid-auto-rows: minmax(0, 1fr);
}

.left-rail {
  grid-area: left;
  transition: all 0.3s ease;
}

.left-rail.is-collapsed {
  min-width: 60px;
  max-width: 60px;
}

.left-rail.is-collapsed .rail-title {
  display: none;
}

.left-rail.is-collapsed .left-rail__header {
  justify-content: center;
}


.left-rail__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(var(--line-rgb), 0.18);
}

.rail-title {
  font-size: 0.7rem;
  letter-spacing: 0.02em;
  color: var(--text-secondary);
  font-family: var(--font-display);
}

.collapse-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.collapse-toggle:hover {
  border-color: rgba(var(--accent-rgb), 0.4);
  color: var(--text-primary);
}

.toggle-icon {
  font-size: 0.8rem;
  transition: transform 0.2s ease, opacity 0.2s ease;
  font-weight: 600;
}

.collapse-toggle.is-collapsed .toggle-icon {
  transform: rotate(180deg);
}

.left-rail__content {
  display: grid;
  gap: 14px;
  min-height: 0;
  overflow: auto;
  padding-right: 4px;
}

.section-toggle {
  padding: 4px 8px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.65rem;
  cursor: pointer;
  transition: all 0.2s ease;
  border-radius: 10px;
}

.collapsible-section {
  max-height: 1200px;
  opacity: 1;
  transform: translateY(0);
  transition: max-height 0.25s ease, opacity 0.2s ease, transform 0.25s ease;
  overflow: hidden;
}

.collapsible-section.is-collapsed {
  max-height: 0;
  opacity: 0;
  transform: translateY(-6px);
  pointer-events: none;
}


.section-toggle:hover {
  border-color: rgba(var(--accent-rgb), 0.35);
  color: var(--text-primary);
}

.section-toggle.is-collapsed {
  transform: rotate(180deg);
}

.center-rail {
  grid-area: center;
  grid-auto-rows: minmax(0, 1fr);
}

.right-rail {
  grid-area: right;
  grid-template-rows: minmax(0, 1fr) minmax(0, 0.9fr);
  transition: opacity 0.2s ease, transform 0.25s ease;
  min-width: 0;
}

.bottom-rail {
  grid-area: bottom;
  min-height: 0;
  transition: opacity 0.2s ease, transform 0.25s ease;
  min-width: 0;
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
  font-size: 0.65rem;
  letter-spacing: 0.08em;
  color: var(--text-tertiary);
}

.telemetry-value {
  font-size: 0.7rem;
  letter-spacing: 0.04em;
  color: var(--text-primary);
}

.telemetry-bar {
  height: 6px;
  border-radius: 999px;
  background: rgba(var(--line-rgb), 0.18);
  overflow: hidden;
}

.telemetry-bar.slim {
  height: 4px;
}

.telemetry-bar__fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, rgba(var(--accent-rgb), 0.7), rgba(var(--accent-3-rgb), 0.6));
}

.telemetry-bar__fill.success {
  background: linear-gradient(90deg, rgba(var(--accent-2-rgb), 0.7), rgba(var(--accent-rgb), 0.5));
}

.telemetry-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.telemetry-card {
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
  padding: 8px 10px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  font-size: 0.65rem;
  letter-spacing: 0.03em;
  color: var(--text-tertiary);
}

.telemetry-card strong {
  font-size: 0.8rem;
  color: var(--text-primary);
}

.telemetry-feed {
  border-top: 1px solid rgba(var(--line-rgb), 0.18);
  padding-top: 8px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.feed-title {
  margin: 0 0 4px;
  color: var(--accent);
  font-size: 0.7rem;
  letter-spacing: 0.08em;
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


.right-rail.is-collapsed {
  opacity: 0;
  transform: translateX(12px);
  pointer-events: none;
}

.bottom-rail.is-collapsed {
  opacity: 0;
  transform: translateY(12px);
  pointer-events: none;
}

.left-rail__mini {
  display: grid;
  gap: 10px;
  padding: 10px 8px;
  justify-items: center;
}

.mini-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  cursor: pointer;
  transition: all 0.2s ease;
}

.mini-icon:hover {
  border-color: rgba(var(--accent-rgb), 0.4);
  color: var(--text-primary);
  box-shadow: 0 8px 16px rgba(var(--accent-rgb), 0.12);
}
@media (max-width: 1200px) {
  .cockpit-grid {
    grid-template-columns: 1fr !important;
    grid-template-rows: auto !important;
    grid-template-areas:
      "left"
      "center"
      "right"
      "bottom" !important;
    overflow: auto;
  }

  .right-rail {
    grid-template-rows: auto;
  }
}
</style>
