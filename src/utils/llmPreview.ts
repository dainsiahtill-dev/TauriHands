export type LlmPreviewAction = {
  type: string;
  title: string;
  detail?: string;
  path?: string;
  contentPreview?: string;
  steps?: string[];
};

export type LlmPreview = {
  message?: string;
  actions: LlmPreviewAction[];
};

type RawAction = Record<string, unknown>;

export function parseLlmPreview(raw: string): LlmPreview | null {
  const text = raw.trim();
  if (!text) return null;

  const candidates = [
    extractJsonAfterMarker(text, "cleaned_preview"),
    extractJsonAfterMarker(text, "snippet_preview"),
    extractJsonObject(text),
  ];

  for (const candidate of candidates) {
    if (!candidate) continue;
    const parsed = tryParseJson(candidate);
    if (parsed && typeof parsed === "object") {
      const preview = buildPreview(parsed as Record<string, unknown>);
      if (preview) {
        return preview;
      }
    }
  }

  return null;
}

function buildPreview(value: Record<string, unknown>): LlmPreview | null {
  const message = typeof value.message === "string" ? value.message : undefined;
  const actions = Array.isArray(value.actions)
    ? value.actions
        .map((item) => toActionPreview(item as RawAction))
        .filter((item): item is LlmPreviewAction => Boolean(item))
    : [];

  if (!message && actions.length === 0) {
    return null;
  }

  return { message, actions };
}

function toActionPreview(action: RawAction | null | undefined): LlmPreviewAction | null {
  if (!action || typeof action !== "object") return null;
  const type = String(action.type ?? "action");

  switch (type) {
    case "plan.update": {
      const plan = (action.plan as Record<string, unknown> | undefined) ?? {};
      const goal = coerceString(plan.goal);
      const steps = extractSteps(plan.steps);
      return {
        type,
        title: "Plan update",
        detail: goal || undefined,
        steps,
      };
    }
    case "task.update": {
      const tasksValue = (action.tasks as Record<string, unknown> | undefined) ?? {};
      const items = Array.isArray(tasksValue.items) ? tasksValue.items : tasksValue.tasks;
      const count = Array.isArray(items) ? items.length : 0;
      return {
        type,
        title: "Task update",
        detail: count ? `${count} tasks` : undefined,
      };
    }
    case "terminal.exec":
      return {
        type,
        title: "terminal.exec",
        detail: coerceString(action.cmd) || undefined,
      };
    case "terminal.run":
      return {
        type,
        title: "terminal.run",
        detail: formatCommand(action.program, action.args),
      };
    case "fs.read": {
      const path = coerceString(action.path);
      return {
        type,
        title: type,
        path: path || undefined,
      };
    }
    case "fs.search": {
      const path = coerceString(action.path);
      const pattern = coerceString(action.pattern);
      return {
        type,
        title: type,
        detail: pattern || undefined,
        path: path || undefined,
      };
    }
    case "fs.write": {
      const path = coerceString(action.path);
      const content = coerceString(action.content);
      return {
        type,
        title: type,
        path: path || undefined,
        contentPreview: content ? previewText(content, 900, 12) : undefined,
      };
    }
    case "git.status":
    case "git.diff":
      return {
        type,
        title: type,
        detail: coerceString(action.path) || undefined,
      };
    case "tests.run":
      return {
        type,
        title: "tests.run",
        detail: formatCommand(action.program, action.args),
      };
    case "user.ask":
      return {
        type,
        title: "user.ask",
        detail: coerceString(action.question) || undefined,
      };
    default:
      return {
        type,
        title: type,
      };
  }
}

function formatCommand(program: unknown, args: unknown): string | undefined {
  const programText = coerceString(program);
  if (!programText) return undefined;
  const argList = Array.isArray(args)
    ? args.map((value) => coerceString(value)).filter(Boolean)
    : [];
  return [programText, ...argList].join(" ").trim();
}

function extractSteps(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) return undefined;
  const steps = value
    .map((item) => {
      if (typeof item === "string") return item.trim();
      if (!item || typeof item !== "object") return "";
      const record = item as Record<string, unknown>;
      return coerceString(record.title) || coerceString(record.text) || "";
    })
    .filter(Boolean);
  return steps.length ? steps : undefined;
}

function coerceString(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function previewText(value: string, maxChars: number, maxLines: number): string {
  const lines = value.split(/\r?\n/);
  const sliced = lines.slice(0, maxLines);
  let text = sliced.join("\n");
  let truncated = lines.length > maxLines;
  if (text.length > maxChars) {
    text = text.slice(0, maxChars);
    truncated = true;
  }
  return truncated ? `${text}\n…` : text;
}

function extractJsonAfterMarker(text: string, marker: string): string | null {
  const index = text.indexOf(marker);
  if (index < 0) return null;
  const start = text.indexOf("{", index);
  if (start < 0) return null;
  return extractJsonObject(text.slice(start));
}

function extractJsonObject(text: string): string | null {
  const start = text.indexOf("{");
  if (start < 0) return null;
  let depth = 0;
  for (let i = start; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === "{") depth += 1;
    if (ch === "}") depth -= 1;
    if (depth === 0) {
      return text.slice(start, i + 1);
    }
  }
  return null;
}

function tryParseJson(text: string): unknown | null {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}
