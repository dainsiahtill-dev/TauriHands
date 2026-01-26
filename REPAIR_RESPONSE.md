Product Owner  
- Keep scope tiny: normalize JudgeResult rendering only.  
UX/UI  
- Show consistent pass/pending/fail chip and a concise “status — reason — counts” line.  
Frontend  
- Add status helpers + summary formatter in `src/components/TimelinePanel.vue`; bind chip/summary to helpers.  
Backend  
- No changes.  
QA  
- Replay RUNLOG; check pass→green, fail→amber when top-level status missing; search/filter still works.

Smallest shippable change (cannot apply due to sandbox write block)  
- Insert helpers after imports in `src/components/TimelinePanel.vue`:
```ts
type JudgeCheck = { id?: string; type?: string; status?: string; reason?: string; evidence?: string[] };
type JudgeResult = { status?: string; reasons?: string[]; checks?: JudgeCheck[] };

const STATUS_ORDER = ["pass", "pending", "fail"] as const;

function normalizeStatus(value?: string) {
  const s = value?.toLowerCase();
  if (["pass", "ok", "success"].includes(s)) return "pass";
  if (["fail", "failed", "error"].includes(s)) return "fail";
  if (["pending", "running", "unknown"].includes(s)) return "pending";
  return "pending";
}

function deriveJudgeStatus(result?: JudgeResult) {
  if (!result) return "pending";
  const top = normalizeStatus(result.status);
  if (top !== "pending" || !result.checks?.length) return top;
  const worst = result.checks.reduce((score, check) => {
    const s = normalizeStatus(check.status);
    return Math.max(score, s === "fail" ? 2 : s === "pending" ? 1 : 0);
  }, 0);
  return worst === 2 ? "fail" : worst === 1 ? "pending" : "pass";
}

function formatJudgeSummary(result: JudgeResult) {
  const status = deriveJudgeStatus(result);
  const reason = result.reasons?.[0] ?? result.checks?.find((c) => c.reason)?.reason;
  const counts = result.checks?.reduce<Record<string, number>>((acc, check) => {
    const s = normalizeStatus(check.status);
    acc[s] = (acc[s] ?? 0) + 1;
    return acc;
  }, {});
  const countsLabel = counts
    ? Object.entries(counts)
        .filter(([, v]) => v)
        .map(([k, v]) => `${v} ${k}`)
        .join(" · ")
    : "";
  return [status, reason, countsLabel].filter(Boolean).join(" — ").slice(0, 160);
}
```
- Update `summarize` branch:
```ts
if (event.type === "JudgeResult") {
  const result = event.payload?.result as JudgeResult | undefined;
  return result ? formatJudgeSummary(result) : "judge result";
}
```
- Bind chip in template:
```vue
<span class="judge-chip" :data-status="deriveJudgeStatus(event.payload.result)">
  {{ deriveJudgeStatus(event.payload.result) }}
</span>
```

Decision  
- Proceed with the single TimelinePanel status normalization; no backend changes.

QA notes  
- Replay `RUNLOG.md`; expect summary like `fail — tests failed — 1 fail · 1 pending`.  
- Confirm chip colors track derived status.  
- Verify search/filter and detail toggles still operate.

Summary: Provided the exact TimelinePanel patch to normalize JudgeResult status/summary; unable to apply due to write restrictions.  
Next Step: Apply the patch to `src/components/TimelinePanel.vue`, then replay RUNLOG to confirm chips and summaries render correctly.