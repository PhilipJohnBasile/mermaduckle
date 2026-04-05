use crate::db::DbPool;
use crate::models::*;
use actix_web::{HttpResponse, get, web};

#[get("/api/audit")]
pub async fn list_audit_events(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = pool.get().unwrap();
    let mut stmt = conn.prepare("SELECT id, type, severity, actor_id, actor_name, actor_email, target_type, target_id, target_name, metadata, timestamp FROM audit_events ORDER BY timestamp DESC").unwrap();

    let events: Vec<AuditEvent> = stmt
        .query_map([], |row| {
            let meta_str: String = row.get::<_, String>(9).unwrap_or_else(|_| "{}".into());
            Ok(AuditEvent {
                id: row.get(0)?,
                event_type: row.get(1)?,
                severity: row.get(2)?,
                actor: AuditActor {
                    id: row.get(3)?,
                    name: row.get(4)?,
                    email: row.get(5)?,
                },
                target: AuditTarget {
                    target_type: row.get(6)?,
                    id: row.get(7)?,
                    name: row.get(8)?,
                },
                metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({})),
                timestamp: row.get(10)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    HttpResponse::Ok().json(events)
}
