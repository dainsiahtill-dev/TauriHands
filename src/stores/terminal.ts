import { ref } from "vue";

const activeSessionId = ref<string | null>(null);

function setActiveSessionId(sessionId: string | null) {
  activeSessionId.value = sessionId;
}

export const terminalStore = {
  activeSessionId,
  setActiveSessionId,
};
