use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Context, Result};
use async_trait::async_trait;

use super::engine::{AutomationResult, AutomationConfig, TaskType, TaskStatus};

#[async_trait]
pub trait TaskValidator: Send + Sync {
    async fn validate(&self, result: &AutomationResult) -> Result<bool>;
    fn get_validation_report(&self, result: &AutomationResult) -> Result<ValidationReport>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub task_id: uuid::Uuid,
    pub is_valid: bool,
    pub checks: Vec<ValidationCheck>,
    pub score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

pub struct DefaultValidator {
    config: AutomationConfig,
}

impl DefaultValidator {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self { config })
    }

    async fn validate_code_generation(&self, result: &AutomationResult) -> Result<ValidationReport> {
        let mut checks = Vec::new();
        let mut score = 100.0;

        // Check if files were generated
        if result.artifacts.is_empty() {
            checks.push(ValidationCheck {
                name: "Files Generated".to_string(),
                passed: false,
                message: "No files were generated".to_string(),
                severity: ValidationSeverity::Error,
            });
            score -= 50.0;
        } else {
            checks.push(ValidationCheck {
                name: "Files Generated".to_string(),
                passed: true,
                message: format!("{} files were generated", result.artifacts.len()),
                severity: ValidationSeverity::Info,
            });
        }

        // Check if generated code compiles
        let mut compilation_passed = true;
        for artifact in &result.artifacts {
            if artifact.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(output) = Command::new("rustc")
                    .arg(artifact)
                    .arg("--emit")
                    .arg("metadata")
                    .output()
                {
                    if !output.status.success() {
                        compilation_passed = false;
                        checks.push(ValidationCheck {
                            name: "Compilation".to_string(),
                            passed: false,
                            message: format!("Failed to compile {:?}", artifact),
                            severity: ValidationSeverity::Error,
                        });
                        score -= 30.0;
                    }
                }
            }
        }

        if compilation_passed {
            checks.push(ValidationCheck {
                name: "Compilation".to_string(),
                passed: true,
                message: "All generated code compiles successfully".to_string(),
                severity: ValidationSeverity::Info,
            });
        }

        // Check code quality metrics
        if let Some(lines_of_code) = result.metrics.get("lines_of_code") {
            if *lines_of_code < 10.0 {
                checks.push(ValidationCheck {
                    name: "Code Volume".to_string(),
                    passed: false,
                    message: "Very little code generated".to_string(),
                    severity: ValidationSeverity::Warning,
                });
                score -= 10.0;
            } else if *lines_of_code > 1000.0 {
                checks.push(ValidationCheck {
                    name: "Code Volume".to_string(),
                    passed: false,
                    message: "Excessive code generated, consider breaking into smaller modules".to_string(),
                    severity: ValidationSeverity::Warning,
                });
                score -= 5.0;
            }
        }

        let is_valid = score >= 70.0;
        let mut recommendations = Vec::new();
        
        if !is_valid {
            recommendations.push("Review and fix compilation errors".to_string());
            if result.artifacts.is_empty() {
                recommendations.push("Ensure code generation produces actual files".to_string());
            }
        }

        Ok(ValidationReport {
            task_id: result.task_id,
            is_valid,
            checks,
            score,
            recommendations,
        })
    }

    async fn validate_testing(&self, result: &AutomationResult) -> Result<ValidationReport> {
        let mut checks = Vec::new();
        let mut score = 100.0;

        // Check if tests were generated
        if let Some(tests_generated) = result.metrics.get("tests_generated") {
            if *tests_generated == 0.0 {
                checks.push(ValidationCheck {
                    name: "Tests Generated".to_string(),
                    passed: false,
                    message: "No tests were generated".to_string(),
                    severity: ValidationSeverity::Error,
                });
                score -= 50.0;
            } else {
                checks.push(ValidationCheck {
                    name: "Tests Generated".to_string(),
                    passed: true,
                    message: format!("{} tests were generated", *tests_generated),
                    severity: ValidationSeverity::Info,
                });
            }
        }

        // Check test results
        let total_tests = result.metrics.get("tests_passed").unwrap_or(&0.0) 
            + result.metrics.get("tests_failed").unwrap_or(&0.0);
        
        if total_tests > 0.0 {
            let pass_rate = result.metrics.get("tests_passed").unwrap_or(&0.0) / total_tests;
            
            if pass_rate < 0.8 {
                checks.push(ValidationCheck {
                    name: "Test Pass Rate".to_string(),
                    passed: false,
                    message: format!("Low pass rate: {:.1}%", pass_rate * 100.0),
                    severity: ValidationSeverity::Error,
                });
                score -= 40.0;
            } else {
                checks.push(ValidationCheck {
                    name: "Test Pass Rate".to_string(),
                    passed: true,
                    message: format!("Good pass rate: {:.1}%", pass_rate * 100.0),
                    severity: ValidationSeverity::Info,
                });
            }
        }

        let is_valid = score >= 70.0;
        let mut recommendations = Vec::new();
        
        if !is_valid {
            recommendations.push("Improve test coverage and fix failing tests".to_string());
        }

        Ok(ValidationReport {
            task_id: result.task_id,
            is_valid,
            checks,
            score,
            recommendations,
        })
    }

    async fn validate_documentation(&self, result: &AutomationResult) -> Result<ValidationReport> {
        let mut checks = Vec::new();
        let mut score = 100.0;

        // Check if documentation files were created
        if result.artifacts.is_empty() {
            checks.push(ValidationCheck {
                name: "Documentation Files".to_string(),
                passed: false,
                message: "No documentation files were created".to_string(),
                severity: ValidationSeverity::Error,
            });
            score -= 50.0;
        } else {
            checks.push(ValidationCheck {
                name: "Documentation Files".to_string(),
                passed: true,
                message: format!("{} documentation files were created", result.artifacts.len()),
                severity: ValidationSeverity::Info,
            });
        }

        // Check if README was created
        let has_readme = result.artifacts.iter().any(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_lowercase() == "readme.md")
                .unwrap_or(false)
        });

        if !has_readme {
            checks.push(ValidationCheck {
                name: "README".to_string(),
                passed: false,
                message: "No README.md file was created".to_string(),
                severity: ValidationSeverity::Warning,
            });
            score -= 20.0;
        } else {
            checks.push(ValidationCheck {
                name: "README".to_string(),
                passed: true,
                message: "README.md file was created".to_string(),
                severity: ValidationSeverity::Info,
            });
        }

        // Check documentation quality
        for artifact in &result.artifacts {
            if let Some(ext) = artifact.extension() {
                if ext == "md" {
                    if let Ok(content) = std::fs::read_to_string(artifact) {
                        let word_count = content.split_whitespace().count();
                        if word_count < 50 {
                            checks.push(ValidationCheck {
                                name: "Documentation Length".to_string(),
                                passed: false,
                                message: format!("Documentation too short: {} words", word_count),
                                severity: ValidationSeverity::Warning,
                            });
                            score -= 10.0;
                        }
                    }
                }
            }
        }

        let is_valid = score >= 70.0;
        let mut recommendations = Vec::new();
        
        if !is_valid {
            recommendations.push("Expand documentation with more detailed explanations".to_string());
            if !has_readme {
                recommendations.push("Create a comprehensive README.md file".to_string());
            }
        }

        Ok(ValidationReport {
            task_id: result.task_id,
            is_valid,
            checks,
            score,
            recommendations,
        })
    }
}

#[async_trait]
impl TaskValidator for DefaultValidator {
    async fn validate(&self, result: &AutomationResult) -> Result<bool> {
        let report = self.get_validation_report(result)?;
        Ok(report.is_valid)
    }

    fn get_validation_report(&self, result: &AutomationResult) -> Result<ValidationReport, anyhow::Error> {
        // For now, we'll use a simplified synchronous validation
        // In a real implementation, this would be async
        match result.status {
            TaskStatus::Completed => {
                // This is a placeholder - in real implementation, we'd need to know the task type
                // For now, assume code generation
                let mut checks = Vec::new();
                let mut score = 100.0;

                if result.artifacts.is_empty() {
                    checks.push(ValidationCheck {
                        name: "Output Files".to_string(),
                        passed: false,
                        message: "No output files generated".to_string(),
                        severity: ValidationSeverity::Error,
                    });
                    score -= 50.0;
                } else {
                    checks.push(ValidationCheck {
                        name: "Output Files".to_string(),
                        passed: true,
                        message: format!("Generated {} files", result.artifacts.len()),
                        severity: ValidationSeverity::Info,
                    });
                }

                Ok(ValidationReport {
                    task_id: result.task_id,
                    is_valid: score >= 70.0,
                    checks,
                    score,
                    recommendations: if score < 70.0 {
                        vec!["Review and improve the output".to_string()]
                    } else {
                        Vec::new()
                    },
                })
            }
            _ => Ok(ValidationReport {
                task_id: result.task_id,
                is_valid: false,
                checks: vec![ValidationCheck {
                    name: "Task Status".to_string(),
                    passed: false,
                    message: "Task did not complete successfully".to_string(),
                    severity: ValidationSeverity::Error,
                }],
                score: 0.0,
                recommendations: vec!["Fix the underlying task execution issue".to_string()],
            }),
        }
    }
}

pub struct StrictValidator {
    config: AutomationConfig,
}

impl StrictValidator {
    pub fn new(config: AutomationConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl TaskValidator for StrictValidator {
    async fn validate(&self, result: &AutomationResult) -> Result<bool> {
        let report = self.get_validation_report(result)?;
        Ok(report.is_valid && report.score >= 90.0)
    }

    fn get_validation_report(&self, result: &AutomationResult) -> Result<ValidationReport, anyhow::Error> {
        let mut checks = Vec::new();
        let mut score = 100.0;

        // Strict validation criteria
        if !result.success {
            checks.push(ValidationCheck {
                name: "Success".to_string(),
                passed: false,
                message: "Task was not successful".to_string(),
                severity: ValidationSeverity::Error,
            });
            score -= 100.0;
        }

        if result.artifacts.is_empty() {
            checks.push(ValidationCheck {
                name: "Artifacts".to_string(),
                passed: false,
                message: "No artifacts produced".to_string(),
                severity: ValidationSeverity::Error,
            });
            score -= 50.0;
        }

        if result.error.is_some() {
            checks.push(ValidationCheck {
                name: "Errors".to_string(),
                passed: false,
                message: "Task had errors".to_string(),
                severity: ValidationSeverity::Error,
            });
            score -= 30.0;
        }

        Ok(ValidationReport {
            task_id: result.task_id,
            is_valid: score >= 90.0,
            checks,
            score,
            recommendations: if score < 90.0 {
                vec![
                    "Fix all errors and warnings".to_string(),
                    "Ensure all requirements are met".to_string(),
                ]
            } else {
                Vec::new()
            },
        })
    }
}
