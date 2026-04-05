use crate::db::DbPool;
use crate::models::HealthServices;
use crate::models::HealthStatus;
use actix_web::{HttpResponse, get, web};

#[get("/api/health")]
pub async fn health_check(pool: web::Data<DbPool>) -> HttpResponse {
    let db_status = match pool.get().await {
        Ok(client) => match client.query_one("SELECT 1", &[]).await {
            Ok(_) => "ok".to_string(),
            Err(_) => "error".to_string(),
        },
        Err(_) => "error".to_string(),
    };

    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
    let ollama_status = match reqwest::get(format!("{ollama_url}/api/tags")).await {
        Ok(r) if r.status().is_success() => "ok".to_string(),
        Ok(r) => format!("error: status {}", r.status()),
        Err(e) => format!("unreachable: {e}"),
    };

    let overall = if db_status == "ok" && ollama_status == "ok" {
        "ok"
    } else {
        "degraded"
    };

    HttpResponse::Ok().json(HealthStatus {
        status: overall.into(),
        version: "0.1.0".into(),
        services: HealthServices {
            database: db_status,
            ollama: ollama_status,
        },
    })
}
