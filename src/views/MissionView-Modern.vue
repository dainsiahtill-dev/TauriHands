<template>
  <div class="space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold text-primary">Mission Control</h1>
        <p class="text-secondary mt-1">Manage your AI agent missions and tasks</p>
      </div>
      <div class="flex gap-3">
        <button class="btn btn--primary">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
          New Mission
        </button>
      </div>
    </div>

    <!-- Stats Grid -->
    <div class="grid grid--cols-4">
      <div class="card">
        <div class="card__body">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-tertiary">Active Missions</p>
              <p class="text-2xl font-bold text-primary mt-1">3</p>
            </div>
            <div class="w-10 h-10 rounded-lg bg-primary-100 flex items-center justify-center">
              <svg class="w-5 h-5 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card__body">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-tertiary">Completed Today</p>
              <p class="text-2xl font-bold text-success mt-1">12</p>
            </div>
            <div class="w-10 h-10 rounded-lg bg-success-100 flex items-center justify-center">
              <svg class="w-5 h-5 text-success-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card__body">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-tertiary">Success Rate</p>
              <p class="text-2xl font-bold text-warning mt-1">87%</p>
            </div>
            <div class="w-10 h-10 rounded-lg bg-warning-100 flex items-center justify-center">
              <svg class="w-5 h-5 text-warning-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card__body">
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-tertiary">Total Tasks</p>
              <p class="text-2xl font-bold text-info mt-1">248</p>
            </div>
            <div class="w-10 h-10 rounded-lg bg-info-100 flex items-center justify-center">
              <svg class="w-5 h-5 text-info-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
              </svg>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Active Missions -->
    <div class="card">
      <div class="card__header">
        <h2 class="text-lg font-semibold">Active Missions</h2>
      </div>
      <div class="card__body">
        <div class="space-y-4">
          <div v-for="mission in activeMissions" :key="mission.id" class="flex items-center justify-between p-4 border rounded-lg hover:bg-surface transition-colors">
            <div class="flex items-center gap-4">
              <div class="w-10 h-10 rounded-lg bg-primary-100 flex items-center justify-center">
                <svg class="w-5 h-5 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                </svg>
              </div>
              <div>
                <h3 class="font-medium text-primary">{{ mission.title }}</h3>
                <p class="text-sm text-tertiary">{{ mission.description }}</p>
              </div>
            </div>
            <div class="flex items-center gap-3">
              <div class="badge" :class="`badge--${mission.status}`">{{ mission.status }}</div>
              <button class="btn btn--ghost btn--sm">View</button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Recent Activity -->
    <div class="card">
      <div class="card__header">
        <h2 class="text-lg font-semibold">Recent Activity</h2>
      </div>
      <div class="card__body">
        <div class="space-y-3">
          <div v-for="activity in recentActivity" :key="activity.id" class="flex items-start gap-3">
            <div class="w-2 h-2 rounded-full mt-2" :class="`bg-${activity.type}`"></div>
            <div class="flex-1">
              <p class="text-sm text-primary">{{ activity.message }}</p>
              <p class="text-xs text-tertiary mt-1">{{ activity.time }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeMissions = ref([
  {
    id: 1,
    title: 'Code Review Assistant',
    description: 'Automated code review and suggestions',
    status: 'success'
  },
  {
    id: 2,
    title: 'Documentation Generator',
    description: 'Generate comprehensive documentation',
    status: 'warning'
  },
  {
    id: 3,
    title: 'Test Suite Builder',
    description: 'Create comprehensive test suites',
    status: 'primary'
  }
])

const recentActivity = ref([
  {
    id: 1,
    message: 'Completed code review for feature/authentication',
    time: '2 minutes ago',
    type: 'success'
  },
  {
    id: 2,
    message: 'Started new mission: API Documentation',
    time: '5 minutes ago',
    type: 'primary'
  },
  {
    id: 3,
    message: 'Updated configuration settings',
    time: '10 minutes ago',
    type: 'info'
  },
  {
    id: 4,
    message: 'Fixed issue in terminal module',
    time: '15 minutes ago',
    type: 'success'
  }
])
</script>
