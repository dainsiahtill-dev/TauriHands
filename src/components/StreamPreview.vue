<script setup lang="ts">
import { computed } from "vue";
import { parseLlmPreview } from "../utils/llmPreview";

const props = defineProps<{
  content: string;
}>();

const preview = computed(() => parseLlmPreview(props.content));
const hasPreview = computed(
  () => Boolean(preview.value?.message) || Boolean(preview.value?.actions?.length),
);

function actionCategory(type: string) {
  if (type.startsWith("fs.")) return "fs";
  if (type.startsWith("terminal.")) return "terminal";
  if (type.startsWith("git.")) return "git";
  if (type.startsWith("plan.")) return "plan";
  if (type.startsWith("task.")) return "task";
  if (type.startsWith("tests.")) return "tests";
  if (type === "user.ask") return "ask";
  return "misc";
}

function actionIcon(type: string) {
  const category = actionCategory(type);
  switch (category) {
    case "fs":
      return "FS";
    case "terminal":
      return "SH";
    case "git":
      return "GT";
    case "plan":
      return "PL";
    case "task":
      return "TK";
    case "tests":
      return "TS";
    case "ask":
      return "QA";
    default:
      return "AI";
  }
}
</script>

<template>
  <div class="stream-preview">
    <template v-if="hasPreview">
      <div v-if="preview?.message" class="stream-card message-card" data-category="message">
        <div class="card-header">
          <span class="card-icon" data-category="message" aria-hidden="true">
            <svg viewBox="0 0 24 24">
              <path
                d="M5 6h14a3 3 0 0 1 3 3v5a3 3 0 0 1-3 3H11l-4 3v-3H5a3 3 0 0 1-3-3V9a3 3 0 0 1 3-3z"
                fill="none"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linejoin="round"
              />
            </svg>
          </span>
          <span class="card-tag">Message</span>
        </div>
        <p class="card-body">{{ preview.message }}</p>
      </div>
      <div v-if="preview?.actions?.length" class="stream-actions">
        <div
          v-for="(action, index) in preview.actions"
          :key="`${action.type}-${index}`"
          class="stream-card action-card"
          :data-type="action.type"
          :data-category="actionCategory(action.type)"
        >
          <div class="card-header">
            <span class="card-icon" :data-category="actionCategory(action.type)" aria-hidden="true">
              <svg v-if="actionCategory(action.type) === 'fs'" viewBox="0 0 24 24">
                <path
                  d="M7 3h7l4 4v14H7z"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linejoin="round"
                />
                <path d="M14 3v4h4" fill="none" stroke="currentColor" stroke-width="1.8" />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'terminal'" viewBox="0 0 24 24">
                <path
                  d="M5 6l6 6-6 6M12 18h7"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'git'" viewBox="0 0 24 24">
                <path
                  d="M7 7v6a3 3 0 0 0 3 3h4"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                />
                <circle cx="7" cy="7" r="2" fill="none" stroke="currentColor" stroke-width="1.8" />
                <circle cx="7" cy="17" r="2" fill="none" stroke="currentColor" stroke-width="1.8" />
                <circle cx="17" cy="16" r="2" fill="none" stroke="currentColor" stroke-width="1.8" />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'plan'" viewBox="0 0 24 24">
                <path
                  d="M6 7h12M6 12h12M6 17h8"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'task'" viewBox="0 0 24 24">
                <path
                  d="M6 12l4 4 8-8"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.4" />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'tests'" viewBox="0 0 24 24">
                <path
                  d="M9 3h6M10 3v5l-4 8a4 4 0 0 0 3.6 5h4.8a4 4 0 0 0 3.6-5l-4-8V3"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.6"
                  stroke-linejoin="round"
                />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'ask'" viewBox="0 0 24 24">
                <path
                  d="M9.5 9.5a3 3 0 1 1 4.5 2.6c-1 .6-1.5 1.1-1.5 2.4"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                />
                <circle cx="12" cy="18" r="1" fill="currentColor" />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'message'" viewBox="0 0 24 24">
                <path
                  d="M5 6h14a3 3 0 0 1 3 3v5a3 3 0 0 1-3 3H11l-4 3v-3H5a3 3 0 0 1-3-3V9a3 3 0 0 1 3-3z"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linejoin="round"
                />
              </svg>
              <svg v-else-if="actionCategory(action.type) === 'raw'" viewBox="0 0 24 24">
                <path
                  d="M8 7l-4 5 4 5M16 7l4 5-4 5M12 6l-2 12"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.6"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
              <svg v-else viewBox="0 0 24 24">
                <path
                  d="M12 3l2.2 4.7 5.2.7-3.8 3.6.9 5.1-4.5-2.4-4.5 2.4.9-5.1-3.8-3.6 5.2-.7z"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.6"
                  stroke-linejoin="round"
                />
              </svg>
              <span class="card-icon-text">{{ actionIcon(action.type) }}</span>
            </span>
            <span class="card-tag">{{ action.type }}</span>
            <span class="card-title">{{ action.title }}</span>
          </div>
          <p v-if="action.detail" class="card-meta">{{ action.detail }}</p>
          <p v-if="action.path" class="card-path">{{ action.path }}</p>
          <pre v-if="action.contentPreview" class="card-code">{{ action.contentPreview }}</pre>
          <ul v-if="action.steps?.length" class="card-steps">
            <li v-for="(step, stepIndex) in action.steps" :key="stepIndex">{{ step }}</li>
          </ul>
        </div>
      </div>
    </template>
    <div v-else class="stream-card raw-card" data-category="raw">
      <div class="card-header">
        <span class="card-icon" data-category="raw" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path
              d="M8 7l-4 5 4 5M16 7l4 5-4 5M12 6l-2 12"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </span>
        <span class="card-tag">Raw</span>
        <span class="card-title">LLM stream</span>
      </div>
      <pre class="card-code">{{ content }}</pre>
    </div>
  </div>
</template>

<style scoped>
.stream-preview {
  display: grid;
  gap: 10px;
  width: 100%;
}

.stream-actions {
  display: grid;
  gap: 8px;
}

.stream-card {
  position: relative;
  border-radius: 12px;
  border: 1px solid rgba(45, 246, 255, 0.18);
  background: linear-gradient(145deg, rgba(8, 12, 20, 0.92), rgba(10, 18, 30, 0.92));
  padding: 10px 12px;
  display: grid;
  gap: 8px;
  box-shadow: 0 8px 18px rgba(0, 0, 0, 0.3);
  overflow: hidden;
}

.stream-card::before {
  content: "";
  position: absolute;
  left: 0;
  top: 10px;
  bottom: 10px;
  width: 3px;
  border-radius: 999px;
  background: linear-gradient(180deg, rgba(45, 246, 255, 0.9), rgba(74, 125, 255, 0.3));
  opacity: 0.7;
}

.stream-card[data-category="fs"]::before {
  background: linear-gradient(180deg, rgba(182, 255, 75, 0.9), rgba(45, 246, 255, 0.3));
}

.stream-card[data-category="terminal"]::before,
.stream-card[data-category="tests"]::before {
  background: linear-gradient(180deg, rgba(45, 246, 255, 0.9), rgba(255, 184, 77, 0.3));
}

.stream-card[data-category="plan"]::before,
.stream-card[data-category="task"]::before {
  background: linear-gradient(180deg, rgba(74, 125, 255, 0.9), rgba(45, 246, 255, 0.3));
}

.stream-card[data-category="git"]::before {
  background: linear-gradient(180deg, rgba(255, 184, 77, 0.9), rgba(45, 246, 255, 0.3));
}

.stream-card[data-category="ask"]::before {
  background: linear-gradient(180deg, rgba(182, 255, 75, 0.9), rgba(74, 125, 255, 0.3));
}

.stream-card[data-category="raw"]::before,
.stream-card[data-category="message"]::before {
  background: linear-gradient(180deg, rgba(138, 160, 183, 0.7), rgba(45, 246, 255, 0.25));
}

.message-card {
  border-color: rgba(74, 125, 255, 0.25);
}

.raw-card {
  border-color: rgba(138, 160, 183, 0.25);
}

.card-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  color: #9bb0c6;
}

.card-icon {
  position: relative;
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.6rem;
  letter-spacing: 0.08em;
  color: #2df6ff;
  background: linear-gradient(150deg, rgba(8, 12, 20, 0.95), rgba(12, 20, 32, 0.85));
  border: 1px solid rgba(45, 246, 255, 0.45);
  text-transform: uppercase;
  box-shadow:
    inset 0 0 10px rgba(45, 246, 255, 0.12),
    0 0 12px rgba(45, 246, 255, 0.35);
}

.card-icon::after {
  content: "";
  position: absolute;
  inset: 4px;
  border-radius: 6px;
  border: 1px solid rgba(45, 246, 255, 0.2);
  opacity: 0.8;
}

.card-icon svg {
  width: 16px;
  height: 16px;
  stroke: currentColor;
  filter: drop-shadow(0 0 6px rgba(45, 246, 255, 0.25));
}

.card-icon-text {
  display: none;
}

.card-icon[data-category="fs"] {
  color: #b6ff4b;
  border-color: rgba(182, 255, 75, 0.45);
  box-shadow:
    inset 0 0 10px rgba(182, 255, 75, 0.16),
    0 0 12px rgba(182, 255, 75, 0.35);
}

.card-icon[data-category="fs"]::after {
  border-color: rgba(182, 255, 75, 0.25);
}

.card-icon[data-category="terminal"],
.card-icon[data-category="tests"] {
  color: #2df6ff;
}

.card-icon[data-category="git"] {
  color: #ffb84d;
  border-color: rgba(255, 184, 77, 0.45);
  box-shadow:
    inset 0 0 10px rgba(255, 184, 77, 0.16),
    0 0 12px rgba(255, 184, 77, 0.35);
}

.card-icon[data-category="git"]::after {
  border-color: rgba(255, 184, 77, 0.25);
}

.card-icon[data-category="plan"],
.card-icon[data-category="task"] {
  color: #4a7dff;
  border-color: rgba(74, 125, 255, 0.45);
  box-shadow:
    inset 0 0 10px rgba(74, 125, 255, 0.16),
    0 0 12px rgba(74, 125, 255, 0.35);
}

.card-icon[data-category="plan"]::after,
.card-icon[data-category="task"]::after {
  border-color: rgba(74, 125, 255, 0.25);
}

.card-icon[data-category="ask"] {
  color: #b6ff4b;
}

.card-icon[data-category="raw"],
.card-icon[data-category="message"] {
  color: #8aa0b7;
  border-color: rgba(138, 160, 183, 0.45);
  box-shadow:
    inset 0 0 10px rgba(138, 160, 183, 0.16),
    0 0 12px rgba(138, 160, 183, 0.3);
}

.card-icon[data-category="raw"]::after,
.card-icon[data-category="message"]::after {
  border-color: rgba(138, 160, 183, 0.2);
}

.card-tag {
  padding: 2px 8px;
  border-radius: 999px;
  border: 1px solid rgba(45, 246, 255, 0.3);
  background: rgba(45, 246, 255, 0.1);
  color: #2df6ff;
  font-size: 0.6rem;
}

.card-title {
  color: var(--text-secondary);
}

.card-body,
.card-meta,
.card-path {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-primary);
  word-break: break-word;
  overflow-wrap: anywhere;
}

.card-meta {
  color: var(--text-secondary);
}

.card-path {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.72rem;
  color: #9bb0c6;
}

.card-steps {
  margin: 0;
  padding-left: 16px;
  display: grid;
  gap: 4px;
  font-size: 0.76rem;
  color: var(--text-secondary);
}

.card-code {
  margin: 0;
  font-size: 0.72rem;
  color: #c7d7ec;
  font-family: "JetBrains Mono", monospace;
  white-space: pre-wrap;
  max-height: 240px;
  overflow: auto;
  background: rgba(5, 8, 14, 0.7);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 8px;
}
</style>
