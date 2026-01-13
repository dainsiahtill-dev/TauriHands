# TauriHands 最终定义与架构蓝图

TauriHands = 本地桌面“软件工程自动驾驶仪”（Tauri + Vue3，Windows 优先）。
你给一个目标，它在真实工程环境里自动完成：

**计划 → 改代码 → 跑命令/测试 → 读错误 → 修复 → 再验证 → 交付**

直到 Judge（客观验收）判定通过，或触发预算/风险策略暂停让你接管。

一句话卖点：一个本地优先、可审计、可回放、可回滚的 AI 编程自动驾驶仪。

---

## 核心原则

- 不信 “done”，只信 “验收通过”。
- autonomous loop 是产品主循环（Loop Engine），不是外部 hook。
- 自动化的是机械执行；价值判断通过 Decision Slots 交给人类。
- 每轮都有证据包（命令/输出/diff/错误摘要），保证可审计与可回放。
- 安全策略默认收敛：越界/高危/外网都需明确策略放行。

---

## 最终体验（Task 驱动）

一个 Task = 一个可运行的“自动循环体”。

创建任务时配置：

- Workspace：受控目录（默认只允许读写此目录）
- Goal：任务目标
- Completion Criteria：验收（tests/build/lint/git clean/自定义规则）
- Budget：最大迭代次数、工具调用次数、最长运行时间、成本上限（可选）
- Risk Policy：网络开关、命令策略（allowlist/blocklist/confirm）、越界文件策略
- Autonomy Level：全自动 / 半自动 / 只出计划

点击 Run 后进入自动循环；随时 Pause / Step / Resume / Rollback。

---

## UI 最终形态（驾驶舱七面板）

- **Mission**：任务列表、状态、预算消耗、最近失败证据
- **Plan**：Checklist Steps（每步含验收标准，可 HARD STOP）
- **Loop**：迭代号、当前 step、动作队列、门禁状态（tests/build/lint）
- **Terminal**：真实命令流式输出（PTY）
- **Diff/Files**：以 patch/diff 为主的变更审阅 + 一键回滚
- **Git/Delivery**：commit/patch/branch（可选 push）
- **Timeline**：事件流 ndjson，可回放、可分叉继续跑

---

## Loop Engine（主循环）

三角色（可单模型实现，后续可多代理）：

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

---

## Tool System（手和笔）

所有工具调用必须走 Tauri command（Rust 后端统一入口），以便权限控制与审计。

必备工具（v1）：

- `terminal.run` / `terminal.exec`（PTY）
- `fs.read` / `fs.write` / `fs.applyPatch` / `fs.search`
- `git.status` / `git.diff` / `git.commit`
- `checkpoint.create` / `checkpoint.restore`
- `browser.fetch` / `browser.search`（可选，受 allowNetwork 控制）

---

## 数据与回放（时光机）

每个 Task 一个目录，事件流追加写（崩溃也不损坏）：

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

App 级数据（全局）：

- `settings.json`：最近 Workspace
- `.taurihands/llm.json`：LLM Profile

---

## Safety & Guardrails

- 文件沙箱：默认只允许 workspaceRoot 读写；敏感路径默认拒绝
- 命令策略：blocklist / allowlist / confirm
- 网络策略：allowNetwork=false 时禁用外网与浏览器抓取

---

## 里程碑（建议落地顺序）

**M1：事件流与可回放骨架**

- EventBus + Timeline（追加写 ndjson）
- RunState + Snapshot（恢复能力）
- UI Timeline 订阅与渲染

**M2：Loop + Tool Host 最小闭环**

- Controller/LoopEngine 主循环
- terminal.exec + fs.read/write/search
- 工具调用审计与错误摘要

**M3：Plan/Task 面板与计划闭环**

- plan.update / task.update
- Plan 面板生成与状态切换
- 任务配置（Completion/Budget/Risk）

**M4：Judge + Stop Policy**

- tests/build/lint 判定规则
- Budget 触发 AWAITING_USER
- Decision Slots（关键点暂停）

**M5：Checkpoint/Rollback**

- git patch/stash 级别回滚
- timeline 跳转 + 分叉恢复

---

## 实现架构（推荐）

- **Renderer（Vue3）**：纯 UI + 状态订阅（tauri event）
- **Core（Rust）**：LoopEngine / ToolDispatcher / Judge / TimelineWriter / CheckpointManager
- **Sidecar（可选）**：pty-host（Node + node-pty 或 Rust PTY），browser-host（Playwright）

数据原则：

- 事件流是 UI 的唯一真相来源
- Plan/Task 变更必须走 Action/Reducer
- 工具层负责“真实执行”；控制层负责“循环迭代与收敛”
