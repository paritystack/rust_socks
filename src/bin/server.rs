use clap::{Parser, Subcommand};
use rust_socks::auth::{MultiAuth, UserPassAuth};
use rust_socks::config::Config;
use rust_socks::server::ServerBuilder;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// SOCKS5 proxy server application
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the SOCKS5 server
    Start {
        /// Path to configuration file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Bind address (overrides config file)
        #[arg(short, long)]
        bind: Option<String>,

        /// Disable authentication (overrides config file)
        #[arg(long)]
        no_auth: bool,

        /// Set log level (trace, debug, info, warn, error)
        #[arg(short, long, default_value = "info")]
        log_level: String,
    },

    /// Generate a default configuration file
    GenerateConfig {
        /// Output path for the configuration file
        #[arg(short, long, default_value = "socks5.toml")]
        output: PathBuf,

        /// Overwrite existing file
        #[arg(short, long)]
        force: bool,
    },

    /// Validate a configuration file
    Validate {
        /// Path to configuration file
        #[arg(value_name = "FILE")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            config,
            bind,
            no_auth,
            log_level,
        } => {
            start_server(config, bind, no_auth, log_level).await?;
        }
        Commands::GenerateConfig { output, force } => {
            generate_config(output, force)?;
        }
        Commands::Validate { config } => {
            validate_config(config)?;
        }
    }

    Ok(())
}

async fn start_server(
    config_path: Option<PathBuf>,
    bind_override: Option<String>,
    no_auth: bool,
    log_level: String,
) -> anyhow::Result<()> {
    // Load configuration
    let config = if let Some(path) = config_path {
        info!("Loading configuration from: {}", path.display());
        Config::from_file(path)?
    } else {
        info!("Using default configuration");
        Config::default()
    };

    // Initialize logging
    let filter = EnvFilter::try_new(&log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    if config.logging.json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .init();
    }

    info!("Starting SOCKS5 server...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Determine bind address
    let bind_addr = if let Some(addr) = bind_override {
        addr
    } else {
        config.server.bind_addr.clone()
    };

    info!("Bind address: {}", bind_addr);

    // Create server builder
    let mut builder = ServerBuilder::new()
        .bind_addr(bind_addr.parse()?)
        .enable_connect(config.server.enable_connect)
        .enable_bind(config.server.enable_bind)
        .enable_udp(config.server.enable_udp)
        .timeout(config.server.timeout_secs);

    // Configure authentication
    if no_auth {
        info!("Authentication: None (disabled via command line)");
        builder = builder.no_auth();
    } else {
        match config.auth.mode.as_str() {
            "none" => {
                info!("Authentication: None");
                builder = builder.no_auth();
            }
            "password" => {
                info!("Authentication: Username/Password");
                info!("Configured users: {}", config.auth.users.len());
                let auth = UserPassAuth::with_users(
                    config.auth.users.iter()
                        .map(|(u, p)| (u.clone(), p.clone()))
                        .collect()
                );
                builder = builder.authenticator(Arc::new(auth));
            }
            "multi" => {
                info!("Authentication: Multiple methods (NoAuth + Username/Password)");
                let auth = UserPassAuth::with_users(
                    config.auth.users.iter()
                        .map(|(u, p)| (u.clone(), p.clone()))
                        .collect()
                );
                let multi_auth = MultiAuth::new()
                    .with_no_auth()
                    .with_user_pass(auth);
                builder = builder.multi_auth(multi_auth);
            }
            _ => {
                error!("Invalid authentication mode: {}", config.auth.mode);
                anyhow::bail!("Invalid authentication mode");
            }
        }
    }

    // Log enabled commands
    info!("Commands enabled:");
    info!("  CONNECT: {}", config.server.enable_connect);
    info!("  BIND: {}", config.server.enable_bind);
    info!("  UDP ASSOCIATE: {}", config.server.enable_udp);
    info!("Connection timeout: {} seconds", config.server.timeout_secs);

    // Build and run server
    let server = builder.build();
    info!("Server ready, accepting connections...");

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}

fn generate_config(output: PathBuf, force: bool) -> anyhow::Result<()> {
    if output.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists: {}\nUse --force to overwrite",
            output.display()
        );
    }

    let config = Config::generate_default();
    config.to_file(&output)?;

    println!("✓ Generated default configuration file: {}", output.display());
    println!("\nConfiguration details:");
    println!("  Bind address: {}", config.server.bind_addr);
    println!("  Authentication: {}", config.auth.mode);
    println!("  Users configured: {}", config.auth.users.len());
    println!("\nEdit the file to customize your server settings.");

    Ok(())
}

fn validate_config(path: PathBuf) -> anyhow::Result<()> {
    println!("Validating configuration file: {}", path.display());

    let config = Config::from_file(&path)?;

    println!("✓ Configuration is valid!\n");
    println!("Server settings:");
    println!("  Bind address: {}", config.server.bind_addr);
    println!("  CONNECT: {}", config.server.enable_connect);
    println!("  BIND: {}", config.server.enable_bind);
    println!("  UDP ASSOCIATE: {}", config.server.enable_udp);
    println!("  Timeout: {} seconds", config.server.timeout_secs);

    println!("\nAuthentication settings:");
    println!("  Mode: {}", config.auth.mode);
    println!("  Users: {}", config.auth.users.len());
    if !config.auth.users.is_empty() {
        println!("  Usernames: {}", config.auth.users.keys()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", "));
    }

    println!("\nLogging settings:");
    println!("  Level: {}", config.logging.level);
    println!("  JSON format: {}", config.logging.json);

    Ok(())
}
