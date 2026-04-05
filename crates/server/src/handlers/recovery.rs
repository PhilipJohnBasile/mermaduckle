use crate::db::DbPool;
use crate::models::{HealRequest, HealResponse};
use actix_web::{HttpResponse, post, web};
use mermaduckle_engine::call_ollama;

#[post("/recovery/heal")]
pub async fn self_heal_node(pool: web::Data<DbPool>, body: web::Json<HealRequest>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let run_id = &body.run_id;
    let node_id = &body.node_id;

    // 1. Get run logs and workflow configuration
    let run_row = client
        .query_opt(
            "SELECT logs, workflow_id FROM workflow_runs WHERE id = $1",
            &[run_id],
        )
        .await;

    let (logs_val, workflow_id): (serde_json::Value, String) = match run_row {
        Ok(Some(row)) => (row.get(0), row.get(1)),
        _ => {
            return HttpResponse::NotFound().json(serde_json::json!({"error": "Run not found"}));
        }
    };

    let wf_row = client
        .query_opt(
            "SELECT nodes, edges FROM workflows WHERE id = $1",
            &[&workflow_id],
        )
        .await;

    let nodes_val: serde_json::Value = match wf_row {
        Ok(Some(row)) => row.get(0),
        _ => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Workflow not found"}));
        }
    };

    let nodes: Vec<serde_json::Value> = serde_json::from_value(nodes_val).unwrap_or_default();
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
        logs_val
    );

    let prompt = format!("{} \n\n Analysis Context: {}", sys_prompt, context);

    match call_ollama(&ollama_url, "llama3.2", prompt).await {
        Ok(response) => {
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
