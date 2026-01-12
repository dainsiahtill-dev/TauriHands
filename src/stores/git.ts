import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";

export function useGitStore() {
  const runId = computed(() => agentStore.state.run?.runId ?? "");
  return { runId };
}
