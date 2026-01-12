# TauriHands

TauriHands 是一个本地桌面 **“软件工程自动驾驶仪”**（Tauri + Vue3，Windows 优先）。

你给一个目标，它会在真实工程环境里自动执行：

**计划 → 改代码 → 跑命令/测试 → 读错误 → 修复 → 再验证 → 交付**

直到 **Judge（客观验收）** 判定通过，或触发预算/风险策略暂停让你接管。

- 完整定义与架构：`CONCEPT.md`
- Agent 执行约束与代码映射：`AGENTS.md`

---

## 为什么是 TauriHands

传统的 AI 编程助手大多停留在“生成文本代码”。TauriHands 关注的是把软件工程闭环跑通：

- **真执行**：在本地 Workspace 里运行真实命令、真实测试
- **可审计**：所有工具调用、输出、变更、决策都有事件流与审计日志
- **可回放**：能回看每一步发生了什么，便于复盘与纠错
- **可控自动化**：预算与风险策略让自动化“可收敛、可暂停、可接管”

---

## 核心能力（目标能力）

- **任务驱动的多回合自动执行**：失败不终止，继续迭代直到通过（或触发预算/风险策略）。
- **真终端与真实执行反馈**：PTY、流式输出、exit code、取消/重试、多 Tab。
- **文件工作区与代码编辑**：读写/搜索/批量变更，支持 diff/patch（建议以 patch 为主）。
- **Git 变更闭环**：status/diff/commit/patch，可回滚可追溯。
- **浏览器/文档检索**：可控浏览与抓取，解决知识截止与依赖问题（按策略开关）。
- **时间线/事件日志/可回放**：每一步可审计、可复盘、可恢复（runs + events）。

> 关键原则：**不信“done”，只信“验收通过”。**  
> 自动化的是机械执行；价值判断通过“暂停点/检查点”交给人类接管。

---

## 当前实现（基于现有代码）

已实现：

- 驾驶舱布局（Mission / Plan / Loop / Terminal / Diff / Git / Timeline）
- Workspace 沙箱与文件树（路径选择、目录浏览、树视图）
- PTY 终端（xterm.js）与回放日志
- 对话与工具输出回显（Tool output 可折叠展示）
- 运行循环与事件流（runs/events 落盘）
- LLM Profile 存储与连接测试
- Workspace 路径记忆（启动自动恢复上次路径）
- 审计日志（`.taurihands/audit.log`）

进行中 / 计划：

- Judge（客观验收规则引擎：tests/build/lint/git clean）
- Completion Criteria（任务级停止条件）
- Checkpoint / Rollback（最小可行：git patch/stash + 事件）
- Budget & Stop Policy（最大迭代/工具调用/运行时间）
- Decision Slots（关键步骤暂停等待确认）

---

## UI 面板（驾驶舱）

- **Mission**：任务配置（Goal/Completion/Budget/Risk/Autonomy）+ Workspace
- **Plan**：计划生成与步骤管理
- **Loop**：运行状态、Tool/LLM 活动、对话与执行控制
- **Terminal**：PTY 终端与回放
- **Diff / Git / Timeline**：变更审阅、交付与事件回放

---

## 计划面板（Plan）

Plan 面板现在已支持**对话框式计划生成**，并与 Kernel 的计划系统打通：

- 点击 **Generate plan** 打开对话框
- **Goal** 必填；**Steps** 可选（每行一个）
- Steps 为空时，自动调用 LLM 生成 4~8 步计划
- Steps 填写时，直接以手动步骤生成计划
- 计划生成后自动同步 Task 列表，并可切换步骤状态
- 计划会写入任务目录（`.taurihands/tasks/<taskId>/plan.json`）

> 若启用“自动生成”，需要先在 **LLM Settings** 配置可用模型与 Key。

---

## 架构一览（高层）

- **Vue3 UI（Renderer）**：驾驶舱 UI（Mission / Plan / Loop / Terminal / Diff / Git / Timeline）
- **Kernel（Rust/Tauri Service）**：LLM 运行循环、工具调度、事件流、快照
- **Tools（Rust）**：文件/搜索/命令执行与安全拦截
- **PTY（Rust Service）**：终端会话与流式输出
- **Audit / Runs / Events**：追加式审计与可回放事件存储

数据原则：

- **事件流是 UI 的唯一真相来源**
- Plan/Task 的变更必须走 **Action/Reducer**（不允许 UI 直接改写状态）
- 工具层负责“真实执行”；控制层负责“循环迭代与收敛”

---

## 数据存储

- Workspace 级数据：`.taurihands/`
  - `runs/`：运行快照
  - `events/`：事件流
  - `tasks/<taskId>/task.json`、`plan.json`
  - `audit.log`：审计日志
- App 级数据（全局）：
  - `settings.json`：最近 Workspace
  - `.taurihands/llm.json`：LLM Profile

---

## 快速开始

### 前置条件

- Node.js 18+
- Rust toolchain（`rustup` + `cargo`）
- Tauri CLI（已通过 npm scripts 集成，无需全局安装也可）

### 运行

```bash
npm install
npm run dev        # 前端（可选）
npm run tauri dev  # 桌面端（推荐）
```
