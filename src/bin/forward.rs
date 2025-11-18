use clap::Parser;
use rust_socks::client::{ClientBuilder, SocksClient};
use rust_socks::protocol::{SocksAddr, TargetAddr};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};
use tracing_subscriber;

/// Port forwarding tool - forwards local port to remote destination through SOCKS5 proxy
#[derive(Parser)]
#[command(author, version, about = "Forward local port to remote through SOCKS5 proxy", long_about = None)]
struct Cli {
    /// Local address to listen on (e.g., 127.0.0.1:8080 or 0.0.0.0:8080)
    #[arg(short, long)]
    local: String,

    /// Remote target address (domain or IP)
    #[arg(short, long)]
    remote: String,

    /// Remote target port
    #[arg(short = 'P', long)]
    port: u16,

    /// SOCKS5 proxy address (e.g., 127.0.0.1:1080)
    #[arg(short = 's', long, default_value = "127.0.0.1:1080")]
    proxy: String,

    /// Username for SOCKS5 authentication
    #[arg(short, long)]
    username: Option<String>,

    /// Password for SOCKS5 authentication
    #[arg(short = 'w', long)]
    password: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    info!("SOCKS5 Port Forwarder");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Local: {}", cli.local);
    info!("Remote: {}:{}", cli.remote, cli.port);
    info!("Proxy: {}", cli.proxy);

    // Create SOCKS5 client template
    let client_template = if let (Some(username), Some(password)) = (&cli.username, &cli.password) {
        info!("Using authentication: {}", username);
        ClientBuilder::new(cli.proxy.clone())
            .with_credentials(username.clone(), password.clone())
            .build()
    } else {
        info!("No authentication");
        SocksClient::new(cli.proxy.clone())
    };

    // Determine target address type
    let target_addr = if cli.remote.parse::<std::net::IpAddr>().is_ok() {
        let ip: std::net::IpAddr = cli.remote.parse()?;
        match ip {
            std::net::IpAddr::V4(ipv4) => TargetAddr::new(SocksAddr::V4(ipv4), cli.port),
            std::net::IpAddr::V6(ipv6) => TargetAddr::new(SocksAddr::V6(ipv6), cli.port),
        }
    } else {
        TargetAddr::new(SocksAddr::Domain(cli.remote.clone()), cli.port)
    };

    // Start listening on local port
    let listener = TcpListener::bind(&cli.local).await?;
    info!("✓ Listening on {}", cli.local);
    info!("Forwarding to {} via {}", target_addr, cli.proxy);
    info!("Ready to accept connections...");

    let mut connection_count = 0u64;

    loop {
        match listener.accept().await {
            Ok((local_stream, peer_addr)) => {
                connection_count += 1;
                let conn_id = connection_count;
                info!("[{}] New connection from {}", conn_id, peer_addr);

                let client = client_template.clone();
                let target = target_addr.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(conn_id, local_stream, client, target).await {
                        error!("[{}] Connection error: {}", conn_id, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    conn_id: u64,
    mut local_stream: TcpStream,
    client: SocksClient,
    target: TargetAddr,
) -> anyhow::Result<()> {
    debug!("[{}] Connecting to {} via proxy...", conn_id, target);

    // Connect to remote through SOCKS5 proxy
    let mut remote_stream = match client.connect(target.clone()).await {
        Ok(stream) => {
            info!("[{}] ✓ Connected to {}", conn_id, target);
            stream
        }
        Err(e) => {
            error!("[{}] Failed to connect to {}: {}", conn_id, target, e);
            return Err(e.into());
        }
    };

    // Split streams for bidirectional forwarding
    let (mut local_read, mut local_write) = local_stream.split();
    let (mut remote_read, mut remote_write) = remote_stream.split();

    // Forward data in both directions
    let client_to_server = async {
        match copy_with_stats(&mut local_read, &mut remote_write, conn_id, "upstream").await {
            Ok(bytes) => {
                debug!("[{}] Upstream closed ({} bytes)", conn_id, bytes);
                Ok(bytes)
            }
            Err(e) => {
                debug!("[{}] Upstream error: {}", conn_id, e);
                Err(e)
            }
        }
    };

    let server_to_client = async {
        match copy_with_stats(&mut remote_read, &mut local_write, conn_id, "downstream").await {
            Ok(bytes) => {
                debug!("[{}] Downstream closed ({} bytes)", conn_id, bytes);
                Ok(bytes)
            }
            Err(e) => {
                debug!("[{}] Downstream error: {}", conn_id, e);
                Err(e)
            }
        }
    };

    // Wait for both directions to complete
    let (upstream_result, downstream_result) = tokio::join!(client_to_server, server_to_client);

    let upstream_bytes = upstream_result.unwrap_or(0);
    let downstream_bytes = downstream_result.unwrap_or(0);

    info!(
        "[{}] Connection closed (↑ {} bytes, ↓ {} bytes)",
        conn_id, upstream_bytes, downstream_bytes
    );

    Ok(())
}

async fn copy_with_stats<R, W>(
    reader: &mut R,
    writer: &mut W,
    conn_id: u64,
    direction: &str,
) -> io::Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; 8192];
    let mut total = 0u64;

    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        };

        writer.write_all(&buf[..n]).await?;
        writer.flush().await?;
        total += n as u64;

        if total % 1048576 == 0 {
            // Log every MB
            debug!(
                "[{}] {} transferred: {} MB",
                conn_id,
                direction,
                total / 1048576
            );
        }
    }

    Ok(total)
}
