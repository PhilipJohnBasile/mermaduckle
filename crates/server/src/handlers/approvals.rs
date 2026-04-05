use crate::db::{DbPool, add_audit_event};
use crate::models::*;
use actix_web::{HttpResponse, get, post, web};
use mermaduckle_engine::{Workflow, WorkflowEdge, WorkflowNode, execute_workflow_engine};

#[get("/api/approvals")]
pub async fn list_pending_approvals(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, workflow_id, status, started_at, output, logs, context, paused_node_id FROM workflow_runs WHERE status = 'pending_approval' ORDER BY started_at DESC", &[])
        .await
        .unwrap_or_default();

    let runs: Vec<WorkflowRun> = rows
        .iter()
        .map(|row| WorkflowRun {
            id: row.get(0),
            workflow_id: row.get(1),
            status: row.get(2),
            started_at: row.get(3),
            completed_at: None,
            output: row.get(4),
            error: None,
            logs: row.get::<_, serde_json::Value>(5),
            context: row.get::<_, serde_json::Value>(6),
            paused_node_id: row.get(7),
        })
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
    let client = pool.get().await.unwrap();

    let run_data = client
        .query_opt(
            "SELECT workflow_id, output, context, paused_node_id FROM workflow_runs WHERE id = $1 AND status = 'pending_approval'",
            &[&run_id],
        )
        .await;

    let row = match run_data {
        Ok(Some(r)) => r,
        _ => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Pending run not found"}));
        }
    };

    let workflow_id: String = row.get(0);
    let context_val: serde_json::Value = row.get(2);
    let paused_node_id: String = row.get(3);

    if body.action == "reject" {
        let now = chrono::Utc::now().to_rfc3339();
        client.execute(
            "UPDATE workflow_runs SET status = 'failed', completed_at = $1, error = 'Rejected by user' WHERE id = $2",
            &[&now, &run_id],
        ).await.ok();
        return HttpResponse::Ok().json(serde_json::json!({"success": true, "status": "rejected"}));
    }

    // Fetch the workflow to resume
    let wf_row = client
        .query_opt(
            "SELECT nodes, edges, name FROM workflows WHERE id = $1",
            &[&workflow_id],
        )
        .await;

    let wf_row = match wf_row {
        Ok(Some(r)) => r,
        _ => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Workflow not found"}));
        }
    };

    let nodes_val: serde_json::Value = wf_row.get(0);
    let edges_val: serde_json::Value = wf_row.get(1);
    let wf_name: String = wf_row.get(2);

    let nodes: Vec<WorkflowNode> = serde_json::from_value(nodes_val).unwrap_or_default();
    let edges: Vec<WorkflowEdge> = serde_json::from_value(edges_val).unwrap_or_default();
    let workflow = Workflow {
        nodes: nodes.clone(),
        edges: edges.clone(),
    };

    let next_node_id = edges
        .iter()
        .find(|e| e.source == paused_node_id)
        .map(|e| e.target.clone());

    let Some(next_node) = next_node_id else {
        let now = chrono::Utc::now().to_rfc3339();
        client.execute(
            "UPDATE workflow_runs SET status = 'completed', completed_at = $1 WHERE id = $2",
            &[&now, &run_id],
        ).await.ok();
        return HttpResponse::Ok()
            .json(serde_json::json!({"success": true, "status": "completed"}));
    };

    let context_map: std::collections::HashMap<String, String> =
        serde_json::from_value(context_val).unwrap_or_default();
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    let result = execute_workflow_engine(
        &workflow,
        Some(&ollama_url),
        Some(context_map),
        Some(&next_node),
        false,
    )
    .await;

    let completed_at = chrono::Utc::now().to_rfc3339();
    let logs_json: serde_json::Value = serde_json::to_value(&result.logs).unwrap_or(serde_json::json!([]));
    let context_json: serde_json::Value = serde_json::to_value(&result.context).unwrap_or(serde_json::json!({}));
    let completed_at_opt: Option<String> = if result.status == "completed" || result.status == "failed" {
        Some(completed_at)
    } else {
        None
    };
    let error_opt: Option<String> = if result.status == "failed" {
        Some(result.output.clone())
    } else {
        None
    };

    client.execute(
        "UPDATE workflow_runs SET status = $1, completed_at = $2, output = $3, error = $4, logs = $5, context = $6, paused_node_id = $7 WHERE id = $8",
        &[&result.status, &completed_at_opt, &result.output, &error_opt, &logs_json, &context_json, &result.paused_node_id, &run_id],
    ).await.ok();

    add_audit_event(
        &client,
        "workflow_approved",
        "low",
        "usr_1",
        "Sarah Chen",
        "sarah@mermaduckle.io",
        "workflow",
        &workflow_id,
        &wf_name,
        &serde_json::json!({"runId": run_id}),
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "status": result.status,
        "output": result.output
    }))
}
