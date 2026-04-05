use crate::db::DbPool;
use actix_web::{HttpRequest, HttpResponse, get, post, web};

#[post("/webhook/{path:.*}")]
pub async fn handle_webhook(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Bytes,
) -> HttpResponse {
    let path = req.match_info().get("path").unwrap_or("");
    let payload = String::from_utf8_lossy(&body).to_string();
    let id = format!("wh_{}", chrono::Utc::now().timestamp_millis());
    let now = chrono::Utc::now().to_rfc3339();

    let client = pool.get().await.unwrap();

    // Check for interactive payloads (e.g. Slack/Discord)
    if path.contains("slack/interactive") {
        if let Ok(slack_payload) = serde_json::from_str::<serde_json::Value>(&payload) {
            let actual_payload = slack_payload
                .get("payload")
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .unwrap_or(slack_payload);

            if let Some(run_id) = actual_payload.get("callback_id").and_then(|v| v.as_str()) {
                client
                    .execute(
                        "UPDATE workflow_runs SET status = 'approved_externally' WHERE id = $1",
                        &[&run_id],
                    )
                    .await
                    .ok();
                return HttpResponse::Ok()
                    .json(serde_json::json!({"text": "Action received and processed."}));
            }
        }
    }

    // Find workflows with a trigger matching this path
    let pattern = format!("%{}%", path);
    let rows = client
        .query(
            "SELECT id FROM workflows WHERE status = 'active' AND nodes::text LIKE $1",
            &[&pattern],
        )
        .await
        .unwrap_or_default();

    let workflow_ids: Vec<String> = rows.iter().map(|row| row.get(0)).collect();

    let status = if workflow_ids.is_empty() {
        "no_match"
    } else {
        "triggered"
    };

    let wf_id = workflow_ids.first().cloned().unwrap_or_default();
    client.execute(
        "INSERT INTO webhook_logs (id, path, method, payload, workflow_id, status, created_at) VALUES ($1,$2,'POST',$3,$4,$5,$6)",
        &[&id, &path, &payload, &wf_id, &status, &now],
    ).await.ok();

    HttpResponse::Ok().json(serde_json::json!({
        "received": true,
        "webhookId": id,
        "matchedWorkflows": workflow_ids.len(),
    }))
}

#[get("/webhook-logs")]
pub async fn list_webhook_logs(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, path, method, payload, workflow_id, status, response, created_at FROM webhook_logs ORDER BY created_at DESC LIMIT 50", &[])
        .await
        .unwrap_or_default();

    let logs: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<_, String>(0),
                "path": row.get::<_, String>(1),
                "method": row.get::<_, String>(2),
                "payload": row.get::<_, Option<String>>(3),
                "workflowId": row.get::<_, Option<String>>(4),
                "status": row.get::<_, Option<String>>(5),
                "response": row.get::<_, Option<String>>(6),
                "createdAt": row.get::<_, Option<String>>(7),
            })
        })
        .collect();

    HttpResponse::Ok().json(logs)
}
