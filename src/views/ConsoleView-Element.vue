<template>
  <div class="console-container">
    <!-- Mission Overview -->
    <el-row :gutter="20" class="overview-row">
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <el-statistic title="Active Missions" :value="3" />
          <template #footer>
            <div class="stat-footer">
              <span>Running</span>
              <el-tag type="success" size="small">Online</el-tag>
            </div>
          </template>
        </el-card>
      </el-col>
      
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <el-statistic title="Completed Today" :value="12" />
          <template #footer>
            <div class="stat-footer">
              <span>Success Rate</span>
              <el-tag type="success" size="small">87%</el-tag>
            </div>
          </template>
        </el-card>
      </el-col>
      
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <el-statistic title="Budget Used" :value="budgetPercent" suffix="%" />
          <template #footer>
            <el-progress 
              :percentage="budgetPercent" 
              :status="budgetPercent > 80 ? 'exception' : 'success'"
              :show-text="false"
              :stroke-width="4"
            />
          </template>
        </el-card>
      </el-col>
      
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <el-statistic title="Total Tasks" :value="248" />
          <template #footer>
            <div class="stat-footer">
              <span>Power</span>
              <el-tag type="primary" size="small">{{ powerPercent }}%</el-tag>
            </div>
          </template>
        </el-card>
      </el-col>
    </el-row>

    <!-- Main Content Area -->
    <el-row :gutter="20" class="main-row">
      <!-- Left Panel - Mission Control -->
      <el-col :span="8">
        <el-card class="mission-card" shadow="never">
          <template #header>
            <div class="card-header-content">
              <span>Mission Control</span>
              <el-button type="primary" size="small" @click="showCreateDialog = true">
                <el-icon><Plus /></el-icon>
                New Mission
              </el-button>
            </div>
          </template>
          
          <div class="mission-list">
            <div 
              v-for="mission in missions" 
              :key="mission.id"
              class="mission-item"
              :class="{ 'is-active': selectedMission === mission.id }"
              @click="selectedMission = mission.id"
            >
              <div class="mission-header">
                <el-avatar :size="32" class="mission-avatar">
                  <el-icon><Lightning /></el-icon>
                </el-avatar>
                <div class="mission-info">
                  <h4>{{ mission.title }}</h4>
                  <p>{{ mission.description }}</p>
                </div>
              </div>
              <div class="mission-status">
                <el-tag :type="getStatusType(mission.status)" size="small">
                  {{ mission.status }}
                </el-tag>
                <el-button type="text" size="small">View</el-button>
              </div>
            </div>
          </div>
        </el-card>
      </el-col>

      <!-- Center Panel - Main Content -->
      <el-col :span="8">
        <el-card class="content-card" shadow="never">
          <template #header>
            <el-tabs v-model="activeTab" class="content-tabs">
              <el-tab-pane label="Overview" name="overview" />
              <el-tab-pane label="Plan" name="plan" />
              <el-tab-pane label="Tasks" name="tasks" />
              <el-tab-pane label="Tools" name="tools" />
              <el-tab-pane label="Logs" name="logs" />
            </el-tabs>
          </template>
          
          <div class="tab-content">
            <!-- Overview Tab -->
            <div v-if="activeTab === 'overview'" class="overview-content">
              <el-descriptions :column="2" border>
                <el-descriptions-item label="Mission ID">
                  <el-text type="info">{{ currentMission?.id }}</el-text>
                </el-descriptions-item>
                <el-descriptions-item label="Status">
                  <el-tag :type="getStatusType(currentMission?.status)">
                    {{ currentMission?.status }}
                  </el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="Created">
                  <el-text>{{ currentMission?.created }}</el-text>
                </el-descriptions-item>
                <el-descriptions-item label="Progress">
                  <el-progress :percentage="currentMission?.progress || 0" />
                </el-descriptions-item>
              </el-descriptions>
              
              <el-divider />
              
              <h3>Mission Details</h3>
              <el-text>{{ currentMission?.description }}</el-text>
            </div>
            
            <!-- Plan Tab -->
            <div v-if="activeTab === 'plan'" class="plan-content">
              <el-steps :active="currentStep" direction="vertical" finish-status="success">
                <el-step 
                  v-for="step in planSteps" 
                  :key="step.id"
                  :title="step.title"
                  :description="step.description"
                />
              </el-steps>
            </div>
            
            <!-- Tasks Tab -->
            <div v-if="activeTab === 'tasks'" class="tasks-content">
              <el-table :data="tasks" stripe>
                <el-table-column prop="id" label="ID" width="80" />
                <el-table-column prop="title" label="Task" />
                <el-table-column prop="status" label="Status" width="100">
                  <template #default="scope">
                    <el-tag :type="getStatusType(scope.row.status)" size="small">
                      {{ scope.row.status }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="progress" label="Progress" width="120">
                  <template #default="scope">
                    <el-progress :percentage="scope.row.progress" :show-text="false" :stroke-width="4" />
                  </template>
                </el-table-column>
              </el-table>
            </div>
            
            <!-- Tools Tab -->
            <div v-if="activeTab === 'tools'" class="tools-content">
              <el-timeline>
                <el-timeline-item 
                  v-for="tool in toolHistory" 
                  :key="tool.id"
                  :timestamp="tool.timestamp"
                  :type="tool.type"
                >
                  <el-card>
                    <h4>{{ tool.name }}</h4>
                    <p>{{ tool.description }}</p>
                    <el-tag :type="getStatusType(tool.status)" size="small">
                      {{ tool.status }}
                    </el-tag>
                  </el-card>
                </el-timeline-item>
              </el-timeline>
            </div>
            
            <!-- Logs Tab -->
            <div v-if="activeTab === 'logs'" class="logs-content">
              <el-input
                v-model="logFilter"
                placeholder="Filter logs..."
                :prefix-icon="Search"
                clearable
                class="log-filter"
              />
              <div class="log-container">
                <div 
                  v-for="log in filteredLogs" 
                  :key="log.id"
                  class="log-entry"
                  :class="`log-${log.level.toLowerCase()}`"
                >
                  <span class="log-time">{{ log.time }}</span>
                  <span class="log-level">{{ log.level }}</span>
                  <span class="log-message">{{ log.message }}</span>
                </div>
              </div>
            </div>
          </div>
        </el-card>
      </el-col>

      <!-- Right Panel - Activity -->
      <el-col :span="8">
        <el-card class="activity-card" shadow="never">
          <template #header>
            <span>Recent Activity</span>
          </template>
          
          <el-timeline>
            <el-timeline-item 
              v-for="activity in recentActivity" 
              :key="activity.id"
              :timestamp="activity.time"
              :type="activity.type"
              size="small"
            >
              <p>{{ activity.message }}</p>
            </el-timeline-item>
          </el-timeline>
        </el-card>
        
        <el-card class="quick-actions-card" shadow="never">
          <template #header>
            <span>Quick Actions</span>
          </template>
          
          <div class="quick-actions">
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
      </el-col>
    </el-row>

    <!-- Create Mission Dialog -->
    <el-dialog 
      v-model="showCreateDialog" 
      title="Create New Mission" 
      width="500px"
    >
      <el-form :model="newMission" label-width="80px">
        <el-form-item label="Title">
          <el-input v-model="newMission.title" placeholder="Mission title" />
        </el-form-item>
        <el-form-item label="Description">
          <el-input 
            v-model="newMission.description" 
            type="textarea" 
            :rows="3"
            placeholder="Mission description"
          />
        </el-form-item>
        <el-form-item label="Type">
          <el-select v-model="newMission.type" placeholder="Select type">
            <el-option label="Development" value="development" />
            <el-option label="Testing" value="testing" />
            <el-option label="Documentation" value="documentation" />
            <el-option label="Maintenance" value="maintenance" />
          </el-select>
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="showCreateDialog = false">Cancel</el-button>
        <el-button type="primary" @click="createMission">Create</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Plus,
  Lightning,
  Search,
  VideoPlay,
  VideoPause,
  CircleCloseFilled
} from '@element-plus/icons-vue'

// State
const activeTab = ref('overview')
const selectedMission = ref(1)
const showCreateDialog = ref(false)
const isLoading = ref(false)
const logFilter = ref('')
const currentStep = ref(1)

const powerPercent = ref(85)
const budgetPercent = ref(65)

// Data
const missions = ref([
  {
    id: 1,
    title: 'Code Review Assistant',
    description: 'Automated code review and suggestions',
    status: 'running',
    progress: 75,
    created: '2024-01-22 10:30'
  },
  {
    id: 2,
    title: 'Documentation Generator',
    description: 'Generate comprehensive documentation',
    status: 'pending',
    progress: 30,
    created: '2024-01-22 11:15'
  },
  {
    id: 3,
    title: 'Test Suite Builder',
    description: 'Create comprehensive test suites',
    status: 'completed',
    progress: 100,
    created: '2024-01-22 09:45'
  }
])

const currentMission = computed(() => 
  missions.value.find(m => m.id === selectedMission.value)
)

const planSteps = ref([
  { id: 1, title: 'Analysis', description: 'Analyze requirements and existing code' },
  { id: 2, title: 'Planning', description: 'Create detailed implementation plan' },
  { id: 3, title: 'Implementation', description: 'Implement the solution' },
  { id: 4, title: 'Testing', description: 'Test and validate the implementation' },
  { id: 5, title: 'Deployment', description: 'Deploy and monitor' }
])

const tasks = ref([
  { id: 1, title: 'Setup environment', status: 'completed', progress: 100 },
  { id: 2, title: 'Analyze codebase', status: 'completed', progress: 100 },
  { id: 3, title: 'Generate suggestions', status: 'running', progress: 60 },
  { id: 4, title: 'Review results', status: 'pending', progress: 0 }
])

const toolHistory = ref([
  { id: 1, name: 'Code Analysis', description: 'Analyzed 15 files', status: 'completed', timestamp: '10:30', type: 'success' },
  { id: 2, name: 'Documentation', description: 'Generated API docs', status: 'completed', timestamp: '10:45', type: 'success' },
  { id: 3, name: 'Testing', description: 'Running unit tests', status: 'running', timestamp: '11:00', type: 'primary' }
])

const recentActivity = ref([
  { id: 1, message: 'Completed code review for feature/authentication', time: '2 minutes ago', type: 'success' },
  { id: 2, message: 'Started new mission: API Documentation', time: '5 minutes ago', type: 'primary' },
  { id: 3, message: 'Updated configuration settings', time: '10 minutes ago', type: 'info' },
  { id: 4, message: 'Fixed issue in terminal module', time: '15 minutes ago', type: 'success' }
])

const logs = ref([
  { id: 1, time: '11:30:45', level: 'INFO', message: 'Starting mission execution' },
  { id: 2, time: '11:30:46', level: 'DEBUG', message: 'Loading workspace configuration' },
  { id: 3, time: '11:30:47', level: 'INFO', message: 'Workspace loaded: /project/src' },
  { id: 4, time: '11:30:48', level: 'WARN', message: 'Found 3 potential issues' },
  { id: 5, time: '11:30:49', level: 'ERROR', message: 'Failed to load config file' },
  { id: 6, time: '11:30:50', level: 'INFO', message: 'Using default configuration' }
])

const newMission = ref({
  title: '',
  description: '',
  type: ''
})

// Computed
const filteredLogs = computed(() => {
  if (!logFilter.value) return logs.value
  return logs.value.filter(log => 
    log.message.toLowerCase().includes(logFilter.value.toLowerCase()) ||
    log.level.toLowerCase().includes(logFilter.value.toLowerCase())
  )
})

// Methods
const getStatusType = (status: string) => {
  switch (status) {
    case 'running':
    case 'completed':
    case 'success':
      return 'success'
    case 'pending':
    case 'warning':
      return 'warning'
    case 'error':
    case 'failed':
      return 'danger'
    default:
      return 'info'
  }
}

const createMission = () => {
  if (!newMission.value.title || !newMission.value.description) {
    ElMessage.warning('Please fill in all required fields')
    return
  }
  
  const mission = {
    id: missions.value.length + 1,
    title: newMission.value.title,
    description: newMission.value.description,
    status: 'pending',
    progress: 0,
    created: new Date().toLocaleString()
  }
  
  missions.value.unshift(mission)
  showCreateDialog.value = false
  newMission.value = { title: '', description: '', type: '' }
  
  ElMessage.success('Mission created successfully')
}

const handleContinue = async () => {
  isLoading.value = true
  try {
    await new Promise(resolve => setTimeout(resolve, 2000))
    ElMessage.success('Mission continued')
  } finally {
    isLoading.value = false
  }
}

const handlePause = () => {
  ElMessage.info('Mission paused')
}

const handleStop = () => {
  ElMessage.warning('Mission stopped')
}

onMounted(() => {
  // Initialize data
})
</script>

<style scoped>
.console-container {
  padding: 20px;
  height: 100%;
  overflow-y: auto;
}

.overview-row {
  margin-bottom: 20px;
}

.stat-card {
  height: 120px;
}

.stat-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.main-row {
  height: calc(100vh - 200px);
}

.mission-card,
.content-card,
.activity-card,
.quick-actions-card {
  height: 100%;
}

.card-header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.mission-list {
  max-height: 400px;
  overflow-y: auto;
}

.mission-item {
  padding: 12px;
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  margin-bottom: 8px;
  cursor: pointer;
  transition: all 0.3s;
}

.mission-item:hover {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}

.mission-item.is-active {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-8);
}

.mission-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.mission-avatar {
  background: var(--el-color-primary);
}

.mission-info h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.mission-info p {
  margin: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.mission-status {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.content-tabs {
  border-bottom: none;
}

.tab-content {
  padding: 20px 0;
}

.overview-content h3 {
  margin: 16px 0 8px 0;
  font-size: 16px;
  font-weight: 600;
}

.plan-content,
.tasks-content,
.tools-content,
.logs-content {
  height: 100%;
}

.log-filter {
  margin-bottom: 16px;
}

.log-container {
  height: 300px;
  overflow-y: auto;
  background-color: var(--el-bg-color-page);
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  padding: 12px;
  font-family: monospace;
  font-size: 12px;
}

.log-entry {
  display: flex;
  gap: 12px;
  margin-bottom: 4px;
  padding: 2px 0;
}

.log-time {
  color: var(--el-text-color-secondary);
  min-width: 80px;
}

.log-level {
  min-width: 60px;
  font-weight: 600;
}

.log-info .log-level {
  color: var(--el-color-info);
}

.log-warn .log-level {
  color: var(--el-color-warning);
}

.log-error .log-level {
  color: var(--el-color-danger);
}

.log-message {
  flex: 1;
}

.quick-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.activity-card {
  margin-bottom: 16px;
}
</style>
