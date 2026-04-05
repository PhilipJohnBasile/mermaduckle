use crate::db::DbPool;
use crate::models::*;
use actix_web::{HttpResponse, get, web};

#[get("/api/dashboard")]
pub async fn get_dashboard(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();

    let total_workflows: i64 = client
        .query_one("SELECT COUNT(*)::bigint FROM workflows", &[])
        .await
        .map(|r| r.get(0))
        .unwrap_or(0);
    let active_workflows: i64 = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM workflows WHERE status = 'active'",
            &[],
        )
        .await
        .map(|r| r.get(0))
        .unwrap_or(0);
    let total_runs: i64 = client
        .query_one(
            "SELECT COALESCE(SUM(run_count), 0)::bigint FROM workflows",
            &[],
        )
        .await
        .map(|r| r.get(0))
        .unwrap_or(0);
    let successful_runs: i64 = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM workflow_runs WHERE status = 'completed'",
            &[],
        )
        .await
        .map(|r| r.get(0))
        .unwrap_or(0);
    let failed_runs: i64 = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM workflow_runs WHERE status = 'failed'",
            &[],
        )
        .await
        .map(|r| r.get(0))
        .unwrap_or(0);
    let pending_approvals: i64 = client
        .query_one(
            "SELECT COUNT(*)::bigint FROM workflow_runs WHERE status = 'pending_approval'",
            &[],
        )
        .await
        .map(|r| r.get(0))
        .unwrap_or(0);

    let rows = client
        .query(
            "SELECT id, name, run_count, 95.0::double precision FROM workflows ORDER BY run_count DESC LIMIT 5",
            &[],
        )
        .await
        .unwrap_or_default();

    let top_workflows: Vec<TopWorkflow> = rows
        .iter()
        .map(|row| TopWorkflow {
            id: row.get(0),
            name: row.get(1),
            runs: row.get(2),
            success_rate: row.get(3),
        })
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
    let client = pool.get().await.unwrap();
    let rows = client
        .query(
            "SELECT wr.id, w.name, wr.status, wr.logs, wr.started_at
             FROM workflow_runs wr
             JOIN workflows w ON wr.workflow_id = w.id
             ORDER BY wr.started_at DESC LIMIT 20",
            &[],
        )
        .await
        .unwrap_or_default();

    let history: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let logs: serde_json::Value = row.get(3);
            let logs_arr = logs.as_array();
            let latest_log = logs_arr.and_then(|a| a.last()).cloned();
            serde_json::json!({
                "runId": row.get::<_, String>(0),
                "workflowName": row.get::<_, String>(1),
                "status": row.get::<_, String>(2),
                "latestLog": latest_log,
                "timestamp": row.get::<_, String>(4),
            })
        })
        .collect();

    HttpResponse::Ok().json(history)
}
