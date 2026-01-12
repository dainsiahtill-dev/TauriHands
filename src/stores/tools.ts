import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";

export function useToolsStore() {
  const toolCalls = computed(() => agentStore.state.toolCalls ?? []);
  const toolOutputs = computed(() => agentStore.state.toolOutputs ?? {});
  return { toolCalls, toolOutputs };
}
