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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JudgeContext {
    pub iteration: u32,
    pub last_error: Option<String>,
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

    pub fn evaluate(&self, context: &JudgeContext) -> JudgeResult {
        if self.rules.is_empty() {
            return JudgeResult {
                status: "skip".to_string(),
                reasons: Vec::new(),
                evidence: Vec::new(),
            };
        }
        let mut reasons = Vec::new();
        if let Some(error) = &context.last_error {
            reasons.push(error.clone());
        }
        JudgeResult {
            status: "pending".to_string(),
            reasons,
            evidence: Vec::new(),
        }
    }
}
