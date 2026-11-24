//! A minimal Axum web server for Containify: Postgres through sqlx (runtime
//! queries, so building needs no database), migrations at startup, and
//! graceful shutdown on SIGTERM.

use std::{env, io::IsTerminal, net::SocketAddr, time::Duration};

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, signal};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Colours only in a terminal: Containify's log viewer shows plain text.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is not set")?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await?;

    // `axum-starter migrate` only migrates, so it also works as a pre-deploy command.
    let migrate_only = env::args().nth(1).as_deref() == Some("migrate");
    if migrate_only || env::var("MIGRATE_ON_START").as_deref() != Ok("false") {
        // Embedded at compile time from migrations/. sqlx takes an advisory
        // lock, so replicas starting together don't apply one twice.
        sqlx::migrate!().run(&pool).await?;
        tracing::info!("migrations applied");
    }
    if migrate_only {
        return Ok(());
    }

    let app = Router::new()
        .route("/", get(home))
        .route("/health", get(health))
        .with_state(pool);

    // Containify sets HOST=:: (every interface, IPv6 and IPv4). It doesn't set
    // PORT: the template sets it, and the endpoint points at the same port.
    let host = env::var("HOST").unwrap_or_else(|_| "::".into());
    let port: u16 = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let addr = SocketAddr::new(host.parse()?, port);
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");

    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

/// Records a visit in Postgres and answers how many there have been.
async fn home(State(pool): State<PgPool>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let visits: i64 = async {
        sqlx::query("INSERT INTO visits (path) VALUES ($1)").bind("/").execute(&pool).await?;
        sqlx::query_scalar("SELECT count(*) FROM visits").fetch_one(&pool).await
    }
    .await
    .map_err(|err| {
        tracing::error!(%err, "recording visit");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "database unavailable" })))
    })?;
    Ok(Json(json!({ "message": "Hello from Axum on Containify", "visits": visits })))
}

/// The health check: 200 while the server is up and Postgres answers.
async fn health(State(pool): State<PgPool>) -> (StatusCode, Json<Value>) {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "unavailable", "database": err.to_string() })),
        ),
    }
}

/// A deploy sends SIGTERM to the old replica: stop accepting connections
/// and finish the requests in flight.
async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("Ctrl+C handler") };
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
