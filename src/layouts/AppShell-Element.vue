<template>
  <el-container class="app-container">
    <!-- Header -->
    <el-header class="app-header">
      <div class="header-left">
        <div class="brand-section">
          <el-avatar :size="40" class="brand-avatar">
            <el-icon><Lightning /></el-icon>
          </el-avatar>
          <div class="brand-text">
            <h1 class="brand-title">TauriHands</h1>
            <p class="brand-subtitle">AI Development Agent</p>
          </div>
        </div>
        <div class="workspace-chip">
          <span class="chip-label">Workspace</span>
          <span class="chip-value">{{ workspaceName }}</span>
        </div>
      </div>

      <div class="header-center">
        <div class="status-pill" :data-status="agentState">
          <span class="status-dot"></span>
          <span class="status-text">{{ agentState }}</span>
        </div>
        <div class="meta-inline">Run <strong>{{ shortRunId }}</strong></div>
        <div class="meta-inline">Budget <strong>{{ budgetLabel }}</strong></div>
      </div>

      <div class="header-right">
        <el-button type="primary" :icon="Search" circle @click="handleSearch" />
        <ThemeToggle />
        <el-button :icon="Setting" circle @click="handleSettings" />
      </div>
    </el-header>

    <!-- Main Container -->
    <el-container class="main-container">
      <!-- Sidebar -->
      <el-aside :width="sidebarWidth" class="app-sidebar">
        <el-menu
          :default-active="activeMenu"
          :collapse="isCollapse"
          :unique-opened="true"
          class="sidebar-menu"
        >
          <el-menu-item
            v-for="item in menuItems"
            :key="item.id"
            :index="item.id"
            @click="navigateTo(item.id)"
          >
            <el-icon><component :is="item.icon" /></el-icon>
            <template #title>{{ item.label }}</template>
          </el-menu-item>
        </el-menu>
      </el-aside>

      <!-- Main Content -->
      <el-main class="app-main">
        <div class="main-content">
          <router-view />
        </div>
      </el-main>

      <!-- Right Panel -->
      <el-aside v-if="showRightPanel" :width="300" class="right-panel">
        <el-card class="status-card" shadow="never">
          <div class="status-header">
            <div>
              <p class="card-eyebrow">Run Control</p>
              <h3 class="card-title">Agent status</h3>
            </div>
            <el-tag :type="getStatusType(agentState)" size="small">
              {{ agentState }}
            </el-tag>
          </div>

          <div class="status-grid">
            <div class="status-row">
              <span>Workspace</span>
              <strong>{{ workspaceName }}</strong>
            </div>
            <div class="status-row">
              <span>Run ID</span>
              <strong>{{ shortRunId }}</strong>
            </div>
            <div class="status-row">
              <span>Budget</span>
              <strong>{{ budgetLabel }}</strong>
            </div>
            <div class="status-row">
              <span>Power</span>
              <strong>{{ powerPercent }}%</strong>
            </div>
          </div>

          <div class="status-progress">
            <el-progress
              :percentage="budgetPercent"
              :status="budgetPercent > 80 ? 'exception' : 'success'"
              :show-text="false"
              :stroke-width="6"
            />
          </div>

          <div class="action-buttons">
            <el-button type="primary" @click="handleContinue" :loading="isLoading" :disabled="!canContinue">
              <el-icon><VideoPlay /></el-icon>
              Continue
            </el-button>
            <el-button @click="handlePause" :disabled="!canPause">
              <el-icon><VideoPause /></el-icon>
              Pause
            </el-button>
            <el-button type="danger" @click="handleStop">
              <el-icon><CircleCloseFilled /></el-icon>
              Stop
            </el-button>
          </div>
        </el-card>

        <el-card class="tools-card" shadow="never">
          <div class="status-header">
            <div>
              <p class="card-eyebrow">Tooling</p>
              <h3 class="card-title">Tool status</h3>
            </div>
            <el-tag type="info" size="small">{{ toolStats.running }} running</el-tag>
          </div>

          <div v-if="activeTool" class="active-tool">
            <el-tag type="primary">{{ activeTool.tool }}</el-tag>
            <el-text type="info" size="small">{{ activeTool.detail }}</el-text>
          </div>
          <div v-else class="no-tool">
            <el-text type="info">No active tools</el-text>
          </div>

          <div class="tool-stats">
            <div class="tool-stat">
              <span>Total</span>
              <strong>{{ toolStats.total }}</strong>
            </div>
            <div class="tool-stat">
              <span>Success</span>
              <strong>{{ toolStats.ok }}</strong>
            </div>
            <div class="tool-stat">
              <span>Running</span>
              <strong>{{ toolStats.running }}</strong>
            </div>
            <div class="tool-stat">
              <span>Errors</span>
              <strong>{{ toolStats.error }}</strong>
            </div>
          </div>
        </el-card>
      </el-aside>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { agentStore } from '../agents/orchestrator'
import ThemeToggle from '../components/ThemeToggle-Simple.vue'
import {
  Lightning,
  Search,
  Setting,
  Tools,
  Lock,
  Monitor,
  Document,
  Connection,
  Platform,
  Files,
  Link,
  Clock,
  VideoPlay,
  VideoPause,
  CircleCloseFilled
} from '@element-plus/icons-vue'

const router = useRouter()
const route = useRoute()
const { state, initKernelStore, pause, resume, continueRun, stop } = agentStore

// State
const isCollapse = ref(false)
const showRightPanel = ref(true)
const isLoading = ref(false)

// Computed
const run = computed(() => state.run)
const agentState = computed(() => run.value?.agentState ?? "IDLE")
const isAwaiting = computed(() => agentState.value === "AWAITING_USER")
const isPaused = computed(() => agentState.value === "PAUSED")
const isRunning = computed(() => agentState.value === "RUNNING")
const canContinue = computed(() => isAwaiting.value || isPaused.value)
const canPause = computed(() => isRunning.value)
const needsAttention = computed(() => agentState.value === "AWAITING_USER")
const runId = computed(() => run.value?.runId ?? "-")
const shortRunId = computed(() => (runId.value ? runId.value.slice(0, 8) : "-"))
const workspacePath = computed(() => run.value?.toolContext?.cwd ?? "Not set")
const workspaceName = computed(() => {
  if (!workspacePath.value || workspacePath.value === "Not set") return "Not set"
  const parts = workspacePath.value.split(/[\\/]/).filter(Boolean)
  return parts[parts.length - 1] ?? workspacePath.value
})

const budget = computed(() => run.value?.budget)
const budgetLabel = computed(() =>
  budget.value ? `${budget.value.usedSteps}/${budget.value.maxSteps}` : "-"
)
const budgetPercent = computed(() => {
  const current = budget.value
  if (!current || current.maxSteps === 0) return 0
  return Math.min(100, Math.round((current.usedSteps / current.maxSteps) * 100))
})

const toolCalls = computed(() => state.toolCalls ?? [])
const activeTool = computed(() => toolCalls.value.find((call) => call.status === "running"))
const toolStats = computed(() => {
  const stats = { total: 0, running: 0, ok: 0, error: 0 }
  for (const call of toolCalls.value) {
    stats.total += 1
    if (call.status === "running") stats.running += 1
    if (call.status === "ok") stats.ok += 1
    if (call.status === "error") stats.error += 1
  }
  return stats
})

const powerPercent = ref(85)
const sidebarWidth = computed(() => isCollapse.value ? '64px' : '200px')
const activeMenu = computed(() => route.name as string || 'console')

const cockpitRoutes = new Set([
  "console",
  "mission",
  "plan",
  "loop",
  "terminal",
  "diff",
  "git",
  "timeline",
])

// Menu items
const menuItems = [
  { id: 'console', label: 'Console', icon: Monitor },
  { id: 'mission', label: 'Mission', icon: Document },
  { id: 'plan', label: 'Plan', icon: Connection },
  { id: 'loop', label: 'Loop', icon: Lightning },
  { id: 'terminal', label: 'Terminal', icon: Platform },
  { id: 'diff', label: 'Diff', icon: Files },
  { id: 'git', label: 'Git', icon: Link },
  { id: 'timeline', label: 'Timeline', icon: Clock },
  { id: 'llm', label: 'Settings', icon: Setting },
  { id: 'tools', label: 'Tools', icon: Tools },
  { id: 'security', label: 'Security', icon: Lock }
]

// Methods
const navigateTo = (id: string) => {
  router.push({ name: id })
}

const handleSearch = async () => {
  if (!cockpitRoutes.has(route.name as string)) {
    await router.push({ name: "console" })
  }
  await nextTick()
  window.dispatchEvent(new Event("focus-chat-input"))
}

const handleSettings = () => {
  router.push({ name: "llm" })
}

const getStatusType = (state: string) => {
  switch (state) {
    case 'RUNNING': return 'success'
    case 'AWAITING_USER': return 'warning'
    case 'ERROR': return 'danger'
    default: return 'info'
  }
}

const handleContinue = async () => {
  if (!canContinue.value) return
  isLoading.value = true
  try {
    if (isPaused.value) {
      await resume()
    } else {
      await continueRun()
    }
  } finally {
    isLoading.value = false
  }
}

const handlePause = async () => {
  if (!canPause.value) return
  await pause()
}

const handleStop = () => {
  stop()
}

onMounted(() => {
  initKernelStore()
})
</script>

<style scoped>
.app-container {
  height: 100vh;
  background-color: var(--el-bg-color);
  color: var(--text-primary);
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 20px;
  height: 68px;
  background: linear-gradient(
    135deg,
    rgba(var(--bg-rgb), 0.92),
    rgba(var(--accent-rgb), 0.14),
    rgba(var(--accent-3-rgb), 0.08)
  );
  border-bottom: 1px solid rgba(var(--line-rgb), 0.18);
  box-shadow: 0 10px 24px rgba(5, 10, 18, 0.22);
}

.header-left,
.header-center,
.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.header-left {
  flex: 1;
  min-width: 0;
}

.header-center {
  flex: 1;
  justify-content: center;
  flex-wrap: wrap;
}

.header-right {
  flex: 0 0 auto;
  justify-content: flex-end;
}

.brand-section {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-avatar {
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.55), rgba(var(--accent-2-rgb), 0.35));
  border: 1px solid rgba(var(--accent-rgb), 0.4);
  box-shadow: 0 10px 18px rgba(var(--accent-rgb), 0.22);
}

.brand-text {
  display: flex;
  flex-direction: column;
}

.brand-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--el-text-color-primary);
}

.brand-subtitle {
  margin: 0;
  font-size: 0.7rem;
  color: var(--el-text-color-secondary);
}

.workspace-chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.7rem;
}

.chip-label {
  font-size: 0.6rem;
  letter-spacing: 0.04em;
  color: var(--text-tertiary);
}

.chip-value {
  font-weight: 600;
  color: var(--text-primary);
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.2);
  background: rgba(var(--line-rgb), 0.08);
  color: var(--text-secondary);
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: none;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--status-info);
  box-shadow: 0 0 10px rgba(var(--status-info-rgb), 0.5);
}

.status-text {
  letter-spacing: 0.02em;
}

.status-pill[data-status="RUNNING"] {
  border-color: rgba(var(--status-success-rgb), 0.4);
  background: rgba(var(--status-success-rgb), 0.1);
  color: var(--status-success);
}

.status-pill[data-status="RUNNING"] .status-dot {
  background: var(--status-success);
  box-shadow: 0 0 10px rgba(var(--status-success-rgb), 0.5);
}

.status-pill[data-status="AWAITING_USER"] {
  border-color: rgba(var(--status-warning-rgb), 0.4);
  background: rgba(var(--status-warning-rgb), 0.1);
  color: var(--status-warning);
}

.status-pill[data-status="AWAITING_USER"] .status-dot {
  background: var(--status-warning);
  box-shadow: 0 0 10px rgba(var(--status-warning-rgb), 0.5);
}

.status-pill[data-status="ERROR"] {
  border-color: rgba(var(--status-error-rgb), 0.4);
  background: rgba(var(--status-error-rgb), 0.12);
  color: var(--status-error);
}

.status-pill[data-status="ERROR"] .status-dot {
  background: var(--status-error);
  box-shadow: 0 0 10px rgba(var(--status-error-rgb), 0.5);
}

.status-pill[data-status="PAUSED"] {
  border-color: rgba(var(--line-rgb), 0.28);
  background: rgba(var(--line-rgb), 0.14);
  color: var(--text-secondary);
}

.meta-inline {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: rgba(var(--line-rgb), 0.06);
  font-size: 0.68rem;
  color: var(--text-tertiary);
}

.meta-inline strong {
  color: var(--text-primary);
  font-weight: 600;
}

.header-right :deep(.el-button) {
  border-radius: 12px;
  background: rgba(var(--line-rgb), 0.1);
  border: 1px solid rgba(var(--line-rgb), 0.2);
  color: var(--text-primary);
}

.header-right :deep(.el-button:hover) {
  border-color: rgba(var(--accent-rgb), 0.35);
  color: var(--text-primary);
}

.header-right :deep(.el-button--primary) {
  background: rgba(var(--accent-rgb), 0.18);
  border-color: rgba(var(--accent-rgb), 0.35);
  box-shadow: 0 10px 18px rgba(var(--accent-rgb), 0.2);
}

.main-container {
  height: calc(100vh - 68px);
}

.app-sidebar {
  background: var(--el-bg-color-page);
  border-right: 1px solid rgba(var(--line-rgb), 0.16);
  transition: width 0.3s;
}

.sidebar-menu {
  border: none;
  height: 100%;
  background: transparent;
}

.sidebar-menu :deep(.el-menu-item) {
  margin: 6px 8px;
  border-radius: 12px;
  height: 44px;
  line-height: 44px;
  color: var(--text-tertiary);
  transition: all 0.2s ease;
}

.sidebar-menu :deep(.el-menu-item:hover) {
  background: rgba(var(--line-rgb), 0.14);
  color: var(--text-primary);
}

.sidebar-menu :deep(.el-menu-item.is-active) {
  background: rgba(var(--accent-rgb), 0.18);
  color: var(--text-primary);
  box-shadow: 0 10px 18px rgba(var(--accent-rgb), 0.14);
}

.app-main {
  background-color: var(--el-bg-color);
  padding: 0;
}

.main-content {
  padding: 20px;
  height: 100%;
  overflow-y: auto;
}

.right-panel {
  background-color: var(--el-bg-color-page);
  border-left: 1px solid rgba(var(--line-rgb), 0.16);
  padding: 20px 18px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.status-card,
.tools-card {
  margin-bottom: 0;
  border-radius: 16px;
  border: 1px solid rgba(var(--line-rgb), 0.18);
  background: var(--surface, rgba(20, 24, 30, 0.75));
  box-shadow: 0 12px 24px rgba(5, 10, 18, 0.2);
}

.status-card :deep(.el-card__body),
.tools-card :deep(.el-card__body) {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.status-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.card-eyebrow {
  margin: 0 0 4px;
  font-size: 0.65rem;
  color: var(--text-tertiary);
  letter-spacing: 0.04em;
}

.card-title {
  margin: 0;
  font-size: 1rem;
  color: var(--text-primary);
  font-weight: 600;
}

.status-grid {
  display: grid;
  gap: 8px;
}

.status-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.16);
  background: rgba(var(--line-rgb), 0.08);
  font-size: 0.72rem;
  color: var(--text-secondary);
}

.status-row strong {
  color: var(--text-primary);
  font-weight: 600;
}

.status-progress {
  margin-top: 4px;
}

.status-progress :deep(.el-progress-bar__outer) {
  background: rgba(var(--line-rgb), 0.18);
}

.status-progress :deep(.el-progress-bar__inner) {
  background: linear-gradient(90deg, rgba(var(--accent-rgb), 0.75), rgba(var(--accent-2-rgb), 0.75));
}

.action-buttons {
  display: grid;
  gap: 10px;
}

.action-buttons :deep(.el-button) {
  justify-content: center;
  font-weight: 600;
}

.active-tool {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--line-rgb), 0.16);
  background: rgba(var(--line-rgb), 0.08);
}

.no-tool {
  text-align: center;
  padding: 14px 0;
  color: var(--text-tertiary);
}

.tool-stats {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.tool-stat {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  border: 1px solid rgba(var(--line-rgb), 0.14);
  background: rgba(var(--line-rgb), 0.06);
  font-size: 0.7rem;
  color: var(--text-secondary);
}

.tool-stat strong {
  color: var(--text-primary);
  font-weight: 600;
}

</style>
