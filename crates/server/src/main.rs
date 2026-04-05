mod db;
mod handlers;
mod models;
mod scheduler;

use actix_files as fs;
use actix_web::{web, App, HttpServer, middleware, Error};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use argon2::{Argon2, PasswordVerifier};
use argon2::password_hash::PasswordHash;

async fn validator(req: actix_web::dev::ServiceRequest, credentials: BearerAuth) -> Result<actix_web::dev::ServiceRequest, (Error, actix_web::dev::ServiceRequest)> {
    let token = credentials.token();
    let pool = req.app_data::<web::Data<db::DbPool>>().cloned().unwrap();
    let conn = pool.get().unwrap();
    // Fetch active key hashes and verify using Argon2 via password-hash API
    let mut stmt = conn.prepare("SELECT key_hash FROM api_keys WHERE status = 'active'").unwrap();
    let key_iter = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
    for kh in key_iter.filter_map(|r| r.ok()) {
        if let Ok(parsed) = PasswordHash::new(&kh) {
            if Argon2::default().verify_password(token.as_bytes(), &parsed).is_ok() {
                conn.execute("UPDATE api_keys SET last_used_at = CURRENT_TIMESTAMP WHERE key_hash = ?1", [&kh]).ok();
                return Ok(req);
            }
        }
    }
    Err((actix_web::error::ErrorUnauthorized("Invalid API key"), req))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/app.db".into());
    let pool = db::create_pool(&db_path);
    log::info!("Database initialized at {db_path}");

    // Start background scheduler
    let scheduler_pool = pool.clone();
    tokio::spawn(async move {
        scheduler::start_scheduler(scheduler_pool).await;
    });

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    log::info!("Starting Mermaduckle server on {host}:{port}");

    HttpServer::new(move || {
        let static_dir = find_static_dir();

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            // Public routes
            .service(handlers::health::health_check)
            // Dev-only unprotected key creation (local dev convenience)
            .service(handlers::settings::create_api_key_dev)
            // Protected API routes
            .service(
                web::scope("/api")
                    .wrap(HttpAuthentication::bearer(validator))
                    .service(handlers::dashboard::get_dashboard)
                    .service(handlers::dashboard::list_recent_activity)
                    .service(handlers::audit::list_audit_events)
                    // Workflows — order matters: specific paths before parameterized
                    .service(handlers::workflows::export_workflows)
                    .service(handlers::workflows::import_workflows)
                    .service(handlers::workflows::list_workflows)
                    .service(handlers::workflows::create_workflow)
                    .service(handlers::workflows::get_workflow)
                    .service(handlers::workflows::update_workflow)
                    .service(handlers::workflows::delete_workflow)
                    .service(handlers::workflows::run_workflow)
                    .service(handlers::workflows::get_workflow_runs)
                    .service(handlers::workflows::export_workflow)
                    .service(handlers::workflows::import_workflow)
                    // Recovery & Reporting
                    .service(handlers::recovery::self_heal_node)
                    .service(handlers::reporting::generate_workflow_report)
                    // Agents
                    .service(handlers::agents::list_agents)
                    .service(handlers::agents::get_agent)
                    .service(handlers::agents::create_agent)
                    .service(handlers::agents::update_agent)
                    .service(handlers::agents::delete_agent)
                    .service(handlers::agents::chat_with_agent)
                    // Settings
                    .service(handlers::settings::list_api_keys)
                    .service(handlers::settings::create_api_key)
                    .service(handlers::settings::rotate_api_key)
                    .service(handlers::settings::delete_api_key)
                    .service(handlers::settings::list_team)
                    .service(handlers::settings::add_team_member)
                    .service(handlers::settings::remove_team_member)
                    .service(handlers::settings::list_integrations)
                    .service(handlers::settings::update_integration)
                    .service(handlers::settings::get_notifications)
                    .service(handlers::settings::update_notifications)
                    .service(handlers::settings::list_secrets)
                    .service(handlers::settings::create_secret)
                    .service(handlers::settings::delete_secret)
                    // Webhooks
                    .service(handlers::webhook::handle_webhook)
                    .service(handlers::webhook::list_webhook_logs)
                    // Approvals
                    .service(handlers::approvals::list_pending_approvals)
                    .service(handlers::approvals::handle_approval)
                    // Architect
                    .service(handlers::architect::generate_workflow_draft)
            )
            // ── Static Files & SPA Fallback ──
            .service(fs::Files::new("/static", &static_dir).show_files_listing())
            .default_service(web::get().to(serve_index))
    })
    .bind((host, port))?
    .run()
    .await
}

async fn serve_index(req: actix_web::HttpRequest) -> actix_web::HttpResponse {
    // Serve the marketing site at `/`, the docs hub at `/docs`, and the SPA at `/app`.
    let path = req.path();
    if path.starts_with("/app") {
        let html = include_str!("../static/app/index.html");
        actix_web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    } else if path.starts_with("/docs") {
        let html = include_str!("../static/marketing/docs.html");
        actix_web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    } else {
        let html = include_str!("../static/marketing/index.html");
        actix_web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    }
}

fn find_static_dir() -> String {
    // Try multiple locations for the static directory
    for candidate in &[
        "crates/server/static",
        "static",
        "../static",
    ] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }
    "static".to_string()
}
