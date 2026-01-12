import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";

export function useLoopStore() {
  const run = computed(() => agentStore.state.run);
  const toolCalls = computed(() => agentStore.state.toolCalls);
  const llmStream = computed(() => agentStore.state.llmStream);
  return { run, toolCalls, llmStream };
}
