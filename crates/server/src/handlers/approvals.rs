use actix_web::{get, post, web, HttpResponse};
use rusqlite::params;
use crate::db::{DbPool, add_audit_event};
use crate::models::*;
use mermaduckle_engine::{execute_workflow_engine, Workflow, WorkflowNode, WorkflowEdge};

#[get("/api/approvals")]
pub async fn list_pending_approvals(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, workflow_id, status, started_at, output, logs, context, paused_node_id FROM workflow_runs WHERE status = 'pending_approval' ORDER BY started_at DESC")
        .unwrap();

    let runs: Vec<WorkflowRun> = stmt
        .query_map([], |row| {
            let logs_str: String = row.get::<_, String>(5).unwrap_or_else(|_| "[]".into());
            let context_str: String = row.get::<_, String>(6).unwrap_or_else(|_| "{}".into());
            Ok(WorkflowRun {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                completed_at: None,
                output: row.get(4)?,
                error: None,
                logs: serde_json::from_str(&logs_str).unwrap_or(serde_json::json!([])),
                context: serde_json::from_str(&context_str).unwrap_or(serde_json::json!({})),
                paused_node_id: row.get(7)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(runs)
}

#[post("/api/approvals/{id}/action")]
pub async fn handle_approval(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    body: web::Json<ApprovalActionRequest>,
) -> HttpResponse {
    let run_id = path.into_inner();
    let conn = pool.get().unwrap();

    // 1. Get the run details
    let run_data = conn.query_row(
        "SELECT workflow_id, output, context, paused_node_id FROM workflow_runs WHERE id = ?1 AND status = 'pending_approval'",
        params![run_id],
        |row| {
            let workflow_id: String = row.get(0)?;
            let output: Option<String> = row.get(1)?;
            let context_str: String = row.get(2)?;
            let paused_node_id: String = row.get(3)?;
            Ok((workflow_id, output, context_str, paused_node_id))
        },
    );

    let (workflow_id, _prev_output, context_json, paused_node_id) = match run_data {
        Ok(v) => v,
        Err(_) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Pending run not found"})),
    };

    if body.action == "reject" {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE workflow_runs SET status = 'failed', completed_at = ?1, error = 'Rejected by user' WHERE id = ?2",
            params![now, run_id],
        ).ok();
        return HttpResponse::Ok().json(serde_json::json!({"success": true, "status": "rejected"}));
    }

    // 2. Fetch the workflow to resume
    let wf_data = conn.query_row(
        "SELECT nodes, edges, name FROM workflows WHERE id = ?1",
        params![workflow_id],
        |row| {
            let n: String = row.get(0)?;
            let e: String = row.get(1)?;
            let name: String = row.get(2)?;
            Ok((n, e, name))
        },
    ).unwrap();

    let nodes: Vec<WorkflowNode> = serde_json::from_str(&wf_data.0).unwrap_or_default();
    let edges: Vec<WorkflowEdge> = serde_json::from_str(&wf_data.1).unwrap_or_default();
    let workflow = Workflow { nodes: nodes.clone(), edges: edges.clone() };

    // 3. Find the NEXT node after the paused approval node
    let next_node_id = edges.iter()
        .find(|e| e.source == paused_node_id)
        .map(|e| e.target.clone());

    let Some(next_node) = next_node_id else {
        // No more nodes, just complete it
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE workflow_runs SET status = 'completed', completed_at = ?1 WHERE id = ?2",
            params![now, run_id],
        ).ok();
        return HttpResponse::Ok().json(serde_json::json!({"success": true, "status": "completed"}));
    };

    // 4. Resume execution
    let context_map: std::collections::HashMap<String, String> = serde_json::from_str(&context_json).unwrap_or_default();
    let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    let result = execute_workflow_engine(&workflow, Some(&ollama_url), Some(context_map), Some(&next_node), false).await;

    // 5. Update database
    let completed_at = chrono::Utc::now().to_rfc3339();
    let logs_json = serde_json::to_string(&result.logs).unwrap_or_else(|_| "[]".into());
    let context_json = serde_json::to_string(&result.context).unwrap_or_else(|_| "{}".into());

    conn.execute(
        "UPDATE workflow_runs SET status = ?1, completed_at = ?2, output = ?3, error = ?4, logs = ?5, context = ?6, paused_node_id = ?7 WHERE id = ?8",
        params![
            result.status, 
            if result.status == "completed" || result.status == "failed" { Some(completed_at) } else { None },
            result.output,
            if result.status == "failed" { Some(result.output.clone()) } else { None },
            logs_json,
            context_json,
            result.paused_node_id,
            run_id
        ],
    ).ok();

    add_audit_event(&conn, "workflow_approved", "low", "usr_1", "Sarah Chen", "sarah@mermaduckle.io", "workflow", &workflow_id, &wf_data.2, &serde_json::json!({"runId": run_id}));

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "status": result.status,
        "output": result.output
    }))
}
