import { invoke } from "@tauri-apps/api/core";
import { reactive } from "vue";

export type TaskBudget = {
  maxIterations?: number;
  maxToolCalls?: number;
  maxWallTimeMs?: number;
};

export type TaskRiskPolicy = {
  allowNetwork: boolean;
  commandPolicy: string;
  pathPolicy: string;
};

export type TaskConfig = {
  taskId: string;
  workspace: string;
  goal: string;
  completion: string[];
  budget: TaskBudget;
  riskPolicy: TaskRiskPolicy;
  autonomy: string;
};

type MissionState = {
  active: TaskConfig | null;
  loading: boolean;
  error: string;
};

const state = reactive<MissionState>({
  active: null,
  loading: false,
  error: "",
});

const listeners = new Set<(next: MissionState) => void>();

function notify() {
  for (const listener of listeners) {
    listener(state);
  }
}

function subscribe(listener: (next: MissionState) => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function defaultTaskConfig(): TaskConfig {
  return {
    taskId: "",
    workspace: "",
    goal: "",
    completion: ["tests pass", "git clean"],
    budget: { maxIterations: 8, maxToolCalls: 80, maxWallTimeMs: 900000 },
    riskPolicy: {
      allowNetwork: false,
      commandPolicy: "confirm",
      pathPolicy: "workspace_only",
    },
    autonomy: "auto",
  };
}

async function loadActive() {
  state.loading = true;
  state.error = "";
  notify();
  try {
    const result = (await invoke("task_get_active")) as TaskConfig | null;
    state.active = result ?? defaultTaskConfig();
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
    state.active = defaultTaskConfig();
  } finally {
    state.loading = false;
    notify();
  }
}

async function saveActive(config: TaskConfig) {
  state.loading = true;
  state.error = "";
  notify();
  try {
    const result = (await invoke("task_save_config", { request: config })) as TaskConfig;
    state.active = result;
    return result;
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
    throw error;
  } finally {
    state.loading = false;
    notify();
  }
}

export const missionStore = {
  state,
  subscribe,
  loadActive,
  saveActive,
  defaultTaskConfig,
};
