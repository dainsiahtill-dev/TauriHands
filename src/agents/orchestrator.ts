import { reactive, readonly } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { terminalStore } from "../stores/terminal";
import { missionStore } from "../stores/mission";

type RunAgentState = "IDLE" | "RUNNING" | "PAUSED" | "AWAITING_USER" | "ERROR" | "FINISHED";

type ChatMessage = {
  role: string;
  content: string;
};

type ToolContext = {
  cwd: string;
  env: Record<string, string>;
  sessionId?: string | null;
};

type Budget = {
  maxSteps: number;
  usedSteps: number;
};

type PlanStep = {
  id: string;
  title: string;
  status: string;
  done: boolean;
};

type Plan = {
  version: number;
  goal: string;
  steps: PlanStep[];
};

type Task = {
  id: string;
  title: string;
  status: string;
  notes?: string | null;
};

type TaskList = {
  version: number;
  items: Task[];
};

type RunState = {
  runId: string;
  agentState: RunAgentState;
  turn: number;
  messages: ChatMessage[];
  toolContext: ToolContext;
  plan?: Plan | null;
  tasks?: TaskList | null;
  budget: Budget;
  autoRun: boolean;
  lastError?: string | null;
  taskId?: string | null;
};

type KernelEvent = {
  id: string;
  runId: string;
  ts: number;
  seq: number;
  type: string;
  payload: Record<string, unknown>;
};

type ToolCallStatus = "running" | "ok" | "error";

type ToolCall = {
  id: string;
  tool: string;
  detail: string;
  status: ToolCallStatus;
  startedAt: number;
  finishedAt?: number;
  exitCode?: number | null;
  summary?: string | null;
};

type LogEntry = {
  id: string;
  level: "info" | "warn" | "error";
  message: string;
  timestamp: number;
};

type ChatToolCall = {
  id: string;
  tool: string;
  detail: string;
  status: ToolCallStatus;
  startedAt: number;
  finishedAt?: number;
  exitCode?: number | null;
  summary?: string | null;
  output?: string;
};

type ChatEntry = {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
  toolCalls: ChatToolCall[];
};

type LlmStream = {
  content: string;
  updatedAt: number;
  active: boolean;
};

type JudgeCheck = {
  id: string;
  type: string;
  status: string;
  reason?: string | null;
  evidence?: string[];
};

type JudgeResult = {
  status: string;
  reasons: string[];
  evidence: string[];
  checks: JudgeCheck[];
};

const state = reactive({
  run: null as RunState | null,
  toolCalls: [] as ToolCall[],
  logs: [] as LogEntry[],
  events: [] as KernelEvent[],
  toolOutputs: {} as Record<string, string>,
  toolCallIndex: {} as Record<string, { entryIndex: number; toolIndex: number }>,
  chatEntries: [] as ChatEntry[],
  llmStream: {
    content: "",
    updatedAt: 0,
    active: false,
  } as LlmStream,
  judgeResult: null as JudgeResult | null,
  timelineFocusId: "" as string,
});

function applyRun(next: RunState) {
  state.run = next;
}

function applyEvent(event: KernelEvent) {
  state.events.unshift(event);
  if (state.events.length > 200) {
    state.events.pop();
  }

  if (event.type === "StateChanged") {
    const payload = event.payload as { state?: RunState; reason?: string };
    if (payload?.reason === "start") {
      state.toolCalls = [];
      state.logs = [];
      state.events = [event];
      state.toolOutputs = {};
      state.llmStream = { content: "", updatedAt: 0, active: false };
      state.toolCallIndex = {};
      state.chatEntries = [];
      state.judgeResult = null;
      state.timelineFocusId = "";
    }
    if (payload?.state) {
      applyRun(payload.state);
    }
    return;
  }

  if (event.type === "UserMessage") {
    const payload = event.payload as { content?: string };
    const content = String(payload.content ?? "").trim();
    if (!content) return;
    state.chatEntries.push({
      id: event.id,
      role: "user",
      content,
      timestamp: event.ts,
      toolCalls: [],
    });
    return;
  }

  if (event.type === "AgentMessage") {
    const payload = event.payload as { content?: string };
    const content = String(payload.content ?? "").trim();
    if (!content) {
      state.llmStream.active = false;
      state.llmStream.content = "";
      return;
    }
    state.chatEntries.push({
      id: event.id,
      role: "assistant",
      content,
      timestamp: event.ts,
      toolCalls: [],
    });
    state.llmStream.active = false;
    state.llmStream.content = "";
    return;
  }

  if (event.type === "ToolCallStarted") {
    const payload = event.payload as { action?: Record<string, unknown> };
    const action = payload.action ?? {};
    const id = String(action.id ?? "");
    if (!id) return;
    state.toolOutputs[id] = "";
    if (!state.chatEntries.length) {
      state.chatEntries.push({
        id: `system-${event.id}`,
        role: "system",
        content: "Tool activity",
        timestamp: event.ts,
        toolCalls: [],
      });
    }
    const entryIndex = state.chatEntries.length - 1;
    const entry = state.chatEntries[entryIndex];
    const toolIndex = entry.toolCalls.length;
    entry.toolCalls.push({
      id,
      tool: String(action.type ?? "tool"),
      detail: describeAction(action),
      status: "running",
      startedAt: event.ts,
      output: "",
    });
    state.toolCallIndex[id] = { entryIndex, toolIndex };
    state.toolCalls.unshift({
      id,
      tool: String(action.type ?? "tool"),
      detail: describeAction(action),
      status: "running",
      startedAt: event.ts,
    });
    return;
  }

  if (event.type === "ToolCallChunk") {
    const payload = event.payload as { action_id?: string; chunk?: string };
    const id = String(payload.action_id ?? "");
    if (!id) return;
    const chunk = String(payload.chunk ?? "");
    if (!chunk) return;
    const current = state.toolOutputs[id] ?? "";
    const next = `${current}${chunk}`;
    const limit = 8000;
    state.toolOutputs[id] = next.length > limit ? next.slice(next.length - limit) : next;
    const index = state.toolCallIndex[id];
    if (index) {
      const entry = state.chatEntries[index.entryIndex];
      const toolEntry = entry?.toolCalls[index.toolIndex];
      if (toolEntry) {
        const existing = toolEntry.output ?? "";
        const output = `${existing}${chunk}`;
        toolEntry.output = output.length > limit ? output.slice(output.length - limit) : output;
      }
    }
    return;
  }

  if (event.type === "AgentMessageChunk") {
    const payload = event.payload as { content?: string };
    const chunk = String(payload.content ?? "");
    if (!chunk) return;
    if (!state.llmStream.active) {
      state.llmStream.content = "";
    }
    const current = state.llmStream.content ?? "";
    const next = `${current}${chunk}`;
    const limit = 8000;
    state.llmStream.content = next.length > limit ? next.slice(next.length - limit) : next;
    state.llmStream.updatedAt = event.ts;
    state.llmStream.active = true;
    return;
  }

  if (event.type === "AgentMessageDone") {
    state.llmStream.active = false;
    state.llmStream.content = "";
    return;
  }

  if (event.type === "ToolCallFinished") {
    const payload = event.payload as {
      action?: Record<string, unknown>;
      ok?: boolean;
      exit_code?: number;
      summary?: string;
    };
    const action = payload.action ?? {};
    const id = String(action.id ?? "");
    if (!id) return;
    const call = state.toolCalls.find((item) => item.id === id);
    if (call) {
      call.status = payload.ok ? "ok" : "error";
      call.exitCode = payload.exit_code ?? null;
      call.summary = payload.summary ?? null;
      call.finishedAt = event.ts;
    }
    const index = state.toolCallIndex[id];
    if (index) {
      const entry = state.chatEntries[index.entryIndex];
      const toolEntry = entry?.toolCalls[index.toolIndex];
      if (toolEntry) {
        toolEntry.status = payload.ok ? "ok" : "error";
        toolEntry.exitCode = payload.exit_code ?? null;
        toolEntry.summary = payload.summary ?? null;
        toolEntry.finishedAt = event.ts;
        if (!toolEntry.output) {
          const output = state.toolOutputs[id];
          if (output) {
            toolEntry.output = output;
          }
        }
      }
    }
    return;
  }

  if (event.type === "Error") {
    const payload = event.payload as { message?: string };
    const message = payload.message ?? "Unknown error";
    state.logs.unshift({
      id: event.id,
      level: "error",
      message,
      timestamp: event.ts,
    });
    return;
  }

  if (event.type === "JudgeResult") {
    const payload = event.payload as { result?: JudgeResult };
    if (payload?.result) {
      state.judgeResult = payload.result;
      state.timelineFocusId = event.id;
    }
  }
}

function describeAction(action: Record<string, unknown>) {
  const type = String(action.type ?? "");
  switch (type) {
    case "terminal.exec":
      return String(action.cmd ?? "");
    case "terminal.run":
      return `${String(action.program ?? "")} ${(action.args as string[] | undefined)?.join(" ") ?? ""}`.trim();
    case "fs.read":
      return String(action.path ?? "");
    case "fs.write":
      return String(action.path ?? "");
    case "fs.search":
      return String(action.pattern ?? "");
    case "git.status":
      return "git status";
    case "git.diff":
      return action.path ? `git diff ${String(action.path)}` : "git diff";
    case "tests.run":
      return `${String(action.program ?? "")} ${(action.args as string[] | undefined)?.join(" ") ?? ""}`.trim();
    default:
      return type;
  }
}

let initialized = false;

async function initKernelStore() {
  if (initialized) return;
  initialized = true;
  try {
    const snapshot = (await invoke("kernel_get_state")) as RunState;
    applyRun(snapshot);
    if (state.chatEntries.length === 0 && snapshot.messages.length) {
      const base = Date.now();
      snapshot.messages.forEach((msg, index) => {
        const role =
          msg.role === "assistant" || msg.role === "system" ? msg.role : "user";
        state.chatEntries.push({
          id: `seed-${base}-${index}`,
          role,
          content: msg.content,
          timestamp: base + index,
          toolCalls: [],
        });
      });
    }
  } catch (error) {
    console.warn("Unable to load kernel state", error);
  }
  try {
    await listen<KernelEvent>("kernel-event", (event) => {
      applyEvent(event.payload);
    });
  } catch (error) {
    console.warn("Unable to subscribe to kernel events", error);
  }
}

async function start() {
  await initKernelStore();
  if (!missionStore.state.active) {
    await missionStore.loadActive();
  }
  const taskId = missionStore.state.active?.taskId || undefined;
  const snapshot = (await invoke("kernel_start", {
    request: {
      session_id: terminalStore.activeSessionId.value,
      max_steps: 8,
      task_id: taskId,
    },
  })) as RunState;
  applyRun(snapshot);
}

async function pause() {
  const snapshot = (await invoke("kernel_pause")) as RunState;
  applyRun(snapshot);
}

async function resume() {
  const snapshot = (await invoke("kernel_resume")) as RunState;
  applyRun(snapshot);
}

async function stop() {
  const snapshot = (await invoke("kernel_stop")) as RunState;
  applyRun(snapshot);
}

async function continueRun() {
  const snapshot = (await invoke("kernel_continue")) as RunState;
  applyRun(snapshot);
}

async function reset() {
  if (state.run?.agentState === "RUNNING") return;
  await start();
}

async function userInput(content: string) {
  const snapshot = (await invoke("kernel_user_input", { request: { content } })) as RunState;
  applyRun(snapshot);
}

async function updatePlan(goal: string, steps: string[], autoGenerate = false) {
  const snapshot = (await invoke("kernel_plan_update", {
    request: {
      goal,
      steps,
      auto_generate: autoGenerate,
    },
  })) as RunState;
  applyRun(snapshot);
}

async function updatePlanStatus(id: string, status: string) {
  const snapshot = (await invoke("kernel_plan_status", { request: { id, status } })) as RunState;
  applyRun(snapshot);
}

export const agentStore = {
  state: readonly(state),
  initKernelStore,
  start,
  pause,
  resume,
  stop,
  continueRun,
  reset,
  userInput,
  updatePlan,
  updatePlanStatus,
};
