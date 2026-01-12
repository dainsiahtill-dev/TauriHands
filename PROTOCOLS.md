# PROTOCOLS

This file defines the core contracts for the Loop Engine.

## Event Schema

```json
{
  "id": "uuid",
  "runId": "uuid",
  "ts": 1710000000000,
  "seq": 12,
  "type": "ToolCallStarted",
  "payload": {}
}
```

Recommended event types:
- StateChanged
- UserMessage
- AgentMessage
- AgentMessageChunk
- AgentMessageDone
- AgentActionProposed
- ToolCallStarted
- ToolCallChunk
- ToolCallFinished
- Observation
- PlanUpdated
- TaskUpdated
- JudgeResult
- Error

## Tool Action Schema (v1)

Base shape:

```json
{
  "id": "tool-uuid",
  "type": "terminal.run"
}
```

Actions:

```json
{ "id": "...", "type": "terminal.run", "program": "npm", "args": ["test"], "cwd": "optional" }
{ "id": "...", "type": "terminal.exec", "cmd": "dir", "cwd": "optional" }
{ "id": "...", "type": "fs.read", "path": "src/main.ts" }
{ "id": "...", "type": "fs.write", "path": "src/main.ts", "content": "..." }
{ "id": "...", "type": "fs.applyPatch", "path": "src/main.ts", "patch": "..." }
{ "id": "...", "type": "fs.search", "pattern": "TODO", "paths": ["src"] }
{ "id": "...", "type": "git.status" }
{ "id": "...", "type": "git.diff", "path": "optional" }
{ "id": "...", "type": "git.commit", "message": "..." }
{ "id": "...", "type": "checkpoint.create", "label": "optional" }
{ "id": "...", "type": "checkpoint.restore", "id": "checkpoint-id" }
{ "id": "...", "type": "browser.fetch", "url": "https://..." }
```

Tool result -> observation:

```json
{
  "ok": true,
  "summary": "short summary",
  "exitCode": 0,
  "artifacts": { "path": "..." },
  "raw": {}
}
```

## Plan Schema

```json
{
  "version": 1,
  "goal": "Fix failing tests",
  "steps": [
    {
      "id": "step-1",
      "title": "Run tests",
      "criteria": ["tests pass"],
      "status": "pending",
      "done": false,
      "hardStop": false
    }
  ]
}
```

## Task Config Schema

```json
{
  "taskId": "uuid",
  "workspace": "D:/repo",
  "goal": "Fix failing tests",
  "completion": ["tests pass", "git clean"],
  "budget": { "maxIterations": 30, "maxToolCalls": 200, "maxWallTimeMs": 3600000 },
  "riskPolicy": {
    "allowNetwork": false,
    "commandPolicy": "confirm",
    "pathPolicy": "workspace_only"
  },
  "autonomy": "auto"
}
```

## Judge Rules + Result

Rule shape:

```json
{
  "id": "tests_pass",
  "type": "command",
  "command": ["npm", "test"],
  "successMatch": "PASS",
  "failMatch": "FAIL"
}
```

Judge result:

```json
{
  "status": "pass",
  "reasons": [],
  "evidence": ["events:tool-123", "timeline:456"]
}
```

## Evidence Pack (per iteration)

```json
{
  "iteration": 5,
  "stepId": "step-2",
  "toolCalls": ["tool-1", "tool-2"],
  "diff": "patch-id",
  "errors": ["..."],
  "summary": "what changed"
}
```
