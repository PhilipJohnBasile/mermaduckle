use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub run_count: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub model: Option<String>,
    pub runs: i64,
    pub success_rate: f64,
    pub avg_latency: i64,
    pub cost_per_run: f64,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRunResponse {
    pub success: bool,
    pub result: Option<WorkflowRunResult>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRunResult {
    #[serde(rename = "runId")]
    pub run_id: String,
    pub status: String,
    pub output: String,
    pub logs: Vec<serde_json::Value>,
}

pub struct Client {
    url: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn list_workflows(&self) -> Result<Vec<Workflow>, String> {
        let res = self.http.get(format!("{}/api/workflows", self.url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn get_workflow(&self, id: &str) -> Result<Workflow, String> {
        let res = self.http.get(format!("{}/api/workflows/{}", self.url, id))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn run_workflow(&self, id: &str) -> Result<WorkflowRunResult, String> {
        let res = self.http.post(format!("{}/api/workflows/{}/run", self.url, id))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let data: WorkflowRunResponse = res.json().await.map_err(|e| e.to_string())?;
        if data.success {
            data.result.ok_or_else(|| "Missing result".into())
        } else {
            Err(data.error.unwrap_or_else(|| "Unknown error".into()))
        }
    }

    pub async fn list_agents(&self) -> Result<Vec<Agent>, String> {
        let res = self.http.get(format!("{}/api/agents", self.url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        res.json().await.map_err(|e| e.to_string())
    }
}
