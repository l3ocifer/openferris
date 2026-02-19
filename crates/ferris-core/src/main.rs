use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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
    /// Initialize, join the network, and start serving — all in one command
    Start {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8420)]
        port: u16,
        /// Percentage of resources to contribute (0-100)
        #[arg(long)]
        contribute_percent: Option<u32>,
        /// Region identifier (e.g. "us-east")
        #[arg(long)]
        region: Option<String>,
        /// Coordinator URL override
        #[arg(long)]
        coordinator_url: Option<String>,
    },
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
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("ferris=info")),
        )
        .init();

    let cli = Cli::parse();
    let data_dir = ferris_core::resolve_data_dir(cli.data_dir.as_deref());

    let result = match cli.command {
        Commands::Start { host, port, contribute_percent, region, coordinator_url } => {
            cmd_start(
                &data_dir,
                &host,
                port,
                contribute_percent,
                region.as_deref(),
                coordinator_url.as_deref(),
            )
            .await
        }
        Commands::Init { agent_name } => cmd_init(&data_dir, &agent_name).await,
        Commands::Serve { transport, host, port } => {
            cmd_serve(&data_dir, transport.as_deref(), &host, port).await
        }
        Commands::Join { coordinator_url, endpoint_url, region } => {
            cmd_join(
                &data_dir,
                coordinator_url.as_deref(),
                endpoint_url.as_deref(),
                region.as_deref(),
            )
            .await
        }
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

// ── start: the one-command experience ───────────────────────────────────

async fn cmd_start(
    data_dir: &Path,
    host: &str,
    port: u16,
    contribute_percent_override: Option<u32>,
    region: Option<&str>,
    coordinator_url_override: Option<&str>,
) -> ferris_common::Result<()> {
    use ferris_core::identity::Identity;

    // Step 1: Auto-init if needed
    std::fs::create_dir_all(data_dir)?;
    std::fs::create_dir_all(data_dir.join("objects"))?;
    ferris_core::save_default_config(data_dir, "ferris-agent")?;

    let db_path = data_dir.join("ferris.db");
    let pool = ferris_core::init_pool(&db_path).await?;
    ferris_core::run_migrations(&pool).await?;

    let identity = match Identity::load(&pool).await? {
        Some(id) => id,
        None => {
            let id = Identity::generate();
            id.save(&pool).await?;
            println!("Initialized OpenFerris node");
            println!("  agent_id: {}", id.agent_id);
            id
        }
    };

    let mut config = ferris_core::load_config(data_dir)?;
    if let Some(pct) = contribute_percent_override {
        config.network.contribute_percent = pct.min(100);
    }
    let contribute_pct = config.network.contribute_percent;

    // Step 2: Detect resources and Ollama
    let full_resources = ferris_core::resources::detect();
    let contributed = full_resources.contributed(contribute_pct);

    let ollama = ferris_inference::OllamaProxy::new(
        &config.inference.ollama_url,
        config.inference.max_concurrent_requests,
    );
    let models: Vec<ferris_common::ModelInfo> = ollama.list_models().await.unwrap_or_default();

    println!();
    println!("Detected resources:");
    println!(
        "  cpu: {} cores, ram: {} MB, storage: {} MB",
        full_resources.cpu_cores, full_resources.ram_mb, full_resources.storage_avail_mb
    );
    if let Some(gpu) = &full_resources.gpu {
        println!("  gpu: {} ({} MB)", gpu.name, gpu.vram_mb);
    }
    if !models.is_empty() {
        let names: Vec<&str> = models.iter().map(|m| m.model_name.as_str()).collect();
        println!("  ollama: {} models ({})", models.len(), names.join(", "));
    } else {
        println!("  ollama: not detected");
    }

    println!();
    println!("Contributing {}% of resources:", contribute_pct);
    println!(
        "  cpu: {} cores, ram: {} MB, storage: {} MB",
        contributed.cpu_cores, contributed.ram_mb, contributed.storage_avail_mb
    );
    if config.network.contribute_gpu && contributed.gpu.is_some() {
        println!("  gpu: inference enabled");
    }

    // Step 3: Try to join coordinator (non-blocking — run local-only if unreachable)
    let coordinator_url = coordinator_url_override
        .map(String::from)
        .unwrap_or_else(|| config.network.coordinator_url.clone());
    let endpoint_url = format!("http://{host}:{port}");
    let agent_id = identity.agent_id.clone();
    let public_key = identity.public_key_bytes().to_vec();
    let cipher = ferris_crypto::Cipher::from_secret_key_bytes(&identity.signing_key.to_bytes());

    let client =
        ferris_net::CoordinatorClient::new(&coordinator_url, &agent_id, identity.signing_key);

    let reg_req = ferris_common::RegisterRequest {
        agent_id: agent_id.clone(),
        public_key,
        resources: contributed.clone(),
        models: models.clone(),
        contribute_gpu: config.network.contribute_gpu,
        contribute_storage: config.network.contribute_storage,
        contribute_cpu: config.network.contribute_cpu,
        max_concurrent_requests: config.inference.max_concurrent_requests,
        endpoint_url: Some(endpoint_url),
        region: region.map(String::from),
    };

    let connected = match client.register(&reg_req).await {
        Ok(resp) => {
            println!();
            println!("Network: connected to coordinator");
            if resp.signup_bonus_mc > 0 {
                println!("  signup bonus: {:.1} credits", resp.signup_bonus_mc as f64 / 1000.0);
            }
            true
        }
        Err(e) => {
            println!();
            println!("Network: coordinator unreachable ({e})");
            println!("  running in local-only mode — will retry in background");
            false
        }
    };

    // Step 4: Start heartbeat loop (if connected, or retry loop if not)
    let hb_client = client.clone();
    let hb_models = models.clone();
    let hb_resources = contributed.clone();
    let hb_interval = config.network.heartbeat_interval_secs;
    let hb_ollama_url = config.inference.ollama_url.clone();
    let hb_max_concurrent = config.inference.max_concurrent_requests;

    if connected {
        // Already registered — start heartbeat immediately
        let hb_agent_id = agent_id.clone();
        tokio::spawn(async move {
            heartbeat_loop(
                hb_client,
                hb_agent_id,
                hb_resources,
                hb_models,
                hb_interval,
                hb_ollama_url,
                hb_max_concurrent,
            )
            .await;
        });
    } else {
        // Not connected — retry registration in background, then start heartbeat
        let retry_client = client.clone();
        let retry_req = reg_req.clone();
        let retry_agent_id = agent_id.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                match retry_client.register(&retry_req).await {
                    Ok(resp) => {
                        tracing::info!("connected to coordinator: {}", resp.message);
                        heartbeat_loop(
                            retry_client,
                            retry_agent_id,
                            hb_resources,
                            hb_models,
                            hb_interval,
                            hb_ollama_url,
                            hb_max_concurrent,
                        )
                        .await;
                        break;
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "coordinator still unreachable, retrying...");
                    }
                }
            }
        });
    }

    // Step 5: Start HTTP server (blocks)
    println!();
    println!("HTTP server:  http://{host}:{port}");
    println!("MCP server:   use `ferris serve` for stdio transport");
    println!("Heartbeat:    every {}s", config.network.heartbeat_interval_secs);
    println!("Encryption:   AES-256-GCM (at rest)");
    println!();
    println!("Ready. Earning credits from contributed resources.");
    println!("Press Ctrl+C to stop.");
    println!();

    ferris_core::server::run_server(&config, pool, &agent_id, host, port, Some(cipher)).await
}

async fn heartbeat_loop(
    client: ferris_net::CoordinatorClient,
    agent_id: String,
    resources: ferris_common::ResourceManifest,
    models: Vec<ferris_common::ModelInfo>,
    interval_secs: u64,
    ollama_url: String,
    max_concurrent: u32,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let ollama = ferris_inference::OllamaProxy::new(&ollama_url, max_concurrent);
        let current_load = ollama.current_load();

        let req = ferris_common::HeartbeatRequest {
            agent_id: agent_id.clone(),
            resources: resources.clone(),
            models: models.clone(),
            current_requests: current_load,
        };

        match client.heartbeat(&req).await {
            Ok(resp) => {
                if resp.status != "ok" {
                    tracing::warn!(status = %resp.status, "coordinator heartbeat status");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "heartbeat failed");
            }
        }
    }
}

// ── Existing commands (unchanged) ───────────────────────────────────────

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
            "node not initialized — run `ferris init` or `ferris start` first".into(),
        ));
    }

    let pool = ferris_core::init_pool(&db_path).await?;
    ferris_core::run_migrations(&pool).await?;

    let identity = ferris_core::identity::Identity::load(&pool).await?.ok_or_else(|| {
        ferris_common::FerrisError::Config("identity missing — run `ferris init` first".into())
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
            let inference = Arc::new(ferris_inference::OllamaProxy::new(
                &config.inference.ollama_url,
                config.inference.max_concurrent_requests,
            ));

            ferris_mcp::serve_stdio(identity.agent_id, memory, storage, tasks, inference, None)
                .await
        }
        "http" => {
            tracing::info!(agent_id = %identity.agent_id, "starting HTTP server");
            let cipher =
                ferris_crypto::Cipher::from_secret_key_bytes(&identity.signing_key.to_bytes());
            ferris_core::server::run_server(
                &config,
                pool,
                &identity.agent_id,
                host,
                port,
                Some(cipher),
            )
            .await
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
    let identity = ferris_core::identity::Identity::load(&pool).await?.ok_or_else(|| {
        ferris_common::FerrisError::Config("identity missing — run `ferris init` first".into())
    })?;

    let url = coordinator_url.unwrap_or(&config.network.coordinator_url);
    let agent_id = identity.agent_id.clone();
    let public_key = identity.public_key_bytes().to_vec();
    let client = ferris_net::CoordinatorClient::new(url, &agent_id, identity.signing_key);

    let resources = ferris_core::resources::detect().contributed(config.network.contribute_percent);

    let ollama = ferris_inference::OllamaProxy::new(
        &config.inference.ollama_url,
        config.inference.max_concurrent_requests,
    );
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
        max_concurrent_requests: config.inference.max_concurrent_requests,
        endpoint_url: endpoint_url.map(String::from),
        region: region.map(String::from),
    };

    let resp = client.register(&req).await?;
    println!("registration: {}", resp.message);
    if resp.signup_bonus_mc > 0 {
        println!("signup bonus: {} credits", resp.signup_bonus_mc as f64 / 1000.0);
    }

    pool.close().await;
    Ok(())
}

async fn cmd_balance(data_dir: &Path, coordinator_url: Option<&str>) -> ferris_common::Result<()> {
    let config = ferris_core::load_config(data_dir)?;
    let db_path = data_dir.join("ferris.db");

    if !db_path.exists() {
        return Err(ferris_common::FerrisError::Config(
            "node not initialized — run `ferris init` first".into(),
        ));
    }

    let pool = ferris_core::init_pool(&db_path).await?;
    let identity = ferris_core::identity::Identity::load(&pool).await?.ok_or_else(|| {
        ferris_common::FerrisError::Config("identity missing — run `ferris init` first".into())
    })?;

    let url = coordinator_url.unwrap_or(&config.network.coordinator_url);
    let client =
        ferris_net::CoordinatorClient::new(url, &identity.agent_id, identity.signing_key.clone());

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
    let config = ferris_core::load_config(data_dir)?;
    let db_path = data_dir.join("ferris.db");

    if !db_path.exists() {
        println!("node not initialized — run `ferris start` or `ferris init` first");
        return Ok(());
    }

    let pool = ferris_core::init_pool(&db_path).await?;

    let identity = ferris_core::identity::Identity::load(&pool).await?;
    let resources = ferris_core::resources::detect();

    let mem_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM memories").fetch_one(&pool).await.unwrap_or(0);
    let obj_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM objects").fetch_one(&pool).await.unwrap_or(0);
    let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE enabled = 1")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let ollama = ferris_inference::OllamaProxy::new(
        &config.inference.ollama_url,
        config.inference.max_concurrent_requests,
    );
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
        println!("  gpu:          {} ({} MB)", gpu.name, gpu.vram_mb);
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
