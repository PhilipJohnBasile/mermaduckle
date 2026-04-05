use crate::db::DbPool;
use crate::models::{HealRequest, HealResponse};
use actix_web::{HttpResponse, post, web};
use mermaduckle_engine::call_ollama;
use rusqlite::params;

#[post("/api/recovery/heal")]
pub async fn self_heal_node(pool: web::Data<DbPool>, body: web::Json<HealRequest>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let run_id = &body.run_id;
    let node_id = &body.node_id;

    // 1. Get run logs and workflow configuration
    let (logs_json, workflow_id) = match conn.query_row(
        "SELECT logs, workflow_id FROM workflow_runs WHERE id = ?1",
        params![run_id],
        |row| {
            let logs: String = row.get(0)?;
            let wid: String = row.get(1)?;
            Ok((logs, wid))
        },
    ) {
        Ok(v) => v,
        Err(_) => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Run not found"}));
        }
    };

    let (nodes_json, _edges_json) = match conn.query_row(
        "SELECT nodes, edges FROM workflows WHERE id = ?1",
        params![workflow_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ) {
        Ok(v) => v,
        Err(_) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Workflow not found"}));
        }
    };

    let nodes: Vec<serde_json::Value> = serde_json::from_str(&nodes_json).unwrap_or_default();
    let target_node = nodes
        .iter()
        .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(node_id));

    if target_node.is_none() {
        return HttpResponse::NotFound()
            .json(serde_json::json!({"error": "Node not found in workflow"}));
    }

    // 2. Build prompt for Self-Healing AI
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let sys_prompt = "You are the Mermaduckle Self-Healing Oracle. 
    Analyze the provided execution logs and the current node configuration. 
    Suggest a 'patched_config' (JSON) that resolves the error and provide a 'suggestion' (text) explaining why.
    Output ONLY valid JSON.
    Example output format: {\"suggestion\": \"Increase timeout\", \"patched_config\": {\"timeout\": \"60s\"}}";

    let context = format!(
        "
        Node Type: {}
        Current Config: {}
        Logs: {}
    ",
        target_node
            .unwrap()
            .get("type")
            .unwrap_or(&serde_json::json!("unknown")),
        target_node
            .unwrap()
            .get("config")
            .unwrap_or(&serde_json::json!("{}")),
        logs_json
    );

    let prompt = format!("{} \n\n Analysis Context: {}", sys_prompt, context);

    match call_ollama(&ollama_url, "llama3.2", prompt).await {
        Ok(response) => {
            // Extract JSON from potential markdown tags
            let clean_json = response
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim();
            if let Ok(res) = serde_json::from_str::<HealResponse>(clean_json) {
                HttpResponse::Ok().json(res)
            } else {
                HttpResponse::InternalServerError().json(serde_json::json!({"error": "AI suggested fix was unparseable", "raw": response}))
            }
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": format!("Self-healing connection failed: {}", e)})),
    }
}
