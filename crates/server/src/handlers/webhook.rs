use actix_web::{get, post, web, HttpRequest, HttpResponse};
use rusqlite::params;
use crate::db::DbPool;

#[post("/api/webhook/{path:.*}")]
pub async fn handle_webhook(pool: web::Data<DbPool>, req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let path = req.match_info().get("path").unwrap_or("");
    let payload = String::from_utf8_lossy(&body).to_string();
    let id = format!("wh_{}", chrono::Utc::now().timestamp_millis());
    let now = chrono::Utc::now().to_rfc3339();

    let conn = pool.get().unwrap();

    // Check for interactive payloads (e.g. Slack/Discord)
    if path.contains("slack/interactive") {
        if let Ok(slack_payload) = serde_json::from_str::<serde_json::Value>(&payload) {
             // Slack interactive payloads are often in a 'payload' form field
             let actual_payload = slack_payload.get("payload")
                 .and_then(|v| v.as_str())
                 .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                 .unwrap_or(slack_payload);

             // Extract run ID and action (e.g. from callback_id or actions array)
             if let Some(run_id) = actual_payload.get("callback_id").and_then(|v| v.as_str()) {
                 let _action = actual_payload.get("actions").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.get("value")).and_then(|v| v.as_str()).unwrap_or("approve");
                 
                 // Trigger the internal approval handler (simulated via DB update + engine execution)
                 // For now, we update the status and the scheduler/approvals UI will handle the rest,
                 // or we can call the handle_approval logic directly if we restructure it.
                 conn.execute("UPDATE workflow_runs SET status = 'approved_externally' WHERE id = ?1", params![run_id]).ok();
                 return HttpResponse::Ok().json(serde_json::json!({"text": "Action received and processed."}));
             }
        }
    }

    // Find workflows with a trigger matching this path
    let mut stmt = conn.prepare("SELECT id FROM workflows WHERE status = 'active' AND nodes LIKE ?1").unwrap();
    let pattern = format!("%{}%", path);
    let workflow_ids: Vec<String> = stmt.query_map(params![pattern], |row| {
        row.get(0)
    }).unwrap().filter_map(|r| r.ok()).collect();

    let status = if workflow_ids.is_empty() { "no_match" } else { "triggered" };

    conn.execute(
        "INSERT INTO webhook_logs (id, path, method, payload, workflow_id, status, created_at) VALUES (?1,?2,'POST',?3,?4,?5,?6)",
        params![id, path, payload, workflow_ids.first().unwrap_or(&String::new()), status, now],
    ).ok();

    HttpResponse::Ok().json(serde_json::json!({
        "received": true,
        "webhookId": id,
        "matchedWorkflows": workflow_ids.len(),
    }))
}

#[get("/api/webhook-logs")]
pub async fn list_webhook_logs(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, path, method, payload, workflow_id, status, response, created_at FROM webhook_logs ORDER BY created_at DESC LIMIT 50").unwrap();

    let logs: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "path": row.get::<_, String>(1)?,
            "method": row.get::<_, String>(2)?,
            "payload": row.get::<_, Option<String>>(3)?,
            "workflowId": row.get::<_, Option<String>>(4)?,
            "status": row.get::<_, Option<String>>(5)?,
            "response": row.get::<_, Option<String>>(6)?,
            "createdAt": row.get::<_, Option<String>>(7)?,
        }))
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(logs)
}
