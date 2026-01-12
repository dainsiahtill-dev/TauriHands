import { computed } from "vue";
import { agentStore } from "../agents/orchestrator";

export function useTimelineStore() {
  const events = computed(() => agentStore.state.events ?? []);
  return { events };
}
