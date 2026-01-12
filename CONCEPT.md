# TauriHands 最终定义与架构蓝图

TauriHands = 本地桌面“软件工程自动驾驶仪”（Tauri + Vue3，Windows 优先）。
你给一个目标，它在真实工程环境里自动完成：

计划 → 改代码 → 跑命令/测试 → 读错误 → 修复 → 再验证 → 交付

直到 Judge（客观验收）判定通过，或触发预算/风险策略暂停让你接管。

一句话卖点：
一个本地优先、可审计、可回放、可回滚的 AI 编程自动驾驶仪，用客观验收驱动自主循环，让 AI 真正把任务做完。

## 核心理念

- 不信“done”，只信“验收通过”（Judge 通过才算完成）。
- autonomous loop 是产品主循环（Loop Engine），不是外部 hook。
- 机械执行自动化，人类决策在关键点插槽中介入（Decision Slots）。
- 迭代必须可回滚、可恢复、可审计（Evidence + Timeline + Checkpoint）。

## 最终体验（Task 驱动）

一个 Task = 一个可运行的自动循环体。创建任务时配置：

- Workspace：受控目录（默认只允许读写此目录）
- Goal：任务目标
- Completion Criteria：验收（tests/build/lint/git clean/自定义规则）
- Budget：最大迭代次数、工具调用次数、最长运行时间、成本上限（可选）
- Risk Policy：网络开关、命令策略（allowlist/blocklist/confirm）、越界文件策略
- Autonomy Level：全自动 / 半自动 / 只出计划

点击 Run 后进入自动循环；随时 Pause / Step / Resume / Rollback。

## 驾驶舱 UI（七面板）

- Mission（任务）：任务列表、状态、进度、预算消耗、最近失败证据
- Plan（计划）：Checklist Steps（每步有验收标准，可 HARD STOP）
- Loop（循环）：迭代号、当前 step、动作队列、门禁状态（tests/build/lint）
- Terminal（证据 1）：真实命令流式输出（PTY）
- Diff/Files（证据 2）：以 patch/diff 为主的变更审阅 + 一键回滚本轮
- Git/Delivery（交付）：commit/patch/branch（可选 push）
- Timeline（审计/回放）：事件流 ndjson，可跳回 checkpoint 并分叉继续跑

## Loop Engine（主循环）

三个角色（可单模型实现，后续可多代理）：

- Planner：生成/更新计划（Steps + 验收）
- Executor：调用工具执行（终端/文件/git/浏览器）
- Judge：客观判定（通过/失败/需要用户决策）

每轮 iteration：

1) Gather Context：workspace 状态、git diff、最近错误、计划进度
2) Pick Step：下一个未完成 step（失败则注入 repair step）
3) Propose Actions：模型输出结构化 ToolCalls（受 allowedTools 限制）
4) Execute Actions：工具层执行并记录证据
5) Judge Step：按 acceptance rules 判定
6) Fail → Backpressure：抽取失败证据喂回模型
7) Checkpoint：每 N 轮/关键节点创建 checkpoint
8) Global Completion Judge：满足全局验收 → 交付并结束

## Decision Slots（人类决策插槽）

在关键点暂停为 needs_user：

- 架构性改动/大规模重构
- 涉及安全/权限/成本
- 预算阈值达到
- Plan 中标记为 HARD STOP 的步骤

用户选择继续/调整方向/终止后，Loop Engine 再继续自动执行。

## Evidence Pack（证据包）

每轮循环必须落盘证据：

- 运行的命令 + stdout/stderr
- 变更 diff/patch
- 失败摘要（错误栈、测试名）
- 修复动作与结果

证据用于 Judge 判定与回放审计。

## Tool System（手和笔）

所有工具调用必须走 Tauri command（Rust 后端统一入口），以便权限控制、审计记录、可取消/可暂停、跨平台一致。

必备工具（v1）：

- terminal.run（PTY）
- fs.read / fs.write / fs.applyPatch / fs.search
- git.status / git.diff / git.commit
- checkpoint.create / checkpoint.restore
- browser.fetch/search（可选，受 allowNetwork 控制）

## Safety & Guardrails

- 文件沙箱：默认只允许 workspaceRoot 读写；敏感路径默认拒绝
- 命令策略：blocklist / allowlist / confirm
- 网络策略：allowNetwork=false 时禁用外网与浏览器抓取

## 持久化与回放（时光机）

每个 Task 一个目录，事件流追加写：

```
.taurihands/
  tasks/<taskId>/
    task.json
    plan.json
    timeline.ndjson
    checkpoints/
      0001/
        meta.json
        git.patch
```

- timeline.ndjson：模型消息、工具调用、结果、Judge 判定
- checkpoint：最小可行用 git patch/stash；高级版可做文件快照

## 推荐架构

- Renderer（Vue3）：纯 UI + 状态订阅（tauri event）
- Core（Rust）：LoopEngine / ToolDispatcher / Judge / TimelineWriter / CheckpointManager
- Sidecar（推荐）：pty-host（Node + node-pty 或 Rust PTY）；可选 browser-host

## Tauri 版优势

- 更强的安全边界与权限治理（Rust + Tauri）
- 更轻量、更像本地开发工具常驻
- 工具层可用 Rust 实现更稳（fs/patch/judge/timeline）

## 当前实现（基于现有代码）

- Workspace 沙箱与文件树
- PTY 终端（xterm.js）与回放日志
- 对话/Agent 面板 + Kernel 事件流
- LLM Profile 存储与连接测试
- 工具审计日志与运行快照

代码映射（要点）：

前端：
- src/pages/ConsolePage.vue：Workspace + Chat + Terminal + Agent 布局
- src/components/WorkspacePanel.vue：工作区与文件树
- src/components/TerminalPanel.vue：xterm 终端、回放/捕获
- src/components/ChatPanel.vue：Kernel 对话输入与消息渲染
- src/components/AgentPanel.vue：计划/任务/工具调用/日志
- src/agents/orchestrator.ts：事件订阅与状态整合

后端：
- src-tauri/src/services/kernel.rs：LLM 运行循环、行动执行、事件与状态
- src-tauri/src/services/pty.rs：PTY 会话与输出流
- src-tauri/src/services/llm.rs：OpenAI/Anthropic/本地模型调用与存储
- src-tauri/src/services/tools.rs：读写/搜索/命令执行与安全拦截
- src-tauri/src/services/workspace.rs：路径解析与沙箱
- src-tauri/src/services/audit.rs：追加式审计日志
