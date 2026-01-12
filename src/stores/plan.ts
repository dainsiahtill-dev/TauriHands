import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";

export function usePlanStore() {
  const plan = computed(() => agentStore.state.run?.plan ?? null);
  const steps = computed(() => plan.value?.steps ?? []);
  return { plan, steps };
}
