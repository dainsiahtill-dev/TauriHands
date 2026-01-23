import { reactive } from "vue";

type TerminalState = {
  activeSessionId: string | null;
};

const state = reactive<TerminalState>({
  activeSessionId: null,
});

const listeners = new Set<(next: TerminalState) => void>();

function notify() {
  for (const listener of listeners) {
    listener(state);
  }
}

function subscribe(listener: (next: TerminalState) => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function setActiveSessionId(sessionId: string | null) {
  state.activeSessionId = sessionId;
  notify();
}

export const terminalStore = {
  state,
  subscribe,
  setActiveSessionId,
};
