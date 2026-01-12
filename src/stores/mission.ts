import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";

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

const state = reactive({
  active: null as TaskConfig | null,
  loading: false,
  error: "",
});

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
  try {
    const result = (await invoke("task_get_active")) as TaskConfig | null;
    state.active = result ?? defaultTaskConfig();
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
    state.active = defaultTaskConfig();
  } finally {
    state.loading = false;
  }
}

async function saveActive(config: TaskConfig) {
  state.loading = true;
  state.error = "";
  try {
    const result = (await invoke("task_save_config", { request: config })) as TaskConfig;
    state.active = result;
    return result;
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
    throw error;
  } finally {
    state.loading = false;
  }
}

export const missionStore = {
  state,
  loadActive,
  saveActive,
  defaultTaskConfig,
};
