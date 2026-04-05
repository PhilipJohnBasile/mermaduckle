use crate::db::DbPool;
use crate::models::*;
use actix_web::{HttpResponse, get, web};

#[get("/api/audit")]
pub async fn list_audit_events(pool: web::Data<DbPool>) -> HttpResponse {
    let client = pool.get().await.unwrap();
    let rows = client
        .query("SELECT id, type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata, timestamp FROM audit_events ORDER BY timestamp DESC", &[])
        .await
        .unwrap_or_default();

    let events: Vec<AuditEvent> = rows
        .iter()
        .map(|row| AuditEvent {
            id: row.get(0),
            event_type: row.get(1),
            severity: row.get(2),
            actor: AuditActor {
                id: row.get(3),
                name: row.get(4),
                email: row.get(5),
            },
            target: AuditTarget {
                target_type: row.get(6),
                id: row.get(7),
                name: row.get(8),
            },
            metadata: row.get::<_, serde_json::Value>(9),
            timestamp: row.get(10),
        })
        .collect();

    HttpResponse::Ok().json(events)
}
