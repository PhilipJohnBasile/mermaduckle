use crate::db::DbPool;
use crate::models::*;
use actix_web::{HttpResponse, get, web};

#[get("/api/dashboard")]
pub async fn get_dashboard(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();

    let total_workflows: i64 = conn
        .query_row("SELECT COUNT(*) FROM workflows", [], |r| r.get(0))
        .unwrap_or(0);
    let active_workflows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflows WHERE status = 'active'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let total_runs: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(run_count), 0) FROM workflows",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let successful_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_runs WHERE status = 'completed'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let failed_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_runs WHERE status = 'failed'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let pending_approvals: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_runs WHERE status = 'pending_approval'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut stmt = conn
        .prepare("SELECT id, name, run_count, 95.0 FROM workflows ORDER BY run_count DESC LIMIT 5")
        .unwrap();
    let top_workflows: Vec<TopWorkflow> = stmt
        .query_map([], |row| {
            Ok(TopWorkflow {
                id: row.get(0)?,
                name: row.get(1)?,
                runs: row.get(2)?,
                success_rate: row.get(3)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let metrics = DashboardMetrics {
        total_workflows,
        active_workflows,
        total_runs,
        successful_runs,
        failed_runs,
        pending_approvals,
        avg_execution_time: 1250.0,
        total_cost: 142.50,
        top_workflows,
    };

    HttpResponse::Ok().json(metrics)
}

#[get("/api/logs/stream")]
pub async fn list_recent_activity(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn
        .prepare(
            "
        SELECT wr.id, w.name, wr.status, wr.logs, wr.started_at 
        FROM workflow_runs wr
        JOIN workflows w ON wr.workflow_id = w.id
        ORDER BY wr.started_at DESC LIMIT 20
    ",
        )
        .unwrap();

    let history: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let logs_str: String = row.get::<_, String>(3)?;
            let logs: Vec<serde_json::Value> = serde_json::from_str(&logs_str).unwrap_or_default();
            Ok(serde_json::json!({
                "runId": row.get::<_, String>(0)?,
                "workflowName": row.get::<_, String>(1)?,
                "status": row.get::<_, String>(2)?,
                "latestLog": logs.last().cloned(),
                "timestamp": row.get::<_, String>(4)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(history)
}
