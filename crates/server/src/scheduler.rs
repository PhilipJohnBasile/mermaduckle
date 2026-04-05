use crate::db::DbPool;
use std::sync::Arc;

pub fn start_scheduler(pool: Arc<DbPool>) {
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    actix_web::rt::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok(client) = pool.get().await {
                let rows = client
                    .query(
                        "SELECT id, nodes, edges FROM workflows WHERE schedule IS NOT NULL AND status != 'disabled'",
                        &[],
                    )
                    .await
                    .unwrap_or_default();

                for row in &rows {
                    let wf_id: String = row.get(0);
                    let nodes_val: serde_json::Value = row.get(1);
                    let edges_val: serde_json::Value = row.get(2);

                    let nodes: Vec<mermaduckle_engine::WorkflowNode> =
                        serde_json::from_value(nodes_val).unwrap_or_default();
                    let edges: Vec<mermaduckle_engine::WorkflowEdge> =
                        serde_json::from_value(edges_val).unwrap_or_default();

                    let workflow = mermaduckle_engine::Workflow { nodes, edges };
                    let run_id = format!("run_{}", chrono::Utc::now().timestamp_millis());
                    let started_at = chrono::Utc::now().to_rfc3339();
                    let empty_logs = serde_json::json!([]);
                    let empty_ctx = serde_json::json!({});

                    client.execute(
                        "INSERT INTO workflow_runs (id, workflow_id, status, started_at, logs, context) VALUES ($1,$2,'running',$3,$4,$5)",
                        &[&run_id, &wf_id, &started_at, &empty_logs, &empty_ctx],
                    ).await.ok();

                    let result = mermaduckle_engine::execute_workflow_engine(
                        &workflow,
                        Some(&ollama_url),
                        None,
                        None,
                        false,
                    )
                    .await;

                    let completed_at = chrono::Utc::now().to_rfc3339();
                    let logs_json: serde_json::Value =
                        serde_json::to_value(&result.logs).unwrap_or(serde_json::json!([]));
                    let context_json: serde_json::Value =
                        serde_json::to_value(&result.context).unwrap_or(serde_json::json!({}));
                    let completed_opt: Option<String> = Some(completed_at.clone());
                    let error_opt: Option<String> = if result.status == "failed" {
                        Some(result.output.clone())
                    } else {
                        None
                    };

                    client.execute(
                        "UPDATE workflow_runs SET status = $1, completed_at = $2, output = $3, error = $4, logs = $5, context = $6 WHERE id = $7",
                        &[&result.status, &completed_opt, &result.output, &error_opt, &logs_json, &context_json, &run_id],
                    ).await.ok();

                    client.execute(
                        "UPDATE workflows SET run_count = run_count + 1, last_run_at = $1 WHERE id = $2",
                        &[&completed_at, &wf_id],
                    ).await.ok();
                }
            }
        }
    });
}
