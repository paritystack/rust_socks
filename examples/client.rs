use rust_socks::client::{ClientBuilder, SocksClient};
use rust_socks::protocol::{SocksAddr, TargetAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("SOCKS5 Client Examples\n");

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("connect");

    match mode {
        "connect" => {
            println!("=== Example 1: Simple CONNECT ===");
            println!("Connecting to example.com:80 via SOCKS5 proxy\n");

            // Create a simple client without authentication
            let client = SocksClient::new("127.0.0.1:1080".to_string());

            // Connect to example.com:80
            let target = TargetAddr::new(
                SocksAddr::Domain("example.com".to_string()),
                80,
            );

            match client.connect(target).await {
                Ok(mut stream) => {
                    println!("✓ Successfully connected to example.com:80");

                    // Send a simple HTTP GET request
                    let request = "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
                    stream.write_all(request.as_bytes()).await?;
                    println!("✓ Sent HTTP GET request");

                    // Read response
                    let mut buffer = vec![0u8; 1024];
                    let n = stream.read(&mut buffer).await?;
                    let response = String::from_utf8_lossy(&buffer[..n]);
                    println!("\n--- Response (first 1024 bytes) ---");
                    println!("{}", response);
                }
                Err(e) => {
                    eprintln!("✗ Failed to connect: {}", e);
                    eprintln!("Make sure a SOCKS5 server is running on 127.0.0.1:1080");
                }
            }
        }

        "auth" => {
            println!("=== Example 2: CONNECT with Authentication ===");
            println!("Connecting with username/password authentication\n");

            // Create a client with username/password authentication
            let client = ClientBuilder::new("127.0.0.1:1080".to_string())
                .with_credentials("testuser".to_string(), "testpass".to_string())
                .build();

            let target = TargetAddr::new(
                SocksAddr::Domain("example.com".to_string()),
                80,
            );

            match client.connect(target).await {
                Ok(_stream) => {
                    println!("✓ Successfully authenticated and connected");
                }
                Err(e) => {
                    eprintln!("✗ Failed: {}", e);
                    eprintln!("Make sure a SOCKS5 server with username/password auth is running");
                    eprintln!("Credentials: testuser / testpass");
                }
            }
        }

        "ip" => {
            println!("=== Example 3: Connect to IP Address ===");
            println!("Connecting to 1.1.1.1:80 (Cloudflare DNS over HTTP)\n");

            let client = SocksClient::new("127.0.0.1:1080".to_string());

            // Connect using IP address
            let addr = "1.1.1.1:80".parse()?;
            match client.connect_addr(addr).await {
                Ok(_stream) => {
                    println!("✓ Successfully connected to 1.1.1.1:80");
                }
                Err(e) => {
                    eprintln!("✗ Failed to connect: {}", e);
                }
            }
        }

        "domain" => {
            println!("=== Example 4: Connect to Domain (Helper Method) ===");
            println!("Connecting to httpbin.org:80\n");

            let client = SocksClient::new("127.0.0.1:1080".to_string());

            match client.connect_domain("httpbin.org".to_string(), 80).await {
                Ok(mut stream) => {
                    println!("✓ Successfully connected to httpbin.org:80");

                    // Test with httpbin.org/ip endpoint
                    let request = "GET /ip HTTP/1.1\r\nHost: httpbin.org\r\nConnection: close\r\n\r\n";
                    stream.write_all(request.as_bytes()).await?;

                    let mut buffer = String::new();
                    stream.read_to_string(&mut buffer).await?;
                    println!("\n--- Response ---");
                    println!("{}", buffer);
                }
                Err(e) => {
                    eprintln!("✗ Failed to connect: {}", e);
                }
            }
        }

        "bind" => {
            println!("=== Example 5: BIND Command ===");
            println!("Setting up BIND for incoming connection\n");

            let client = SocksClient::new("127.0.0.1:1080".to_string());

            let target = TargetAddr::new(
                SocksAddr::Domain("0.0.0.0".to_string()),
                0,
            );

            match client.bind(target).await {
                Ok((_stream, incoming_addr)) => {
                    println!("✓ BIND successful");
                    println!("Incoming connection from: {}", incoming_addr);
                }
                Err(e) => {
                    eprintln!("✗ BIND failed: {}", e);
                }
            }
        }

        "udp" => {
            println!("=== Example 6: UDP ASSOCIATE ===");
            println!("Setting up UDP association\n");

            let client = SocksClient::new("127.0.0.1:1080".to_string());

            let target = TargetAddr::new(
                SocksAddr::Domain("0.0.0.0".to_string()),
                0,
            );

            match client.udp_associate(target).await {
                Ok((_stream, relay_addr)) => {
                    println!("✓ UDP ASSOCIATE successful");
                    println!("UDP relay address: {}", relay_addr);
                    println!("Note: UDP relay will remain active while TCP connection is open");

                    // Keep connection alive for demonstration
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    println!("Closing UDP association...");
                }
                Err(e) => {
                    eprintln!("✗ UDP ASSOCIATE failed: {}", e);
                }
            }
        }

        _ => {
            println!("Usage: {} [mode]", args[0]);
            println!("\nAvailable modes:");
            println!("  connect  - Simple CONNECT to example.com (default)");
            println!("  auth     - CONNECT with username/password authentication");
            println!("  ip       - CONNECT to IP address");
            println!("  domain   - CONNECT to domain using helper method");
            println!("  bind     - BIND command example");
            println!("  udp      - UDP ASSOCIATE example");
            println!("\nExamples:");
            println!("  {} connect", args[0]);
            println!("  {} auth", args[0]);
            println!("  {} domain", args[0]);
        }
    }

    Ok(())
}
