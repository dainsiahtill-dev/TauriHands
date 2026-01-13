use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct JudgeRule {
    pub id: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub command: Option<Vec<String>>,
    pub success_match: Option<String>,
    pub fail_match: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JudgeResult {
    pub status: String,
    pub reasons: Vec<String>,
    pub evidence: Vec<String>,
    #[serde(default)]
    pub checks: Vec<JudgeCheck>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JudgeContext {
    pub iteration: u32,
    pub last_error: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JudgeCheck {
    pub id: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Clone)]
pub struct JudgeRuleOutcome {
    pub status: String,
    pub reason: Option<String>,
    pub evidence: Vec<String>,
}

impl JudgeRuleOutcome {
    pub fn pass() -> Self {
        Self {
            status: "pass".to_string(),
            reason: None,
            evidence: Vec::new(),
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            status: "fail".to_string(),
            reason: Some(reason.into()),
            evidence: Vec::new(),
        }
    }

    pub fn skip(reason: impl Into<String>) -> Self {
        Self {
            status: "skip".to_string(),
            reason: Some(reason.into()),
            evidence: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct JudgeEngine {
    rules: Vec<JudgeRule>,
}

impl JudgeEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn set_rules(&mut self, rules: Vec<JudgeRule>) {
        self.rules = rules;
    }

    pub fn rules(&self) -> &[JudgeRule] {
        &self.rules
    }

    pub fn evaluate_rules<F>(
        rules: &[JudgeRule],
        context: &JudgeContext,
        mut exec: F,
    ) -> JudgeResult
    where
        F: FnMut(&JudgeRule, &JudgeContext) -> JudgeRuleOutcome,
    {
        if rules.is_empty() {
            return JudgeResult {
                status: "skip".to_string(),
                reasons: Vec::new(),
                evidence: Vec::new(),
                checks: Vec::new(),
            };
        }

        let mut checks = Vec::new();
        let mut reasons = Vec::new();
        let mut evidence = Vec::new();
        let mut saw_pass = false;
        let mut saw_fail = false;
        let mut saw_pending = false;
        let mut saw_skip = false;

        for rule in rules {
            let outcome = exec(rule, context);
            let status = outcome.status.clone();
            match status.as_str() {
                "pass" => saw_pass = true,
                "fail" | "error" => saw_fail = true,
                "pending" => saw_pending = true,
                "skip" => saw_skip = true,
                _ => saw_pending = true,
            }
            if let Some(reason) = &outcome.reason {
                if status == "fail" || status == "error" {
                    reasons.push(reason.clone());
                }
            }
            for item in &outcome.evidence {
                evidence.push(item.clone());
            }
            checks.push(JudgeCheck {
                id: rule.id.clone(),
                rule_type: rule.rule_type.clone(),
                status,
                reason: outcome.reason,
                evidence: outcome.evidence,
            });
        }

        let status = if saw_fail {
            "fail"
        } else if saw_pending {
            "pending"
        } else if saw_pass {
            "pass"
        } else if saw_skip {
            "skip"
        } else {
            "pending"
        };

        if status != "pass" {
            if let Some(error) = &context.last_error {
                if !reasons.iter().any(|value| value == error) {
                    reasons.push(error.clone());
                }
            }
        }

        JudgeResult {
            status: status.to_string(),
            reasons,
            evidence,
            checks,
        }
    }
}
