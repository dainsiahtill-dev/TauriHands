# AGENTS

本文档用于给 Codex/Agent 提供执行约束与当前代码映射。
最终定义与产品蓝图见 `CONCEPT.md`；面向用户的产品说明见 `README.md`。

## 当前能力（基于代码）

- 驾驶舱布局（Mission / Plan / Loop / Terminal / Diff / Git / Timeline）
- Workspace 沙箱与文件树（路径选择、目录浏览、树视图）
- PTY 终端（xterm.js）与回放日志
- 对话与工具输出回显（Tool output 可折叠展示）
- 运行循环与事件流（runs/events 落盘）
- LLM Profile 存储与连接测试（全局配置）
- Workspace 路径记忆（启动自动恢复上次路径）
- Plan 面板：对话框式计划生成 + 步骤状态切换
- Judge 规则执行（command/tests/git clean）与 JudgeResult 事件
- Task 配置：Goal / Completion / Budget / Risk / Autonomy 保存
- 审计日志（`.taurihands/audit.log`）

## 关键模块映射

前端：
- `src/pages/ConsolePage.vue`：驾驶舱布局（Mission/Plan/Loop/Terminal/Diff/Git/Timeline）。
- `src/components/MissionPanel.vue`：任务配置、Workspace、Judge 规则入口。
- `src/components/PlanPanel.vue`：计划生成对话框与步骤状态。
- `src/components/LoopPanel.vue`：运行状态、执行控制、Chat + Agent 详情。
- `src/components/ChatPanel.vue`：对话输入与工具输出回显。
- `src/components/TerminalPanel.vue`：xterm 终端与回放。
- `src/components/DiffPanel.vue` / `GitPanel.vue` / `TimelinePanel.vue`：证据与交付视图。
- `src/agents/orchestrator.ts`：事件订阅、状态聚合、工具输出映射。
- `src/stores/mission.ts`：任务配置存取。
- `src/components/LLMSettingsPanel.vue`：模型配置与连接测试。

后端：
- `src-tauri/src/services/kernel.rs`：运行循环、计划/任务更新、事件与状态。
- `src-tauri/src/services/pty.rs`：PTY 会话与流式输出。
- `src-tauri/src/services/llm.rs`：模型调用与流式响应。
- `src-tauri/src/services/tools.rs`：文件/搜索/命令执行与安全拦截。
- `src-tauri/src/services/workspace.rs`：路径解析、沙箱与目录树。
- `src-tauri/src/services/audit.rs`：追加式审计日志。
- `src-tauri/src/lib.rs`：Tauri 命令入口、Workspace/LLM 设置存储、任务存储。

## 执行约束

- 事件流是 UI 的唯一真相来源。
- Plan/Task 的变更必须走 Kernel 的 Action/Reducer，不直接由 UI 改写。
- 工具层必须可操作真实世界；控制层负责循环迭代与收敛。
- 需要确认时进入 AWAITING_USER，UI 提供继续/停止入口。
