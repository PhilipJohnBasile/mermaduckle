use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Allow,
    Deny,
    Flag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub policy_id: String,
    pub passed: bool,
    pub action: PolicyAction,
}

pub type PolicyContext = HashMap<String, serde_json::Value>;

// ── Governance Engine ──────────────────────────────────────

pub struct GovernanceEngine {
    rate_limit_store: HashMap<String, RateLimitRecord>,
}

struct RateLimitRecord {
    count: u64,
    reset_time: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u64,
}

#[derive(Debug)]
pub struct CostLimitResult {
    pub allowed: bool,
    pub overage: f64,
}

#[derive(Debug)]
pub struct ContentFilterResult {
    pub flagged: bool,
    pub reason: Option<String>,
}

impl GovernanceEngine {
    pub fn new() -> Self {
        Self {
            rate_limit_store: HashMap::new(),
        }
    }

    pub fn check_rate_limit(
        &mut self,
        key: &str,
        limit: u64,
        window_ms: u64,
    ) -> RateLimitResult {
        let now = now_ms();
        let record = self.rate_limit_store.get(key);

        if record.is_none() || record.is_some_and(|r| now >= r.reset_time) {
            self.rate_limit_store.insert(
                key.to_string(),
                RateLimitRecord {
                    count: 1,
                    reset_time: now + window_ms,
                },
            );
            return RateLimitResult {
                allowed: true,
                remaining: limit - 1,
            };
        }

        let record = self.rate_limit_store.get_mut(key).unwrap();

        if record.count >= limit {
            return RateLimitResult {
                allowed: false,
                remaining: 0,
            };
        }

        record.count += 1;
        RateLimitResult {
            allowed: true,
            remaining: limit - record.count,
        }
    }

    pub fn check_cost_limit(&self, current_cost: f64, limit: f64) -> CostLimitResult {
        let overage = if current_cost > limit {
            current_cost - limit
        } else {
            0.0
        };
        CostLimitResult {
            allowed: overage == 0.0,
            overage,
        }
    }

    pub fn check_content_filter(&self, text: &str) -> ContentFilterResult {
        let bad_words = regex::Regex::new(r"(?i)\b(spam|abuse|hate|violence)\b").unwrap();
        let matches: Vec<&str> = bad_words
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();

        if matches.is_empty() {
            ContentFilterResult {
                flagged: false,
                reason: None,
            }
        } else {
            let mut unique: Vec<String> = matches.iter().map(|s| s.to_lowercase()).collect();
            unique.sort();
            unique.dedup();
            ContentFilterResult {
                flagged: true,
                reason: Some(format!(
                    "Contains forbidden terms: {}",
                    unique.join(", ")
                )),
            }
        }
    }

    pub fn check_content_length(&self, text: &str, max_length: usize) -> bool {
        text.len() <= max_length
    }
}

impl Default for GovernanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Policy Evaluation ──────────────────────────────────────

/// Evaluate a list of policies against a context.
/// Each policy evaluates to passed=true if the context satisfies its conditions.
/// This is a simple key-existence check; in production, you'd use a rule engine.
pub fn evaluate_policies(
    policies: &[Policy],
    _context: &PolicyContext,
) -> Vec<PolicyResult> {
    policies
        .iter()
        .map(|policy| PolicyResult {
            policy_id: policy.id.clone(),
            passed: true, // default pass – would be replaced by condition evaluation
            action: policy.action.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_within_limit() {
        let mut engine = GovernanceEngine::new();
        let result = engine.check_rate_limit("test_key", 10, 60_000);
        assert!(result.allowed);
        assert_eq!(result.remaining, 9);
    }

    #[test]
    fn test_rate_limit_blocks_over_limit() {
        let mut engine = GovernanceEngine::new();
        for _ in 0..10 {
            engine.check_rate_limit("test_key", 10, 60_000);
        }
        let result = engine.check_rate_limit("test_key", 10, 60_000);
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
    }

    #[test]
    fn test_cost_limit_within() {
        let engine = GovernanceEngine::new();
        let result = engine.check_cost_limit(50.0, 100.0);
        assert!(result.allowed);
        assert_eq!(result.overage, 0.0);
    }

    #[test]
    fn test_cost_limit_exceeded() {
        let engine = GovernanceEngine::new();
        let result = engine.check_cost_limit(150.0, 100.0);
        assert!(!result.allowed);
        assert_eq!(result.overage, 50.0);
    }

    #[test]
    fn test_content_filter_clean() {
        let engine = GovernanceEngine::new();
        let result = engine.check_content_filter("This is a perfectly fine message");
        assert!(!result.flagged);
    }

    #[test]
    fn test_content_filter_flagged() {
        let engine = GovernanceEngine::new();
        let result = engine.check_content_filter("This contains spam and abuse");
        assert!(result.flagged);
        assert!(result
            .reason
            .unwrap()
            .contains("spam"));
    }

    #[test]
    fn test_evaluate_policies() {
        let policies = vec![Policy {
            id: "p1".into(),
            name: "Test Policy".into(),
            action: PolicyAction::Allow,
        }];
        let context = PolicyContext::new();
        let results = evaluate_policies(&policies, &context);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[test]
    fn test_content_length_within() {
        let engine = GovernanceEngine::new();
        assert!(engine.check_content_length("short", 10));
    }

    #[test]
    fn test_content_length_exceeded() {
        let engine = GovernanceEngine::new();
        assert!(!engine.check_content_length("this is a longer message", 10));
    }
}
