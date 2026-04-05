use serde::{Deserialize, Serialize};

// ── Workflow ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub nodes: serde_json::Value,
    #[serde(default)]
    pub edges: serde_json::Value,
    #[serde(default)]
    pub run_count: i64,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_status() -> String {
    "draft".into()
}

// ── Agent ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_agent_type")]
    #[serde(rename = "type")]
    pub agent_type: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub tools: serde_json::Value,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub runs: i64,
    #[serde(default, rename = "successRate")]
    pub success_rate: f64,
    #[serde(default, rename = "avgLatency")]
    pub avg_latency: i64,
    #[serde(default, rename = "costPerRun")]
    pub cost_per_run: f64,
    #[serde(default)]
    pub tags: serde_json::Value,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn default_agent_type() -> String {
    "llm".into()
}

// ── Audit Event ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub severity: String,
    pub actor: AuditActor,
    pub target: AuditTarget,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTarget {
    #[serde(rename = "type")]
    pub target_type: String,
    pub id: String,
    pub name: String,
}

// ── Workflow Run ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: String,
    #[serde(rename = "workflowId")]
    pub workflow_id: String,
    pub status: String,
    #[serde(rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub logs: serde_json::Value,
    #[serde(default)]
    pub context: serde_json::Value,
    #[serde(rename = "pausedNodeId")]
    pub paused_node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalActionRequest {
    pub action: String, // "approve" or "reject"
}

// ── Settings Models ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing)]
    #[serde(rename = "keyHash")]
    pub key_hash: String,
    pub scopes: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "lastUsed")]
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub status: String,
    #[serde(rename = "joinedAt")]
    pub joined_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub id: String,
    pub provider: String,
    pub config: serde_json::Value,
    pub status: String,
    #[serde(rename = "connectedAt")]
    pub connected_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub id: String,
    #[serde(rename = "emailAlerts")]
    pub email_alerts: i64,
    #[serde(rename = "slackWebhook")]
    pub slack_webhook: Option<String>,
    #[serde(rename = "digestFrequency")]
    pub digest_frequency: String,
    #[serde(rename = "alertSeverity")]
    pub alert_severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub id: String,
    pub key: String,
    pub value: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

// ── Dashboard ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    #[serde(rename = "totalWorkflows")]
    pub total_workflows: i64,
    #[serde(rename = "activeWorkflows")]
    pub active_workflows: i64,
    #[serde(rename = "totalRuns")]
    pub total_runs: i64,
    #[serde(rename = "successfulRuns")]
    pub successful_runs: i64,
    #[serde(rename = "failedRuns")]
    pub failed_runs: i64,
    #[serde(rename = "pendingApprovals")]
    pub pending_approvals: i64,
    #[serde(rename = "avgExecutionTime")]
    pub avg_execution_time: f64,
    #[serde(rename = "totalCost")]
    pub total_cost: f64,
    #[serde(rename = "topWorkflows")]
    pub top_workflows: Vec<TopWorkflow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopWorkflow {
    pub id: String,
    pub name: String,
    pub runs: i64,
    #[serde(rename = "successRate")]
    pub success_rate: f64,
}

// ── Health ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub services: HealthServices,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthServices {
    pub database: String,
    pub ollama: String,
    pub ollama_required: bool,
}

// ── Request Bodies ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub nodes: Option<serde_json::Value>,
    #[serde(default)]
    pub edges: Option<serde_json::Value>,
    #[serde(default)]
    pub schedule: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub nodes: Option<serde_json::Value>,
    #[serde(default)]
    pub edges: Option<serde_json::Value>,
    #[serde(default)]
    pub schedule: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "type")]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub runs: Option<i64>,
    #[serde(default, rename = "successRate")]
    pub success_rate: Option<f64>,
    #[serde(default, rename = "avgLatency")]
    pub avg_latency: Option<i64>,
    #[serde(default, rename = "costPerRun")]
    pub cost_per_run: Option<f64>,
    #[serde(default)]
    pub tags: Option<serde_json::Value>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamMemberRequest {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIntegrationRequest {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationsRequest {
    #[serde(default, rename = "emailAlerts")]
    pub email_alerts: Option<i64>,
    #[serde(default, rename = "slackWebhook")]
    pub slack_webhook: Option<String>,
    #[serde(default, rename = "digestFrequency")]
    pub digest_frequency: Option<String>,
    #[serde(default, rename = "alertSeverity")]
    pub alert_severity: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSecretRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct ArchitectRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchitectResponse {
    pub nodes: serde_json::Value,
    pub edges: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub workflows: Vec<serde_json::Value>,
    pub agents: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct WebhookLogEntry {
    pub id: String,
    pub path: String,
    pub method: String,
    pub payload: Option<String>,
    pub workflow_id: Option<String>,
    pub status: Option<String>,
    pub response: Option<String>,
    pub created_at: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct HealRequest {
    pub run_id: String,
    pub node_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealResponse {
    pub suggestion: String,
    pub patched_config: serde_json::Value,
}
