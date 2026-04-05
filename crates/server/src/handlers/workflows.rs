use crate::db::{DbPool, add_audit_event};
use crate::models::*;
use actix_web::{HttpResponse, delete, get, post, put, web};
use rusqlite::params;

#[get("/api/workflows")]
pub async fn list_workflows(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, description, status, nodes, edges, run_count, last_run_at, schedule, created_at, updated_at FROM workflows ORDER BY updated_at DESC")
        .unwrap();

    let workflows: Vec<Workflow> = stmt
        .query_map([], |row| {
            let nodes_str: String = row.get(4)?;
            let edges_str: String = row.get(5)?;
            Ok(Workflow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                nodes: serde_json::from_str(&nodes_str).unwrap_or(serde_json::json!([])),
                edges: serde_json::from_str(&edges_str).unwrap_or(serde_json::json!([])),
                run_count: row.get(6)?,
                last_run_at: row.get(7)?,
                schedule: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(workflows)
}

#[get("/api/workflows/{id}")]
pub async fn get_workflow(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();

    match conn.query_row(
        "SELECT id, name, description, status, nodes, edges, run_count, last_run_at, schedule, created_at, updated_at FROM workflows WHERE id = ?1",
        params![id],
        |row| {
            let nodes_str: String = row.get(4)?;
            let edges_str: String = row.get(5)?;
            Ok(Workflow {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                nodes: serde_json::from_str(&nodes_str).unwrap_or(serde_json::json!([])),
                edges: serde_json::from_str(&edges_str).unwrap_or(serde_json::json!([])),
                run_count: row.get(6)?,
                last_run_at: row.get(7)?,
                schedule: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    ) {
        Ok(w) => HttpResponse::Ok().json(w),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({"error": "Workflow not found"})),
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
    let nodes = body.nodes.clone().unwrap_or(serde_json::json!([]));
    let edges = body.edges.clone().unwrap_or(serde_json::json!([]));
    let status = body.status.clone().unwrap_or_else(|| "draft".into());

    let schedule = body.schedule.clone();

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO workflows (id, name, description, status, nodes, edges, schedule, run_count, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,0,?8,?9)",
        params![id, body.name, body.description, status, nodes.to_string(), edges.to_string(), schedule, now, now],
    ).ok();

    add_audit_event(
        &conn,
        "workflow_created",
        "low",
        "usr_1",
        "System",
        "system@mermaduckle.io",
        "workflow",
        &id,
        &body.name,
        &serde_json::json!({}),
    );

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
    let conn = pool.get().unwrap();

    // Build dynamic UPDATE
    let mut sets: Vec<String> = vec!["updated_at = ?1".into()];
    let mut idx = 2u32;
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];

    if let Some(ref name) = body.name {
        sets.push(format!("name = ?{idx}"));
        values.push(Box::new(name.clone()));
        idx += 1;
    }
    if let Some(ref desc) = body.description {
        sets.push(format!("description = ?{idx}"));
        values.push(Box::new(desc.clone()));
        idx += 1;
    }
    if let Some(ref status) = body.status {
        sets.push(format!("status = ?{idx}"));
        values.push(Box::new(status.clone()));
        idx += 1;
    }
    if let Some(ref nodes) = body.nodes {
        sets.push(format!("nodes = ?{idx}"));
        values.push(Box::new(nodes.to_string()));
        idx += 1;
    }
    if let Some(ref edges) = body.edges {
        sets.push(format!("edges = ?{idx}"));
        values.push(Box::new(edges.to_string()));
        idx += 1;
    }
    if let Some(ref schedule) = body.schedule {
        sets.push(format!("schedule = ?{idx}"));
        values.push(Box::new(schedule.clone()));
        idx += 1;
    }

    let sql = format!("UPDATE workflows SET {} WHERE id = ?{idx}", sets.join(", "));
    values.push(Box::new(id.clone()));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice()).ok();

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

#[delete("/api/workflows/{id}")]
pub async fn delete_workflow(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();
    conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])
        .ok();
    conn.execute(
        "DELETE FROM workflow_runs WHERE workflow_id = ?1",
        params![id],
    )
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
    let conn = pool.get().unwrap();

    // Load workflow
    let result = conn.query_row(
        "SELECT nodes, edges, name FROM workflows WHERE id = ?1",
        params![id],
        |row| {
            let nodes_str: String = row.get(0)?;
            let edges_str: String = row.get(1)?;
            let name: String = row.get(2)?;
            Ok((nodes_str, edges_str, name))
        },
    );

    let (nodes_str, edges_str, name) = match result {
        Ok(v) => v,
        Err(_) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"error": "Workflow not found"}));
        }
    };

    let nodes: Vec<mermaduckle_engine::WorkflowNode> =
        serde_json::from_str(&nodes_str).unwrap_or_default();
    let edges: Vec<mermaduckle_engine::WorkflowEdge> =
        serde_json::from_str(&edges_str).unwrap_or_default();

    let workflow = mermaduckle_engine::Workflow { nodes, edges };
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());

    // Create run record
    let run_id = format!("run_{}", chrono::Utc::now().timestamp_millis());
    let started_at = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO workflow_runs (id, workflow_id, status, started_at, logs, context) VALUES (?1,?2,'running',?3,'[]', '{}')",
        params![run_id, id, started_at],
    ).ok();

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
    let logs_json = serde_json::to_string(&result.logs).unwrap_or_else(|_| "[]".into());
    let context_json = serde_json::to_string(&result.context).unwrap_or_else(|_| "{}".into());

    conn.execute(
        "UPDATE workflow_runs SET status = ?1, completed_at = ?2, output = ?3, error = ?4, logs = ?5, context = ?6, paused_node_id = ?7 WHERE id = ?8",
        params![
            status,
            if status == "completed" || status == "failed" { Some(completed_at.clone()) } else { None },
            result.output,
            if status == "failed" { Some(result.output.clone()) } else { None },
            logs_json,
            context_json,
            result.paused_node_id,
            run_id
        ],
    ).ok();

    conn.execute(
        "UPDATE workflows SET run_count = run_count + 1, last_run_at = ?1, status = 'active' WHERE id = ?2",
        params![completed_at, id],
    ).ok();

    let severity = if status == "failed" { "high" } else { "low" };
    add_audit_event(
        &conn,
        "workflow_run",
        severity,
        "usr_1",
        "System",
        "system@mermaduckle.io",
        "workflow",
        &id,
        &name,
        &serde_json::json!({"runId": run_id}),
    );

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
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, workflow_id, status, started_at, completed_at, output, error, logs, context, paused_node_id FROM workflow_runs WHERE workflow_id = ?1 ORDER BY started_at DESC").unwrap();

    let runs: Vec<WorkflowRun> = stmt
        .query_map(params![id], |row| {
            let logs_str: String = row.get::<_, String>(7).unwrap_or_else(|_| "[]".into());
            let context_str: String = row.get::<_, String>(8).unwrap_or_else(|_| "{}".into());
            Ok(WorkflowRun {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                output: row.get(5)?,
                error: row.get(6)?,
                logs: serde_json::from_str(&logs_str).unwrap_or(serde_json::json!([])),
                context: serde_json::from_str(&context_str).unwrap_or(serde_json::json!({})),
                paused_node_id: row.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(runs)
}

#[get("/api/workflows/export")]
pub async fn export_workflows(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();

    let mut wf_stmt = conn
        .prepare("SELECT id, name, description, status, nodes, edges, run_count FROM workflows")
        .unwrap();
    let workflows: Vec<serde_json::Value> = wf_stmt.query_map([], |row| {
        let nodes_str: String = row.get(4)?;
        let edges_str: String = row.get(5)?;
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "description": row.get::<_, Option<String>>(2)?,
            "status": row.get::<_, String>(3)?,
            "nodes": serde_json::from_str::<serde_json::Value>(&nodes_str).unwrap_or(serde_json::json!([])),
            "edges": serde_json::from_str::<serde_json::Value>(&edges_str).unwrap_or(serde_json::json!([])),
            "run_count": row.get::<_, i64>(6)?,
        }))
    }).unwrap().filter_map(|r| r.ok()).collect();

    let mut ag_stmt = conn.prepare("SELECT id, name, description, type, model, runs, success_rate, avg_latency, cost_per_run, tags FROM agents").unwrap();
    let agents: Vec<serde_json::Value> = ag_stmt.query_map([], |row| {
        let tags_str: String = row.get(9)?;
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "description": row.get::<_, Option<String>>(2)?,
            "type": row.get::<_, String>(3)?,
            "model": row.get::<_, Option<String>>(4)?,
            "runs": row.get::<_, i64>(5)?,
            "successRate": row.get::<_, f64>(6)?,
            "avgLatency": row.get::<_, i64>(7)?,
            "costPerRun": row.get::<_, f64>(8)?,
            "tags": serde_json::from_str::<serde_json::Value>(&tags_str).unwrap_or(serde_json::json!([])),
        }))
    }).unwrap().filter_map(|r| r.ok()).collect();

    HttpResponse::Ok().json(serde_json::json!({ "workflows": workflows, "agents": agents }))
}

#[post("/api/workflows/import")]
pub async fn import_workflows(
    pool: web::Data<DbPool>,
    body: web::Json<ImportRequest>,
) -> HttpResponse {
    let conn = pool.get().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    for wf in &body.workflows {
        let id = wf.get("id").and_then(|v| v.as_str()).unwrap_or("imported");
        let name = wf
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported");
        let desc = wf.get("description").and_then(|v| v.as_str());
        let nodes_str = wf
            .get("nodes")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "[]".into());
        let edges_str = wf
            .get("edges")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "[]".into());
        conn.execute(
            "INSERT OR REPLACE INTO workflows (id, name, description, status, nodes, edges, created_at, updated_at) VALUES (?1,?2,?3,'draft',?4,?5,?6,?7)",
            params![id, name, desc, nodes_str, edges_str, now, now],
        ).ok();
    }

    for ag in &body.agents {
        let id = ag.get("id").and_then(|v| v.as_str()).unwrap_or("imported");
        let name = ag
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Imported");
        let desc = ag.get("description").and_then(|v| v.as_str());
        let model = ag.get("model").and_then(|v| v.as_str());
        conn.execute(
            "INSERT OR REPLACE INTO agents (id, name, description, model, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, name, desc, model, now, now],
        ).ok();
    }

    HttpResponse::Ok().json(serde_json::json!({"success": true, "workflows": body.workflows.len(), "agents": body.agents.len()}))
}

#[get("/api/workflows/{id}/export")]
pub async fn export_workflow(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = pool.get().unwrap();
    let result = conn.query_row(
        "SELECT name, description, nodes, edges FROM workflows WHERE id = ?1",
        params![id],
        |row| {
            let nodes_str: String = row.get(2)?;
            let edges_str: String = row.get(3)?;
            Ok(serde_json::json!({
                "name": row.get::<_, String>(0)?,
                "description": row.get::<_, Option<String>>(1)?,
                "nodes": serde_json::from_str::<serde_json::Value>(&nodes_str).unwrap_or(serde_json::json!([])),
                "edges": serde_json::from_str::<serde_json::Value>(&edges_str).unwrap_or(serde_json::json!([])),
            }))
        },
    );

    match result {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(_) => HttpResponse::NotFound().json(serde_json::json!({"error": "Workflow not found"})),
    }
}

#[post("/api/workflows/import")]
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
    let nodes = body
        .get("nodes")
        .unwrap_or(&serde_json::json!([]))
        .to_string();
    let edges = body
        .get("edges")
        .unwrap_or(&serde_json::json!([]))
        .to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO workflows (id, name, description, status, nodes, edges, run_count, created_at, updated_at) VALUES (?1,?2,?3,'draft',?4,?5,0,?6,?6)",
        params![id, name, desc, nodes, edges, now],
    ).ok();

    HttpResponse::Ok().json(serde_json::json!({"id": id, "name": name}))
}
