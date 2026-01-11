<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from "vue";
import AgentPanel from "../components/AgentPanel.vue";
import ChatPanel from "../components/ChatPanel.vue";
import TerminalPanel from "../components/TerminalPanel.vue";
import WorkspacePanel from "../components/WorkspacePanel.vue";

const isAgentDetached = ref(false);

const floatState = reactive({
  x: window.innerWidth - 450,
  y: 100,
  w: 400,
  h: 600,
});

const dragging = reactive({
  active: false,
  offX: 0,
  offY: 0,
});

const resizing = reactive({
  active: false,
  dir: "",
  startX: 0,
  startY: 0,
  startW: 0,
  startH: 0,
});

const layout = reactive({
  leftWidth: 280,
  rightWidth: 340,
});

const gridResizing = reactive({
  active: false,
  pane: "", // 'left' or 'right'
  startX: 0,
  startWidth: 0,
});

function toggleAgentDetach() {
  isAgentDetached.value = !isAgentDetached.value;
  if (isAgentDetached.value) {
    floatState.x = Math.max(20, window.innerWidth - 450);
    floatState.y = 100;
  }
}

function startGridResize(e: MouseEvent, pane: string) {
  gridResizing.active = true;
  gridResizing.pane = pane;
  gridResizing.startX = e.clientX;
  gridResizing.startWidth = pane === 'left' ? layout.leftWidth : layout.rightWidth;
  window.addEventListener("mousemove", onGridResize);
  window.addEventListener("mouseup", stopGridResize);
  document.body.style.cursor = "col-resize";
  e.preventDefault();
}

function onGridResize(e: MouseEvent) {
  if (!gridResizing.active) return;
  const dx = e.clientX - gridResizing.startX;
  
  if (gridResizing.pane === 'left') {
    layout.leftWidth = Math.max(200, Math.min(600, gridResizing.startWidth + dx));
  } else {
    // For right panel, dragging left (negative dx) increases width
    layout.rightWidth = Math.max(250, Math.min(800, gridResizing.startWidth - dx));
  }
}

function stopGridResize() {
  gridResizing.active = false;
  window.removeEventListener("mousemove", onGridResize);
  window.removeEventListener("mouseup", stopGridResize);
  document.body.style.cursor = "";
}

function startDrag(e: MouseEvent) {
  if ((e.target as HTMLElement).closest("button")) return;
  dragging.active = true;
  dragging.offX = e.clientX - floatState.x;
  dragging.offY = e.clientY - floatState.y;
  window.addEventListener("mousemove", onDrag);
  window.addEventListener("mouseup", stopDrag);
}

function onDrag(e: MouseEvent) {
  if (!dragging.active) return;
  floatState.x = e.clientX - dragging.offX;
  floatState.y = e.clientY - dragging.offY;
}

function stopDrag() {
  dragging.active = false;
  window.removeEventListener("mousemove", onDrag);
  window.removeEventListener("mouseup", stopDrag);
}

function startResize(e: MouseEvent, dir: string) {
  resizing.active = true;
  resizing.dir = dir;
  resizing.startX = e.clientX;
  resizing.startY = e.clientY;
  resizing.startW = floatState.w;
  resizing.startH = floatState.h;
  window.addEventListener("mousemove", onResize);
  window.addEventListener("mouseup", stopResize);
  e.stopPropagation();
  e.preventDefault();
}

function onResize(e: MouseEvent) {
  if (!resizing.active) return;
  const dx = e.clientX - resizing.startX;
  const dy = e.clientY - resizing.startY;

  if (resizing.dir.includes("r")) {
    floatState.w = Math.max(300, resizing.startW + dx);
  }
  if (resizing.dir.includes("b")) {
    floatState.h = Math.max(200, resizing.startH + dy);
  }
}

function stopResize() {
  resizing.active = false;
  window.removeEventListener("mousemove", onResize);
  window.removeEventListener("mouseup", stopResize);
}
</script>

<template>
  <div 
    class="workbench-grid" 
    :class="{ 'has-detached-sidebar': isAgentDetached }"
    :style="{ 
      '--left-width': layout.leftWidth + 'px',
      '--right-width': layout.rightWidth + 'px'
    }"
  >
    <section class="pane workspace-pane">
      <WorkspacePanel />
    </section>
    
    <div class="grid-resizer left-resizer" @mousedown="startGridResize($event, 'left')"></div>

    <section class="pane center-pane">
      <div class="center-stack">
        <div class="stack-pane chat-pane">
          <ChatPanel />
        </div>
        <div class="stack-pane terminal-pane">
          <TerminalPanel />
        </div>
      </div>
    </section>

    <template v-if="!isAgentDetached">
      <div class="grid-resizer right-resizer" @mousedown="startGridResize($event, 'right')"></div>
      <section class="pane agent-pane">
        <AgentPanel @toggle-detach="toggleAgentDetach" />
      </section>
    </template>
  </div>

  <Teleport to="body">
    <div
      v-if="isAgentDetached"
      class="floating-window"
      :style="{
        left: floatState.x + 'px',
        top: floatState.y + 'px',
        width: floatState.w + 'px',
        height: floatState.h + 'px',
      }"
    >
      <div class="window-drag-area" @mousedown="startDrag"></div>

      <div class="window-content">
        <AgentPanel @toggle-detach="toggleAgentDetach" :is-detached="true" />
      </div>

      <div class="resize-handle corner" @mousedown="startResize($event, 'br')">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M22 22H20V20H22V22ZM22 18H20V16H22V18ZM18 22H16V20H18V22ZM18 18H16V16H18V18ZM14 22H12V20H14V22ZM22 14H20V12H22V14Z" />
        </svg>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.workbench-grid {
  display: grid;
  grid-template-columns: var(--left-width) 6px minmax(0, 1fr) 6px var(--right-width);
  gap: 0;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.workbench-grid.has-detached-sidebar {
  grid-template-columns: var(--left-width) 6px minmax(0, 1fr);
}

.grid-resizer {
  width: 6px;
  cursor: col-resize;
  background: linear-gradient(
    180deg,
    rgba(45, 246, 255, 0.1),
    rgba(8, 12, 20, 0.1),
    rgba(45, 246, 255, 0.1)
  );
  transition: background 0.2s;
  z-index: 10;
}
.grid-resizer:hover, .grid-resizer:active {
  background: rgba(45, 246, 255, 0.25);
}

.pane {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 18px;
  box-shadow: var(--shadow);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: rise 0.35s ease;
}

.center-pane {
  background: transparent;
  border: none;
  box-shadow: none;
}

.center-stack {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 10px;
}

.stack-pane {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  border-radius: 16px;
  background: var(--panel);
  border: 1px solid var(--line);
}

.chat-pane {
  box-shadow: inset 0 0 22px rgba(45, 246, 255, 0.06);
}

/* Floating Window Styles */
.floating-window {
  position: fixed;
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 16px;
  box-shadow: var(--shadow);
  display: flex;
  flex-direction: column;
  z-index: 1000;
  overflow: hidden;
}

.window-drag-area {
  height: 24px;
  background: var(--panel-glass);
  cursor: move;
  flex-shrink: 0;
  border-bottom: 1px solid var(--line);
  display: flex;
  align-items: center;
  justify-content: center;
}
.window-drag-area::after {
  content: ":::";
  color: var(--text-tertiary);
  font-size: 10px;
  letter-spacing: 2px;
}

.window-content {
  flex: 1;
  overflow: hidden;
  position: relative;
}

.resize-handle.corner {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 16px;
  height: 16px;
  cursor: se-resize;
  color: var(--muted);
  pointer-events: auto;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
}

@media (max-width: 1200px) {
  .workbench-grid {
    grid-template-columns: 1fr;
    gap: 12px;
    overflow: auto;
  }

  .workbench-grid.has-detached-sidebar {
    grid-template-columns: 1fr;
  }

  .grid-resizer {
    display: none;
  }

  .pane {
    min-height: 320px;
  }
}
</style>
