<template>
  <div class="app-container">
    <!-- Header -->
    <header class="app-header">
      <div class="app-header__section app-header__left">
        <div class="flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg bg-primary-500 flex items-center justify-center">
            <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <div class="min-w-0">
            <h1 class="text-lg font-semibold text-primary truncate">TauriHands</h1>
            <p class="text-xs text-tertiary truncate">AI Development Agent</p>
          </div>
        </div>
      </div>

      <div class="app-header__section app-header__center">
        <div class="flex items-center gap-4">
          <div class="badge badge--success">Online</div>
          <div class="text-sm text-secondary">{{ workspaceName }}</div>
        </div>
      </div>

      <div class="app-header__section app-header__right">
        <div class="flex items-center gap-3">
          <button class="btn btn--ghost btn--sm">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </button>
          <ThemeToggle />
          <button class="btn btn--ghost btn--sm">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </div>
    </header>

    <!-- Body -->
    <div class="app-body">
      <!-- Primary Sidebar -->
      <aside class="app-sidebar app-sidebar--primary">
        <nav class="nav-vertical">
          <router-link 
            v-for="activity in activities" 
            :key="activity.id"
            :to="activity.to"
            class="nav-item"
            :class="{ 'nav-item--active': currentView === activity.id }"
          >
            <component :is="activity.icon" class="w-5 h-5" />
            <span>{{ activity.label }}</span>
          </router-link>
        </nav>
      </aside>

      <!-- Secondary Sidebar -->
      <aside 
        class="app-sidebar app-sidebar--secondary"
        :class="{ 'app-sidebar--collapsed': !showSecondarySidebar }"
      >
        <div class="p-4 border-b">
          <h2 class="text-lg font-semibold mb-4">Agent Status</h2>
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-sm text-secondary">Status</span>
              <div class="badge badge--success">Running</div>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-secondary">Power</span>
              <span class="text-sm font-medium">{{ powerPercent }}%</span>
            </div>
          </div>
        </div>
        
        <div class="flex-1 overflow-y-auto p-4">
          <div class="space-y-4">
            <div>
              <h3 class="text-sm font-medium mb-2">Quick Actions</h3>
              <div class="space-y-2">
                <button class="btn btn--primary w-full">Continue</button>
                <button class="btn btn--secondary w-full">Pause</button>
                <button class="btn btn--ghost w-full">Stop</button>
              </div>
            </div>
          </div>
        </div>
      </aside>

      <!-- Main Content -->
      <main class="app-main">
        <div class="app-content">
          <router-view />
        </div>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import ThemeToggle from "../components/ThemeToggle-Simple.vue";

// Icons
const IconMission = ref('svg') // Replace with actual icon component
const IconPlan = ref('svg') // Replace with actual icon component
const IconLoop = ref('svg') // Replace with actual icon component
const IconTerminal = ref('svg') // Replace with actual icon component
const IconDiff = ref('svg') // Replace with actual icon component
const IconGit = ref('svg') // Replace with actual icon component
const IconTimeline = ref('svg') // Replace with actual icon component

const router = useRouter()
const currentView = ref('mission')
const showSecondarySidebar = ref(true)
const workspaceName = ref('Default Workspace')
const powerPercent = ref(85)

const activities = [
  {
    id: 'mission',
    label: 'Mission',
    to: '/mission',
    icon: IconMission
  },
  {
    id: 'plan',
    label: 'Plan',
    to: '/plan',
    icon: IconPlan
  },
  {
    id: 'loop',
    label: 'Loop',
    to: '/loop',
    icon: IconLoop
  },
  {
    id: 'terminal',
    label: 'Terminal',
    to: '/terminal',
    icon: IconTerminal
  },
  {
    id: 'diff',
    label: 'Diff',
    to: '/diff',
    icon: IconDiff
  },
  {
    id: 'git',
    label: 'Git',
    to: '/git',
    icon: IconGit
  },
  {
    id: 'timeline',
    label: 'Timeline',
    to: '/timeline',
    icon: IconTimeline
  }
]

// Watch route changes
router.afterEach((to) => {
  currentView.value = to.name as string || 'mission'
})
</script>
