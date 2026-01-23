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
      </div>

      <div class="header-center">
        <el-tag type="success" effect="dark">Online</el-tag>
        <el-text type="info" size="small">{{ workspaceName }}</el-text>
      </div>

      <div class="header-right">
        <el-button type="primary" :icon="Search" circle />
        <ThemeToggle />
        <el-button :icon="Setting" circle />
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
          background-color="#1a1a1a"
          text-color="#a3a3a3"
          active-text-color="#409eff"
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
      <el-aside v-if="showRightPanel" :width="280" class="right-panel">
        <el-card class="status-card" shadow="never">
          <template #header>
            <div class="card-header">
              <span>Agent Status</span>
              <el-tag :type="getStatusType(agentState)" size="small">
                {{ agentState }}
              </el-tag>
            </div>
          </template>
          
          <el-descriptions :column="1" size="small">
            <el-descriptions-item label="Workspace">
              <el-text type="info">{{ workspaceName }}</el-text>
            </el-descriptions-item>
            <el-descriptions-item label="Run ID">
              <el-text type="info">{{ shortRunId }}</el-text>
            </el-descriptions-item>
            <el-descriptions-item label="Budget">
              <el-progress 
                :percentage="budgetPercent" 
                :status="budgetPercent > 80 ? 'exception' : 'success'"
                :show-text="false"
                :stroke-width="4"
              />
              <el-text type="info" size="small">{{ budgetLabel }}</el-text>
            </el-descriptions-item>
            <el-descriptions-item label="Power">
              <el-text type="primary">{{ powerPercent }}%</el-text>
            </el-descriptions-item>
          </el-descriptions>

          <div class="action-buttons">
            <el-button type="primary" @click="handleContinue" :loading="isLoading">
              <el-icon><VideoPlay /></el-icon>
              Continue
            </el-button>
            <el-button @click="handlePause">
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
          <template #header>
            <span>Tool Status</span>
          </template>
          
          <div v-if="activeTool" class="active-tool">
            <el-tag type="primary">{{ activeTool.tool }}</el-tag>
            <el-text type="info" size="small">{{ activeTool.detail }}</el-text>
          </div>
          <div v-else class="no-tool">
            <el-text type="info">No active tools</el-text>
          </div>

          <el-divider />

          <el-statistics :columns="2" size="small">
            <el-statistic title="Total" :value="toolStats.total" />
            <el-statistic title="Success" :value="toolStats.ok" />
            <el-statistic title="Running" :value="toolStats.running" />
            <el-statistic title="Errors" :value="toolStats.error" />
          </el-statistics>
        </el-card>
      </el-aside>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { agentStore } from '../agents/orchestrator'
import ThemeToggle from '../components/ThemeToggle-Simple.vue'
import {
  Lightning,
  Search,
  Setting,
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
const { state, initKernelStore, userInput, stop } = agentStore

// State
const isCollapse = ref(false)
const showRightPanel = ref(true)
const isLoading = ref(false)

// Computed
const run = computed(() => state.run)
const agentState = computed(() => run.value?.agentState ?? "IDLE")
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

// Menu items
const menuItems = [
  { id: 'console', label: 'Console', icon: Monitor },
  { id: 'mission', label: 'Mission', icon: Document },
  { id: 'plan', label: 'Plan', icon: Connection },
  { id: 'loop', label: 'Loop', icon: Lightning },
  { id: 'terminal', label: 'Terminal', icon: Platform },
  { id: 'diff', label: 'Diff', icon: Files },
  { id: 'git', label: 'Git', icon: Link },
  { id: 'timeline', label: 'Timeline', icon: Clock }
]

// Methods
const navigateTo = (id: string) => {
  router.push({ name: id })
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
  isLoading.value = true
  try {
    // Continue logic here
    await new Promise(resolve => setTimeout(resolve, 1000))
  } finally {
    isLoading.value = false
  }
}

const handlePause = () => {
  // Pause logic here
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
}

.app-header {
  background-color: var(--el-bg-color-page);
  border-bottom: 1px solid var(--el-border-color);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  height: 60px;
}

.header-left,
.header-center,
.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.brand-section {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-avatar {
  background: linear-gradient(135deg, #409eff, #67c23a);
}

.brand-text {
  display: flex;
  flex-direction: column;
}

.brand-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.brand-subtitle {
  margin: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.main-container {
  height: calc(100vh - 60px);
}

.app-sidebar {
  background-color: var(--el-bg-color-page);
  border-right: 1px solid var(--el-border-color);
  transition: width 0.3s;
}

.sidebar-menu {
  border: none;
  height: 100%;
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
  border-left: 1px solid var(--el-border-color);
  padding: 20px;
  overflow-y: auto;
}

.status-card,
.tools-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}

.action-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 16px;
}

.active-tool {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.no-tool {
  text-align: center;
  padding: 20px 0;
}

/* Dark theme adjustments */
:root.dark {
  --el-bg-color: #141414;
  --el-bg-color-page: #0a0a0a;
  --el-bg-color-overlay: #1d1e1f;
  --el-text-color-primary: #e5eaf3;
  --el-text-color-regular: #cfd3dc;
  --el-text-color-secondary: #a3a6ad;
  --el-text-color-placeholder: #8d9095;
  --el-text-color-disabled: #6c6e72;
  --el-border-color: #4c4d4f;
  --el-border-color-light: #414243;
  --el-border-color-lighter: #363637;
  --el-border-color-extra-light: #2b2b2c;
  --el-border-color-dark: #58585b;
  --el-border-color-darker: #636466;
  --el-fill-color: #47494c;
  --el-fill-color-light: #36383a;
  --el-fill-color-lighter: #2a2b2d;
  --el-fill-color-extra-light: #1f1f20;
  --el-fill-color-dark: #545459;
  --el-fill-color-darker: #616166;
  --el-fill-color-blank: transparent;
}
</style>
