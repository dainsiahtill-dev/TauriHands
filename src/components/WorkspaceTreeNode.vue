<script setup lang="ts">
type TreeItem = {
  type: "folder" | "file";
  name: string;
  path: string;
  children?: TreeItem[];
};

defineProps<{
  item: TreeItem;
}>();

defineOptions({
  name: "WorkspaceTreeNode",
});
</script>

<template>
  <li>
    <div class="tree-row">
      <span class="dot" :data-type="item.type"></span>
      <span class="tree-name">{{ item.name }}</span>
    </div>
    <ul v-if="item.children && item.children.length" class="tree-children">
      <WorkspaceTreeNode v-for="child in item.children" :key="child.path" :item="child" />
    </ul>
  </li>
</template>

<style scoped>
.tree-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 12px;
  background: rgba(9, 14, 22, 0.8);
  border: 1px solid var(--line);
  min-width: 0;
}

.tree-row {
  justify-content: flex-start;
}

.tree-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #2df6ff;
  box-shadow: 0 0 10px rgba(45, 246, 255, 0.5);
}

.dot[data-type="folder"] {
  background: #b6ff4b;
  box-shadow: 0 0 12px rgba(182, 255, 75, 0.5);
}

.tree-children {
  list-style: none;
  margin: 8px 0 0 20px;
  padding: 0;
  display: grid;
  gap: 6px;
  color: #8aa0b7;
  font-size: 0.8rem;
}
</style>
