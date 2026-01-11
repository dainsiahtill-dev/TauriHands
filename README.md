# TauriHands

TauriHands 是一个本地桌面 AI Agent 工作台（Windows 优先），把“对话 + 计划 + 终端 + 文件 +
Git + 浏览器 + 时间线”统一成一个可追踪、可回放、可自动执行的开发指挥台。目标是让 AI 不只是
“给代码”，而是像工程师一样真正完成任务：改代码、跑命令、修错误、跑测试、交付 patch/PR。

## 最终目标（产品形态）

- 从聊天助手升级为自动化工程师：可计划、可执行、可验证、可交付。
- 一键接管开发闭环：计划 → 执行 → 反馈 → 修复 → 验证 → 交付。
- 全程可追踪可回放：事件日志 + 工具调用 + 文件变更可复盘。
- 工具可插拔：终端/文件/Git/浏览器统一工具协议，模型可替换。
- 本地优先更安全：代码与密钥留在本机，联网策略可控。

## 核心功能（目标能力）

1) 任务驱动的多回合自动执行

用户只给目标，Agent 自动拆解计划并迭代执行，失败也会继续修复直到通过。

2) 真终端 + 真执行反馈

可被 AI 调用的 PTY 终端，支持流式输出、exit code、长任务、取消/重试。

3) 文件工作区与代码编辑

工作区目录树、搜索与批量变更，读写文件带 diff/patch，支持多文件一致性修改。

4) Git 变更闭环

自动检查 status/diff，生成提交信息与 patch，可回滚可追溯。

5) 浏览器/文档检索

可控的网页浏览或自动化，用于查文档、查依赖、查报错。

6) 时间线 / 事件日志 / 可回放

记录模型输出、工具调用、终端输出、文件变更与关键决策，支持回放与回滚。

## 执行闭环（推荐状态机）

Intake → Plan → Execute → Observe → Evaluate → Fix/Refine → Finalize

## 当前已实现（基于代码）

- Workspace 面板：工作区路径设置、文件夹选择、沙箱化文件树。
- PTY 终端：多 Tab、重命名/克隆、回放、输出捕获（xterm.js）。
- 对话面板：用户输入进入 Kernel，渲染 assistant 消息。
- Agent 面板：状态、计划编辑、任务列表、工具调用、日志、最后错误。
- LLM 设置：模型/Provider 配置、保存 Profile、连接测试。
- Kernel 事件流持久化至 `.taurihands/events/*.jsonl`。
- 运行快照 `.taurihands/runs/*.json` + 审计日志 `.taurihands/audit.log`。

## 架构概览

前端（Vue 3 + Vite）

- `src/layouts/AppShell.vue`：顶部导航与页面骨架。
- `src/pages/ConsolePage.vue`：Workspace + Chat + Terminal + Agent 布局。
- `src/pages/LLMSettingsPage.vue`：LLM 设置页。
- `src/components/*`：WorkspacePanel / TerminalPanel / ChatPanel / AgentPanel。
- `src/agents/orchestrator.ts`：Kernel 事件订阅与状态聚合。

后端（Tauri + Rust）

- `src-tauri/src/services/pty.rs`：PTY 会话、输出流、回放日志。
- `src-tauri/src/services/kernel.rs`：LLM 决策、行动执行、事件与状态。
- `src-tauri/src/services/llm.rs`：OpenAI/Anthropic/本地模型请求与存储。
- `src-tauri/src/services/tools.rs`：读写/搜索/命令执行 + 安全审计。
- `src-tauri/src/services/workspace.rs`：工作区沙箱与路径解析。
- `src-tauri/src/services/audit.rs`：追加式审计日志。

## 本地数据目录

工作区根目录下 `.taurihands/`：

- `audit.log`：审计日志。
- `events/*.jsonl`：Kernel 事件流。
- `runs/*.json`：运行快照。
- `llm.json`：LLM Profile。
- `terminal/`：终端回放日志。

## 开发

前置条件：

- Node.js（建议 18+）
- Rust toolchain（rustup + cargo）
- Tauri CLI（`npm run tauri` 已包含）

命令：

```bash
npm install
npm run dev        # 仅前端
npm run tauri dev  # 桌面端完整运行
```

## 备注

- Windows 路径会自动去掉 `\\?\` 前缀再显示与传入。
- 当前 UI 已经绑定 Kernel 事件流；旧的 `agent` 逻辑仍在后端保留但不作为主流程。
