use clap::Parser;
use rust_socks::client::{ClientBuilder, SocksClient};
use rust_socks::protocol::{SocksAddr, TargetAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, debug};
use tracing_subscriber;

/// SOCKS5 client CLI tool for testing and making requests through proxies
#[derive(Parser)]
#[command(author, version, about = "SOCKS5 client for making proxied connections", long_about = None)]
struct Cli {
    /// SOCKS5 proxy address (e.g., 127.0.0.1:1080)
    #[arg(short, long, default_value = "127.0.0.1:1080")]
    proxy: String,

    /// Target address (domain or IP)
    #[arg(short, long)]
    target: String,

    /// Target port
    #[arg(short = 'P', long)]
    port: u16,

    /// Username for authentication
    #[arg(short, long)]
    username: Option<String>,

    /// Password for authentication
    #[arg(short = 'w', long)]
    password: Option<String>,

    /// HTTP request mode - send HTTP GET request
    #[arg(long)]
    http: bool,

    /// Custom HTTP path (default: /)
    #[arg(long, default_value = "/")]
    http_path: String,

    /// Read response (number of bytes, 0 for all)
    #[arg(short, long, default_value = "4096")]
    read: usize,

    /// Send custom data
    #[arg(short, long)]
    send: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Test connection only (don't send/receive data)
    #[arg(long)]
    test_only: bool,
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

    info!("SOCKS5 Client");
    info!("Proxy: {}", cli.proxy);
    info!("Target: {}:{}", cli.target, cli.port);

    // Create client
    let client = if let (Some(username), Some(password)) = (cli.username, cli.password) {
        info!("Using authentication: {}", username);
        ClientBuilder::new(cli.proxy.clone())
            .with_credentials(username, password)
            .build()
    } else {
        info!("No authentication");
        SocksClient::new(cli.proxy.clone())
    };

    // Determine target address type
    let target_addr = if cli.target.parse::<std::net::IpAddr>().is_ok() {
        let ip: std::net::IpAddr = cli.target.parse()?;
        match ip {
            std::net::IpAddr::V4(ipv4) => TargetAddr::new(SocksAddr::V4(ipv4), cli.port),
            std::net::IpAddr::V6(ipv6) => TargetAddr::new(SocksAddr::V6(ipv6), cli.port),
        }
    } else {
        info!("Resolving domain: {}", cli.target);
        TargetAddr::new(SocksAddr::Domain(cli.target.clone()), cli.port)
    };

    // Connect through proxy
    info!("Connecting to {}...", target_addr);
    let mut stream = client.connect(target_addr).await?;
    info!("✓ Connected successfully!");

    if cli.test_only {
        info!("Test completed successfully");
        return Ok(());
    }

    // Send data
    if cli.http {
        // Send HTTP request
        let http_request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            cli.http_path, cli.target
        );
        info!("Sending HTTP GET request for {}", cli.http_path);
        debug!("Request:\n{}", http_request);
        stream.write_all(http_request.as_bytes()).await?;
        stream.flush().await?;
        info!("✓ Request sent");
    } else if let Some(data) = cli.send {
        // Send custom data
        info!("Sending custom data ({} bytes)", data.len());
        stream.write_all(data.as_bytes()).await?;
        stream.flush().await?;
        info!("✓ Data sent");
    }

    // Read response
    if cli.read > 0 {
        info!("Reading response...");
        let mut buffer = if cli.read == 0 {
            Vec::new()
        } else {
            vec![0u8; cli.read]
        };

        let bytes_read = if cli.read == 0 {
            stream.read_to_end(&mut buffer).await?
        } else {
            stream.read(&mut buffer).await?
        };

        info!("✓ Received {} bytes", bytes_read);

        // Try to display as UTF-8
        match String::from_utf8(buffer[..bytes_read].to_vec()) {
            Ok(text) => {
                println!("\n--- Response ({} bytes) ---", bytes_read);
                println!("{}", text);
                println!("--- End Response ---");
            }
            Err(_) => {
                println!("\n--- Binary Response ({} bytes) ---", bytes_read);
                println!("{:02X?}", &buffer[..bytes_read.min(256)]);
                if bytes_read > 256 {
                    println!("... ({} more bytes)", bytes_read - 256);
                }
                println!("--- End Response ---");
            }
        }
    }

    info!("Connection completed successfully");
    Ok(())
}
