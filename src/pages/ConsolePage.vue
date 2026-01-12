<script setup lang="ts">
import { computed, ref, reactive } from "vue";
import WorkbenchTabs from "../components/WorkbenchTabs.vue";
import PanelShell from "../components/PanelShell.vue";
import MissionPanel from "../components/MissionPanel.vue";
import PlanPanel from "../components/PlanPanel.vue";
import LoopPanel from "../components/LoopPanel.vue";
import TerminalPanel from "../components/TerminalPanel.vue";
import DiffPanel from "../components/DiffPanel.vue";
import GitPanel from "../components/GitPanel.vue";
import TimelinePanel from "../components/TimelinePanel.vue";

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

const layout = reactive({
  leftWidth: 320,
  rightWidth: 360,
});

const gridResizing = reactive({
  active: false,
  pane: "", // 'left' or 'right'
  startX: 0,
  startWidth: 0,
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

function startGridResize(e: MouseEvent, pane: string) {
  gridResizing.active = true;
  gridResizing.pane = pane;
  gridResizing.startX = e.clientX;
  gridResizing.startWidth = pane === "left" ? layout.leftWidth : layout.rightWidth;
  window.addEventListener("mousemove", onGridResize);
  window.addEventListener("mouseup", stopGridResize);
  document.body.style.cursor = "col-resize";
  e.preventDefault();
}

function onGridResize(e: MouseEvent) {
  if (!gridResizing.active) return;
  const dx = e.clientX - gridResizing.startX;
  if (gridResizing.pane === "left") {
    layout.leftWidth = Math.max(240, Math.min(520, gridResizing.startWidth + dx));
  } else {
    layout.rightWidth = Math.max(280, Math.min(600, gridResizing.startWidth - dx));
  }
}

function stopGridResize() {
  gridResizing.active = false;
  window.removeEventListener("mousemove", onGridResize);
  window.removeEventListener("mouseup", stopGridResize);
  document.body.style.cursor = "";
}
</script>

<template>
  <div class="cockpit">
    <WorkbenchTabs :tabs="tabs" :active="focusPanel" @select="handleSelectTab" />

    <div
      class="cockpit-grid"
      :style="{ '--left-width': layout.leftWidth + 'px', '--right-width': layout.rightWidth + 'px' }"
    >
      <section class="rail left-rail">
        <PanelShell title="Mission" subtitle="Task control" :class="{ focused: focusPanel === 'mission' }">
          <MissionPanel />
        </PanelShell>
        <PanelShell title="Plan" subtitle="Checklist" :class="{ focused: focusPanel === 'plan' }">
          <PlanPanel />
        </PanelShell>
      </section>

      <div class="grid-resizer left-resizer" @mousedown="startGridResize($event, 'left')"></div>

      <section class="rail center-rail">
        <PanelShell title="Loop" subtitle="Execution" :class="{ focused: focusPanel === 'loop' }">
          <LoopPanel />
        </PanelShell>
        <PanelShell title="Terminal" subtitle="Evidence" :class="{ focused: focusPanel === 'terminal' }">
          <TerminalPanel />
        </PanelShell>
      </section>

      <div class="grid-resizer right-resizer" @mousedown="startGridResize($event, 'right')"></div>

      <section class="rail right-rail">
        <PanelShell :title="rightPanelTitle" subtitle="Evidence" :class="{ focused: focusPanel === rightTab }">
          <component :is="rightPanelComponent" />
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

.cockpit-grid {
  display: grid;
  grid-template-columns: var(--left-width) 6px minmax(0, 1fr) 6px var(--right-width);
  gap: 0;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.grid-resizer {
  width: 6px;
  cursor: col-resize;
  background: linear-gradient(
    180deg,
    rgba(45, 246, 255, 0.08),
    rgba(8, 12, 20, 0.08),
    rgba(45, 246, 255, 0.08)
  );
  transition: background 0.2s;
  z-index: 10;
}

.grid-resizer:hover,
.grid-resizer:active {
  background: rgba(45, 246, 255, 0.2);
}

.rail {
  display: grid;
  gap: 12px;
  min-height: 0;
  overflow: hidden;
}

.left-rail,
.right-rail {
  grid-auto-rows: minmax(0, 1fr);
}

.center-rail {
  grid-auto-rows: minmax(0, 1fr);
}

.focused {
  box-shadow: 0 0 0 1px rgba(45, 246, 255, 0.35), var(--shadow);
}

@media (max-width: 1200px) {
  .cockpit-grid {
    grid-template-columns: 1fr;
    gap: 12px;
    overflow: auto;
  }

  .grid-resizer {
    display: none;
  }

  .rail {
    grid-auto-rows: auto;
  }
}
</style>
