use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use ferris_credits::CreditLedger;
use tracing_subscriber::EnvFilter;

use ferris_coordinator::registry::AgentRegistry;
use ferris_coordinator::router::InferenceRouter;
use ferris_coordinator::routes::{run_coordinator, AppState};
use ferris_coordinator::storage_router::StorageRouter;

#[derive(Parser)]
#[command(
    name = "ferris-coordinator",
    about = "OpenFerris coordinator — agent registry, inference routing, credit ledger"
)]
struct Cli {
    /// Database path
    #[arg(long, default_value = "/var/ferris/coordinator.db")]
    db_path: String,

    /// Listen host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Listen port
    #[arg(long, default_value_t = 8421)]
    port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ferris_coordinator=info")),
        )
        .init();

    let cli = Cli::parse();
    let db_path = PathBuf::from(&cli.db_path);

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("failed to create database directory");
    }

    let pool = ferris_coordinator::init_coordinator_pool(&db_path)
        .await
        .expect("failed to init coordinator DB");

    ferris_coordinator::run_coordinator_migrations(&pool)
        .await
        .expect("failed to run migrations");

    let ledger = CreditLedger::new(pool.clone());
    let registry = AgentRegistry::new(pool.clone(), ledger);
    let router = InferenceRouter::new(pool.clone());
    let storage_router = StorageRouter::new(pool);

    let registry_arc = Arc::new(registry);

    // Background: stale-agent sweep (every 30s)
    let sweep_registry = registry_arc.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = sweep_registry.sweep_stale_agents().await {
                tracing::error!(error = %e, "stale agent sweep failed");
            }
        }
    });

    // Background: availability rewards (every 60s, 10 millicredits per active agent)
    let rewards_registry = registry_arc.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = rewards_registry.award_availability_batch(10).await {
                tracing::error!(error = %e, "availability reward failed");
            }
        }
    });

    let state = AppState {
        registry: registry_arc,
        router: Arc::new(router),
        storage_router: Arc::new(storage_router),
    };

    if let Err(e) = run_coordinator(state, &cli.host, cli.port).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
