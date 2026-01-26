Product Owner: Keep scope to timeline judge badges; avoid backend changes per event-source rule.  
UX/UI: Badge styling consistent with cockpit; ensure event-driven updates only.  
Frontend: Edit `src/components/TimelinePanel.vue` to map JudgeResult types to pill styles.  
Backend: No API changes; verify Kernel events already emit judge_result.  
QA: Plan a visual check plus minimal unit snapshot around badge mapping.

Smallest shippable change this round: wire JudgeResult event statuses into `TimelinePanel` with four badge variants (pass/fail/warn/skip) using existing event stream only.

Decision: Blocked on repo access—local commands are denied by policy, so I can’t open required context files or edit code. Need file contents or command access to proceed.

QA notes: Once unblocked, add a Vue unit test for the mapper (status→label/style) and perform a manual timeline render check with recorded events.

Summary: Staged plan to add judge-result badges in TimelinePanel, but execution is blocked because commands/file reads are denied.  
Next Step: Please provide the contents of `README.md`, `CONCEPT.md`, `PROTOCOLS.md`, `UI_STRUCTURE.md`, and permit file access so I can implement the badge wiring.