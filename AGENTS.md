# AGENTS

本文档用于给 Codex/Agent 提供执行约束与当前代码映射。
最终定义与产品蓝图见 `CONCEPT.md`。

## 当前能力（基于代码）

- Workspace 沙箱：路径选择、目录浏览、文件树视图。
- PTY 终端：portable-pty 流式输出、回放日志、多 Tab UI。
- 对话面板：用户输入进入 Kernel，渲染 assistant 消息。
- Agent 面板：状态、计划/任务、工具调用、日志、最后错误。
- LLM Profile 存储与连接测试。
- Kernel 事件流与运行快照落盘（`.taurihands/events`、`.taurihands/runs`）。
- 工具与终端审计日志（`.taurihands/audit.log`）。

## 关键模块映射

前端：
- `src/pages/ConsolePage.vue`：Workspace + Chat + Terminal + Agent 布局。
- `src/components/WorkspacePanel.vue`：工作区与文件树。
- `src/components/TerminalPanel.vue`：xterm 终端、回放/捕获。
- `src/components/ChatPanel.vue`：Kernel 对话输入与消息渲染。
- `src/components/AgentPanel.vue`：计划/任务/工具调用/日志。
- `src/agents/orchestrator.ts`：事件订阅与状态整合。

后端：
- `src-tauri/src/services/kernel.rs`：LLM 运行循环、行动执行、事件与状态。
- `src-tauri/src/services/pty.rs`：PTY 会话与输出流。
- `src-tauri/src/services/llm.rs`：OpenAI/Anthropic/本地模型调用与存储。
- `src-tauri/src/services/tools.rs`：读写/搜索/命令执行与安全拦截。
- `src-tauri/src/services/workspace.rs`：路径解析与沙箱。
- `src-tauri/src/services/audit.rs`：追加式审计日志。

## 执行约束

- 事件流是 UI 的唯一真相来源。
- Plan/Task 的变更必须走 Action/Reducer，不直接由 UI 改写。
- 工具层必须可操作真实世界；控制层必须能循环迭代。
