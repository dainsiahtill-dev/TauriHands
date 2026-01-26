# PLAN
# Write the next batch of tasks for Codex here.
# Example:
# - Finish ToolSettings integration end-to-end.
# - Implement SecuritySettings allowlist and hook into backend.

- Wire JudgeResult events into TimelinePanel with clear success/needs-attention badges.
- Add MissionPanel validation and inline feedback messaging for Goal/Completion/Budget.
- Persist Plan steps through kernel reducer on status toggles; ensure UI reads from events only.
- Improve tool output folding UX in ChatPanel (auto-collapse large outputs, show summary line).
- Add workspace tree refresh action and ensure last-opened path restores reliably.
