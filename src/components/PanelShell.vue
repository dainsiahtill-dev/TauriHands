<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
  icon?: string;
  noPadding?: boolean;
}>();
</script>

<template>
  <section class="panel-shell flex h-full w-full min-h-0 flex-col">
    <header class="panel-shell__header">
      <div class="panel-shell__title">
        <p v-if="subtitle" class="panel-shell__subtitle">{{ subtitle }}</p>
        <div class="panel-shell__main">
          <span v-if="icon" class="panel-shell__icon">{{ icon }}</span>
          <span>{{ title }}</span>
        </div>
      </div>
      <div v-if="$slots.actions" class="panel-shell__actions">
        <slot name="actions"></slot>
      </div>
    </header>

    <div class="panel-shell__body" :class="noPadding ? 'is-flush' : ''">
      <slot></slot>
    </div>
  </section>
</template>
