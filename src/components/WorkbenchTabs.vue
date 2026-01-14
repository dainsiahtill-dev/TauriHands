<script setup lang="ts">
type Tab = {
  id: string;
  label: string;
};

const props = defineProps<{
  tabs: Tab[];
  active: string;
}>();

const emit = defineEmits<{
  (e: "select", id: string): void;
}>();

function selectTab(id: string) {
  emit("select", id);
}
</script>

<template>
  <div class="hud-tabs">
    <button
      v-for="tab in props.tabs"
      :key="tab.id"
      type="button"
      class="hud-tab"
      :class="{ 'is-active': tab.id === props.active }"
      @click="selectTab(tab.id)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>
