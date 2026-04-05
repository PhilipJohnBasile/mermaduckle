use crate::db::{DbPool, add_audit_event};
use crate::models::*;
use actix_web::{HttpResponse, delete, get, post, put, web};

#[get("/api/workflows")]
pub async fn list_workflows(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, name, description, status, nodes, edges, run_count, last_run_at, schedule, created_at, updated_at FROM workflows ORDER BY updated_at DESC", &[])
        .await
        .unwrap_or_default();

    let workflows: Vec<Workflow> = rows
        .iter()
        .map(|row| Workflow {
            id: row.get(0),
            name: row.get(1),
            description: row.get(2),
            status: row.get(3),
            nodes: row.get::<_, serde_json::Value>(4),
            edges: row.get::<_, serde_json::Value>(5),
            run_count: row.get(6),
            last_run_at: row.get(7),
            schedule: row.get(8),
            created_at: row.get(9),
            updated_at: row.get(10),
        })
        .collect();

    HttpResponse::Ok().json(workflows)
}

#[get("/api/workflows/{id}")]
pub async fn get_workflow(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();

    match client
        .query_opt(
            "SELECT id, name, description, status, nodes, edges, run_count, last_run_at, schedule, created_at, updated_at FROM workflows WHERE id = $1",
            &[&id],
        )
        .await
    {
        Ok(Some(row)) => {
            let wf = Workflow {
                id: row.get(0),
                name: row.get(1),
                description: row.get(2),
                status: row.get(3),
                nodes: row.get::<_, serde_json::Value>(4),
                edges: row.get::<_, serde_json::Value>(5),
                run_count: row.get(6),
                last_run_at: row.get(7),
                schedule: row.get(8),
                created_at: row.get(9),
                updated_at: row.get(10),
            };
            HttpResponse::Ok().json(wf)
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({"error": "Workflow not found"})),
    }
}

#[post("/api/workflows")]
pub async fn create_workflow(
    pool: web::Data<DbPool>,
    body: web::Json<CreateWorkflowRequest>,
) -> HttpResponse {
    let id = format!(
        "wf_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    let now = chrono::Utc::now().to_rfc3339();
    let nodes: serde_json::Value = body.nodes.clone().unwrap_or(serde_json::json!([]));
    let edges: serde_json::Value = body.edges.clone().unwrap_or(serde_json::json!([]));
    let status = body.status.clone().unwrap_or_else(|| "draft".into());
    let schedule = body.schedule.clone();
    let run_count: i64 = 0;

    let client = pool.get().await.unwrap();
    client.execute(
        "INSERT INTO workflows (id, name, description, status, nodes, edges, schedule, run_count, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        &[&id, &body.name, &body.description, &status, &nodes, &edges, &schedule, &run_count, &now, &now],
    ).await.ok();

    add_audit_event(
        &client,
        "workflow_created",
        "low",
        "usr_1",
        "System",
        "system@mermaduckle.io",
        "workflow",
        &id,
        &body.name,
        &serde_json::json!({}),
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({"id": id, "name": body.name}))
}

#[put("/api/workflows/{id}")]
pub async fn update_workflow(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    body: web::Json<UpdateWorkflowRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let now = chrono::Utc::now().to_rfc3339();
    let client = pool.get().await.unwrap();

    // Build dynamic UPDATE with numbered params
    let mut sets: Vec<String> = vec!["updated_at = $1".into()];
    let mut values: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> =
        vec![Box::new(now.clone())];
    let mut idx = 2u32;

    if let Some(ref name) = body.name {
        sets.push(format!("name = ${idx}"));
        values.push(Box::new(name.clone()));
        idx += 1;
    }
    if let Some(ref desc) = body.description {
        sets.push(format!("description = ${idx}"));
        values.push(Box::new(desc.clone()));
        idx += 1;
    }
    if let Some(ref status) = body.status {
        sets.push(format!("status = ${idx}"));
        values.push(Box::new(status.clone()));
        idx += 1;
    }
    if let Some(ref nodes) = body.nodes {
        sets.push(format!("nodes = ${idx}"));
        values.push(Box::new(nodes.clone()));
        idx += 1;
    }
    if let Some(ref edges) = body.edges {
        sets.push(format!("edges = ${idx}"));
        values.push(Box::new(edges.clone()));
        idx += 1;
    }
    if let Some(ref schedule) = body.schedule {
        sets.push(format!("schedule = ${idx}"));
        values.push(Box::new(schedule.clone()));
        idx += 1;
    }

    let sql = format!(
        "UPDATE workflows SET {} WHERE id = ${idx}",
        sets.join(", ")
    );
    values.push(Box::new(id.clone()));

    let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
        values.iter().map(|v| v.as_ref()).collect();
    client.execute(&sql, &params_refs).await.ok();

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[delete("/api/workflows/{id}")]
pub async fn delete_workflow(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    client
        .execute("DELETE FROM workflow_runs WHERE workflow_id = $1", &[&id])
        .await
        .ok();
    client
        .execute("DELETE FROM workflows WHERE id = $1", &[&id])
        .await
        .ok();
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[post("/api/workflows/{id}/run")]
pub async fn run_workflow(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let id = path.into_inner();
    let debug_mode = body.get("debug").and_then(|v| v.as_bool()).unwrap_or(false);
    let client = pool.get().await.unwrap();

    // Load workflow
    let result = client
        .query_opt(
            "SELECT nodes, edges, name FROM workflows WHERE id = $1",
            &[&id],
        )
        .await;

    let row = match result {
        Ok(Some(r)) => r,
        _ => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Workflow not found"}));
        }
    };

    let nodes_val: serde_json::Value = row.get(0);
    let edges_val: serde_json::Value = row.get(1);
    let name: String = row.get(2);

    let nodes: Vec<mermaduckle_engine::WorkflowNode> =
        serde_json::from_value(nodes_val).unwrap_or_default();
    let edges: Vec<mermaduckle_engine::WorkflowEdge> =
        serde_json::from_value(edges_val).unwrap_or_default();

    let workflow = mermaduckle_engine::Workflow { nodes, edges };
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    // Create run record
    let run_id = format!("run_{}", chrono::Utc::now().timestamp_millis());
    let started_at = chrono::Utc::now().to_rfc3339();
    let empty_logs = serde_json::json!([]);
    let empty_ctx = serde_json::json!({});
    client.execute(
        "INSERT INTO workflow_runs (id, workflow_id, status, started_at, logs, context) VALUES ($1,$2,'running',$3,$4,$5)",
        &[&run_id, &id, &started_at, &empty_logs, &empty_ctx],
    ).await.ok();

    // Execute
    let result = mermaduckle_engine::execute_workflow_engine(
        &workflow,
        Some(&ollama_url),
        None,
        None,
        debug_mode,
    )
    .await;

    let completed_at = chrono::Utc::now().to_rfc3339();
    let status = &result.status;
    let logs_json: serde_json::Value =
        serde_json::to_value(&result.logs).unwrap_or(serde_json::json!([]));
    let context_json: serde_json::Value =
        serde_json::to_value(&result.context).unwrap_or(serde_json::json!({}));
    let completed_at_opt: Option<String> =
        if status == "completed" || status == "failed" {
            Some(completed_at.clone())
        } else {
            None
        };
    let error_opt: Option<String> = if status == "failed" {
        Some(result.output.clone())
    } else {
        None
    };

    client.execute(
        "UPDATE workflow_runs SET status = $1, completed_at = $2, output = $3, error = $4, logs = $5, context = $6, paused_node_id = $7 WHERE id = $8",
        &[status, &completed_at_opt, &result.output, &error_opt, &logs_json, &context_json, &result.paused_node_id, &run_id],
    ).await.ok();

    client.execute(
        "UPDATE workflows SET run_count = run_count + 1, last_run_at = $1, status = 'active' WHERE id = $2",
        &[&completed_at, &id],
    ).await.ok();

    let severity = if status == "failed" { "high" } else { "low" };
    add_audit_event(
        &client,
        "workflow_run",
        severity,
        "usr_1",
        "System",
        "system@mermaduckle.io",
        "workflow",
        &id,
        &name,
        &serde_json::json!({"runId": run_id}),
    )
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "success": status == "completed" || status == "pending_approval",
        "result": {
            "runId": run_id,
            "status": status,
            "output": result.output,
            "logs": result.logs,
        }
    }))
}

#[get("/api/workflows/{id}/runs")]
pub async fn get_workflow_runs(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, workflow_id, status, started_at, completed_at, output, error, logs, context, paused_node_id FROM workflow_runs WHERE workflow_id = $1 ORDER BY started_at DESC", &[&id])
        .await
        .unwrap_or_default();

    let runs: Vec<WorkflowRun> = rows
        .iter()
        .map(|row| WorkflowRun {
            id: row.get(0),
            workflow_id: row.get(1),
            status: row.get(2),
            started_at: row.get(3),
            completed_at: row.get(4),
            output: row.get(5),
            error: row.get(6),
            logs: row.get::<_, serde_json::Value>(7),
            context: row.get::<_, serde_json::Value>(8),
            paused_node_id: row.get(9),
        })
        .collect();

    HttpResponse::Ok().json(runs)
}

#[get("/api/workflows/export")]
pub async fn export_workflows(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();

    let wf_rows = client
        .query("SELECT id, name, description, status, nodes, edges, run_count FROM workflows", &[])
        .await
        .unwrap_or_default();
    let workflows: Vec<serde_json::Value> = wf_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<_, String>(0),
                "name": row.get::<_, String>(1),
                "description": row.get::<_, Option<String>>(2),
                "status": row.get::<_, String>(3),
                "nodes": row.get::<_, serde_json::Value>(4),
                "edges": row.get::<_, serde_json::Value>(5),
                "run_count": row.get::<_, i64>(6),
            })
        })
        .collect();

    let ag_rows = client
        .query("SELECT id, name, description, type, model, runs, success_rate, avg_latency, cost_per_run, tags FROM agents", &[])
        .await
        .unwrap_or_default();
    let agents: Vec<serde_json::Value> = ag_rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<_, String>(0),
                "name": row.get::<_, String>(1),
                "description": row.get::<_, Option<String>>(2),
                "type": row.get::<_, String>(3),
                "model": row.get::<_, Option<String>>(4),
                "runs": row.get::<_, i64>(5),
                "successRate": row.get::<_, f64>(6),
                "avgLatency": row.get::<_, i64>(7),
                "costPerRun": row.get::<_, f64>(8),
                "tags": row.get::<_, serde_json::Value>(9),
            })
        })
        .collect();

    HttpResponse::Ok().json(serde_json::json!({ "workflows": workflows, "agents": agents }))
}

#[post("/api/workflows/import")]
pub async fn import_workflows(
    pool: web::Data<DbPool>,
    body: web::Json<ImportRequest>,
) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    for wf in &body.workflows {
        let id = wf.get("id").and_then(|v| v.as_str()).unwrap_or("imported");
        let name = wf
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported");
        let desc = wf.get("description").and_then(|v| v.as_str());
        let nodes_val: serde_json::Value = wf.get("nodes").cloned().unwrap_or(serde_json::json!([]));
        let edges_val: serde_json::Value = wf.get("edges").cloned().unwrap_or(serde_json::json!([]));
        client.execute(
            "INSERT INTO workflows (id, name, description, status, nodes, edges, created_at, updated_at) VALUES ($1,$2,$3,'draft',$4,$5,$6,$7) ON CONFLICT (id) DO UPDATE SET name=$2, description=$3, nodes=$4, edges=$5, updated_at=$7",
            &[&id, &name, &desc, &nodes_val, &edges_val, &now, &now],
        ).await.ok();
    }

    for ag in &body.agents {
        let id = ag.get("id").and_then(|v| v.as_str()).unwrap_or("imported");
        let name = ag
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported");
        let desc = ag.get("description").and_then(|v| v.as_str());
        let model = ag.get("model").and_then(|v| v.as_str());
        client.execute(
            "INSERT INTO agents (id, name, description, model, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO UPDATE SET name=$2, description=$3, model=$4, updated_at=$6",
            &[&id, &name, &desc, &model, &now, &now],
        ).await.ok();
    }

    HttpResponse::Ok().json(serde_json::json!({"success": true, "workflows": body.workflows.len(), "agents": body.agents.len()}))
}

#[get("/api/workflows/{id}/export")]
pub async fn export_workflow(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let client = pool.get().await.unwrap();
    let result = client
        .query_opt(
            "SELECT name, description, nodes, edges FROM workflows WHERE id = $1",
            &[&id],
        )
        .await;

    match result {
        Ok(Some(row)) => {
            let val = serde_json::json!({
                "name": row.get::<_, String>(0),
                "description": row.get::<_, Option<String>>(1),
                "nodes": row.get::<_, serde_json::Value>(2),
                "edges": row.get::<_, serde_json::Value>(3),
            });
            HttpResponse::Ok().json(val)
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({"error": "Workflow not found"})),
    }
}

#[post("/api/workflows/import-single")]
pub async fn import_workflow(
    pool: web::Data<DbPool>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let id = format!("wf_{}", chrono::Utc::now().timestamp_millis());
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Workflow");
    let desc = body.get("description").and_then(|v| v.as_str());
    let nodes: serde_json::Value = body.get("nodes").cloned().unwrap_or(serde_json::json!([]));
    let edges: serde_json::Value = body.get("edges").cloned().unwrap_or(serde_json::json!([]));
    let now = chrono::Utc::now().to_rfc3339();
    let run_count: i64 = 0;

    let client = pool.get().await.unwrap();
    client.execute(
        "INSERT INTO workflows (id, name, description, status, nodes, edges, run_count, created_at, updated_at) VALUES ($1,$2,$3,'draft',$4,$5,$6,$7,$7)",
        &[&id, &name, &desc, &nodes, &edges, &run_count, &now],
    ).await.ok();

    HttpResponse::Ok().json(serde_json::json!({"id": id, "name": name}))
}
