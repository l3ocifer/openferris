use std::path::Path;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "ferris", about = "OpenFerris — local-first agent platform")]
struct Cli {
    /// Data directory (default: ~/.ferris)
    #[arg(long, global = true)]
    data_dir: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new OpenFerris node
    Init {
        /// Agent name
        #[arg(long, default_value = "ferris-agent")]
        agent_name: String,
    },
    /// Start the OpenFerris server (MCP stdio by default, or HTTP)
    Serve {
        /// Transport: stdio (MCP) or http (REST dev server)
        #[arg(long)]
        transport: Option<String>,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8420)]
        port: u16,
    },
    /// Register with the coordinator network
    Join {
        /// Coordinator URL override
        #[arg(long)]
        coordinator_url: Option<String>,
        /// Node's public endpoint URL for receiving inference requests
        #[arg(long)]
        endpoint_url: Option<String>,
        /// Region identifier (e.g. "us-east")
        #[arg(long)]
        region: Option<String>,
    },
    /// Query credit balance from the coordinator
    Balance {
        /// Coordinator URL override
        #[arg(long)]
        coordinator_url: Option<String>,
    },
    /// Show node status and resources
    Status,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ferris=info")),
        )
        .init();

    let cli = Cli::parse();
    let data_dir = ferris_core::resolve_data_dir(cli.data_dir.as_deref());

    let result = match cli.command {
        Commands::Init { agent_name } => cmd_init(&data_dir, &agent_name).await,
        Commands::Serve {
            transport,
            host,
            port,
        } => cmd_serve(&data_dir, transport.as_deref(), &host, port).await,
        Commands::Join {
            coordinator_url,
            endpoint_url,
            region,
        } => cmd_join(&data_dir, coordinator_url.as_deref(), endpoint_url.as_deref(), region.as_deref()).await,
        Commands::Balance { coordinator_url } => {
            cmd_balance(&data_dir, coordinator_url.as_deref()).await
        }
        Commands::Status => cmd_status(&data_dir).await,
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn cmd_init(data_dir: &Path, agent_name: &str) -> ferris_common::Result<()> {
    use ferris_core::identity::Identity;

    std::fs::create_dir_all(data_dir)?;
    std::fs::create_dir_all(data_dir.join("objects"))?;

    ferris_core::save_default_config(data_dir, agent_name)?;

    let db_path = data_dir.join("ferris.db");
    let pool = ferris_core::init_pool(&db_path).await?;
    ferris_core::run_migrations(&pool).await?;

    if let Some(existing) = Identity::load(&pool).await? {
        println!("node already initialized");
        println!("  agent_id: {}", existing.agent_id);
    } else {
        let id = Identity::generate();
        id.save(&pool).await?;
        println!("initialized OpenFerris node");
        println!("  agent_id:  {}", id.agent_id);
        println!("  data_dir:  {}", data_dir.display());
        println!("  config:    {}", data_dir.join("config.toml").display());
    }

    pool.close().await;
    Ok(())
}

async fn cmd_serve(
    data_dir: &Path,
    transport_override: Option<&str>,
    host: &str,
    port: u16,
) -> ferris_common::Result<()> {
    let config = ferris_core::load_config(data_dir)?;
    let db_path = data_dir.join("ferris.db");

    if !db_path.exists() {
        return Err(ferris_common::FerrisError::Config(
            "node not initialized — run `ferris init` first".into(),
        ));
    }

    let pool = ferris_core::init_pool(&db_path).await?;
    ferris_core::run_migrations(&pool).await?;

    let identity = ferris_core::identity::Identity::load(&pool)
        .await?
        .ok_or_else(|| {
            ferris_common::FerrisError::Config(
                "identity missing — run `ferris init` first".into(),
            )
        })?;

    let transport = transport_override.unwrap_or(&config.mcp.transport);

    match transport {
        "stdio" => {
            tracing::info!(agent_id = %identity.agent_id, "starting MCP server (stdio)");
            let objects_dir = data_dir.join("objects");
            let memory =
                Arc::new(ferris_memory::MemoryStore::new(pool.clone(), config.memory.max_entries));
            let storage = Arc::new(ferris_storage::ObjectStore::new(
                pool.clone(),
                objects_dir,
                config.storage.max_mb,
            ));
            let tasks =
                Arc::new(ferris_tasks::TaskScheduler::new(pool, config.tasks.max_scheduled));

            ferris_mcp::serve_stdio(identity.agent_id, memory, storage, tasks).await
        }
        "http" => {
            tracing::info!(agent_id = %identity.agent_id, "starting HTTP server");
            ferris_core::server::run_server(&config, pool, &identity.agent_id, host, port).await
        }
        other => Err(ferris_common::FerrisError::Config(format!(
            "unknown transport: {other} (expected 'stdio' or 'http')"
        ))),
    }
}

async fn cmd_join(
    data_dir: &Path,
    coordinator_url: Option<&str>,
    endpoint_url: Option<&str>,
    region: Option<&str>,
) -> ferris_common::Result<()> {
    let config = ferris_core::load_config(data_dir)?;
    let db_path = data_dir.join("ferris.db");

    if !db_path.exists() {
        return Err(ferris_common::FerrisError::Config(
            "node not initialized — run `ferris init` first".into(),
        ));
    }

    let pool = ferris_core::init_pool(&db_path).await?;
    let identity = ferris_core::identity::Identity::load(&pool)
        .await?
        .ok_or_else(|| {
            ferris_common::FerrisError::Config(
                "identity missing — run `ferris init` first".into(),
            )
        })?;

    let url = coordinator_url.unwrap_or(&config.network.coordinator_url);
    let agent_id = identity.agent_id.clone();
    let public_key = identity.public_key_bytes().to_vec();
    let client = ferris_net::CoordinatorClient::new(url, &agent_id, identity.signing_key);

    let resources = ferris_core::resources::detect();

    // Discover local Ollama models
    let ollama = ferris_inference::OllamaProxy::new("http://localhost:11434", 4);
    let models = match ollama.list_models().await {
        Ok(m) => {
            println!("detected {} local models", m.len());
            m
        }
        Err(_) => {
            println!("no Ollama instance detected (continuing without models)");
            vec![]
        }
    };

    let req = ferris_common::RegisterRequest {
        agent_id: agent_id.clone(),
        public_key,
        resources,
        models,
        contribute_gpu: config.network.contribute_gpu,
        contribute_storage: config.network.contribute_storage,
        contribute_cpu: config.network.contribute_cpu,
        max_concurrent_requests: config.network.max_concurrent_requests,
        endpoint_url: endpoint_url.map(String::from),
        region: region.map(String::from),
    };

    let resp = client.register(&req).await?;
    println!("registration: {}", resp.message);
    if resp.signup_bonus_mc > 0 {
        println!(
            "signup bonus: {} credits",
            resp.signup_bonus_mc as f64 / 1000.0
        );
    }

    pool.close().await;
    Ok(())
}

async fn cmd_balance(
    data_dir: &Path,
    coordinator_url: Option<&str>,
) -> ferris_common::Result<()> {
    let config = ferris_core::load_config(data_dir)?;
    let db_path = data_dir.join("ferris.db");

    if !db_path.exists() {
        return Err(ferris_common::FerrisError::Config(
            "node not initialized — run `ferris init` first".into(),
        ));
    }

    let pool = ferris_core::init_pool(&db_path).await?;
    let identity = ferris_core::identity::Identity::load(&pool)
        .await?
        .ok_or_else(|| {
            ferris_common::FerrisError::Config(
                "identity missing — run `ferris init` first".into(),
            )
        })?;

    let url = coordinator_url.unwrap_or(&config.network.coordinator_url);
    let client = ferris_net::CoordinatorClient::new(url, &identity.agent_id, identity.signing_key.clone());

    let balance = client.get_balance().await?;
    let soft = balance.soft_balance_mc as f64 / 1000.0;
    let hard = balance.hard_balance_mc as f64 / 1000.0;

    println!("Credit Balance");
    println!("──────────────");
    println!("  soft credits:  {soft:.3} (non-transferable)");
    println!("  hard credits:  {hard:.3} (earned, transferable)");
    println!(
        "  total:         {:.3}",
        (balance.soft_balance_mc + balance.hard_balance_mc) as f64 / 1000.0
    );

    pool.close().await;
    Ok(())
}

async fn cmd_status(data_dir: &Path) -> ferris_common::Result<()> {
    let db_path = data_dir.join("ferris.db");

    if !db_path.exists() {
        println!("node not initialized — run `ferris init` first");
        return Ok(());
    }

    let pool = ferris_core::init_pool(&db_path).await?;

    let identity = ferris_core::identity::Identity::load(&pool).await?;
    let resources = ferris_core::resources::detect();

    let mem_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM memories")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
    let obj_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
    let task_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE enabled = 1")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

    // Check Ollama
    let ollama = ferris_inference::OllamaProxy::new("http://localhost:11434", 4);
    let ollama_status = if ollama.health_check().await.unwrap_or(false) {
        let models = ollama.list_models().await.unwrap_or_default();
        format!("running ({} models)", models.len())
    } else {
        "not detected".into()
    };

    println!("OpenFerris Node Status");
    println!("──────────────────────");
    match identity {
        Some(id) => println!("  agent_id:     {}", id.agent_id),
        None => println!("  agent_id:     (not set)"),
    }
    println!("  data_dir:     {}", data_dir.display());
    println!();
    println!("Resources:");
    println!("  cpu_cores:    {}", resources.cpu_cores);
    println!("  ram_mb:       {}", resources.ram_mb);
    println!("  storage_mb:   {}", resources.storage_avail_mb);
    if let Some(gpu) = &resources.gpu {
        println!("  gpu:          {}", gpu.name);
    }
    println!("  ollama:       {ollama_status}");
    println!();
    println!("Data:");
    println!("  memories:     {mem_count}");
    println!("  objects:      {obj_count}");
    println!("  active_tasks: {task_count}");

    pool.close().await;
    Ok(())
}
