use crate::db::DbPool;
use crate::models::Workflow;
use chrono::Utc;
use mermaduckle_engine::execute_workflow_engine;
use std::time::Duration;
use tokio::time::interval;

pub async fn start_scheduler(pool: DbPool) {
    let mut interval = interval(Duration::from_secs(60));
    println!("🚀 Mermaduckle Scheduler started (Interval: 60s)");

    loop {
        interval.tick().await;
        let pool = pool.clone();

        tokio::spawn(async move {
            if let Err(e) = check_and_run_schedules(pool).await {
                eprintln!("❌ Scheduler error: {:?}", e);
            }
        });
    }
}

async fn check_and_run_schedules(pool: DbPool) -> Result<(), Box<dyn std::error::Error>> {
    let workflows: Vec<Workflow> = {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT id, name, nodes, edges, schedule, last_run_at FROM workflows WHERE schedule IS NOT NULL AND status = 'active'")?;

        let rows = stmt.query_map([], |row| {
            // Read all columns in strict order to satisfy Rusqlite's buffer reuse safety
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let nodes_str: String = row.get(2)?;
            let edges_str: String = row.get(3)?;
            let schedule: Option<String> = row.get(4)?;
            let last_run_at: Option<String> = row.get(5)?;

            Ok(Workflow {
                id,
                name,
                description: None,
                status: "active".into(),
                nodes: serde_json::from_str(&nodes_str).unwrap_or(serde_json::json!([])),
                edges: serde_json::from_str(&edges_str).unwrap_or(serde_json::json!([])),
                run_count: 0,
                last_run_at,
                schedule,
                created_at: None,
                updated_at: None,
            })
        })?;

        // Materialize results to avoid borrow checker/iterator issues outside the block
        let mut collection = Vec::new();
        for res in rows {
            if let Ok(wf) = res {
                collection.push(wf);
            }
        }
        collection
    };

    for wf in workflows {
        if should_run(&wf) {
            println!("⏰ Scheduled trigger: {} ({})", wf.name, wf.id);
            run_scheduled_workflow(pool.clone(), wf).await?;
        }
    }

    Ok(())
}

fn should_run(wf: &Workflow) -> bool {
    let schedule = match &wf.schedule {
        Some(s) => s,
        None => return false,
    };

    let last_run = match &wf.last_run_at {
        Some(t) => Utc::now()
            .signed_duration_since(
                chrono::DateTime::parse_from_rfc3339(t)
                    .unwrap()
                    .with_timezone(&Utc),
            )
            .to_std()
            .unwrap_or(Duration::from_secs(0)),
        None => return true, // Never run before
    };

    // Simple duration check for now (e.g. "1h", "10m", "1d")
    let interval = if schedule.ends_with('h') {
        Duration::from_secs(schedule.trim_end_matches('h').parse::<u64>().unwrap_or(1) * 3600)
    } else if schedule.ends_with('m') {
        Duration::from_secs(schedule.trim_end_matches('m').parse::<u64>().unwrap_or(1) * 60)
    } else if schedule.ends_with('d') {
        Duration::from_secs(schedule.trim_end_matches('d').parse::<u64>().unwrap_or(1) * 86400)
    } else {
        Duration::from_secs(3600) // Default 1h
    };

    last_run >= interval
}

async fn run_scheduled_workflow(
    pool: DbPool,
    wf: Workflow,
) -> Result<(), Box<dyn std::error::Error>> {
    let run_id = format!(
        "run_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("x")
    );
    // 1. Create run record
    {
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO workflow_runs (id, workflow_id, status) VALUES (?1, ?2, 'running')",
            rusqlite::params![run_id, wf.id],
        )?;
    }

    // 2. Execute engine
    let engine_nodes: Vec<mermaduckle_engine::WorkflowNode> =
        serde_json::from_value(wf.nodes.clone()).unwrap_or_default();
    let engine_edges: Vec<mermaduckle_engine::WorkflowEdge> =
        serde_json::from_value(wf.edges.clone()).unwrap_or_default();
    let engine_wf = mermaduckle_engine::Workflow {
        nodes: engine_nodes,
        edges: engine_edges,
    };

    let result = execute_workflow_engine(&engine_wf, None, None, None, false).await;

    // 3. Update run record and workflow last_run_at
    {
        let conn = pool.get()?;
        let now = Utc::now().to_rfc3339();
        let status = if result.paused_node_id.is_some() {
            "paused"
        } else {
            "completed"
        };

        conn.execute(
            "UPDATE workflow_runs SET status = ?1, completed_at = ?2, output = ?3, logs = ?4, context = ?5, paused_node_id = ?6 WHERE id = ?7",
            rusqlite::params![
                status,
                now,
                result.output,
                serde_json::to_string(&result.logs)?,
                serde_json::to_string(&result.context)?,
                result.paused_node_id,
                run_id
            ],
        )?;

        conn.execute(
            "UPDATE workflows SET last_run_at = ?1, run_count = run_count + 1 WHERE id = ?2",
            rusqlite::params![now, wf.id],
        )?;
    }

    Ok(())
}
