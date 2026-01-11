# AGENTS

本文档用于回顾 TauriHands 的现有代码与功能，并结合最新需求总结最终目标。

## TauriHands 是什么

TauriHands 是一个本地桌面 AI Agent 工作台（Windows 优先），把“对话 + 计划 + 终端 + 文件 +
Git + 浏览器 + 时间线”统一成一个可追踪、可回放、可自动执行的开发指挥台。目标是让 AI
像工程师一样完成任务：改代码、跑命令、修错误、跑测试、交付 patch/PR。

## 核心功能（目标能力）

1) 任务驱动的多回合自动执行：失败不终止，继续迭代直到通过。
2) 真终端与真实执行反馈：PTY、流式输出、exit code、取消/重试。
3) 文件工作区与代码编辑：读写/搜索/批量变更，支持 diff/patch。
4) Git 变更闭环：status/diff/commit/patch，可回滚可追溯。
5) 浏览器/文档检索：可控浏览与抓取，解决知识截止与依赖问题。
6) 时间线/事件日志/可回放：每一步可审计可复盘。

## 执行闭环（推荐状态机）

Intake → Plan → Execute → Observe → Evaluate → Fix/Refine → Finalize

## 当前能力（基于代码）

- Workspace 沙箱：路径选择、目录浏览、文件树视图。
- PTY 终端：portable-pty 流式输出、回放日志、多 Tab UI。
- 对话面板：用户输入进入 Kernel，渲染 assistant 消息。
- Agent 面板：状态、计划/任务、工具调用、日志、最后错误。
- LLM Profile 存储与连接测试。
- Kernel 事件流与运行快照落盘（`.taurihands/events`、`.taurihands/runs`）。
- 工具与终端审计日志（`.taurihands/audit.log`）。

## 关键模块

前端

- `src/pages/ConsolePage.vue`：Workspace + Chat + Terminal + Agent 布局。
- `src/components/WorkspacePanel.vue`：工作区与文件树。
- `src/components/TerminalPanel.vue`：xterm 终端、回放/捕获。
- `src/components/ChatPanel.vue`：Kernel 对话输入与消息渲染。
- `src/components/AgentPanel.vue`：计划/任务/工具调用/日志。
- `src/agents/orchestrator.ts`：事件订阅与状态整合。

后端

- `src-tauri/src/services/kernel.rs`：LLM 运行循环、行动执行、事件与状态。
- `src-tauri/src/services/pty.rs`：PTY 会话与输出流。
- `src-tauri/src/services/llm.rs`：OpenAI/Anthropic/本地模型调用与存储。
- `src-tauri/src/services/tools.rs`：读写/搜索/命令执行与安全拦截。
- `src-tauri/src/services/workspace.rs`：路径解析与沙箱。
- `src-tauri/src/services/audit.rs`：追加式审计日志。

## 后续注意事项

- 事件流是 UI 的唯一真相来源。
- Plan/Task 的变更必须走 Action/Reducer，不直接由 UI 改写。
- 工具层必须可操作真实世界；控制层必须能循环迭代。
