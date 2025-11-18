use rust_socks::client::SocksClient;
use rust_socks::protocol::{SocksAddr, TargetAddr};
use rust_socks::server::ServerBuilder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing_subscriber;

/// This example demonstrates chaining multiple SOCKS5 proxies together
///
/// Architecture:
/// Client -> Proxy1 (1081) -> Proxy2 (1082) -> Proxy3 (1083) -> Target
///
/// This is useful for:
/// - Multi-hop anonymity
/// - Geographic routing
/// - Load distribution
/// - Network segmentation

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== SOCKS5 Proxy Chain Example ===\n");

    // Start three SOCKS5 servers in the background
    println!("Starting proxy chain...");

    // Proxy 1 on port 1081
    tokio::spawn(async {
        let server = ServerBuilder::new()
            .bind_addr("127.0.0.1:1081".parse().unwrap())
            .no_auth()
            .build();

        if let Err(e) = server.run().await {
            eprintln!("Proxy 1 error: {}", e);
        }
    });

    // Proxy 2 on port 1082
    tokio::spawn(async {
        let server = ServerBuilder::new()
            .bind_addr("127.0.0.1:1082".parse().unwrap())
            .no_auth()
            .build();

        if let Err(e) = server.run().await {
            eprintln!("Proxy 2 error: {}", e);
        }
    });

    // Proxy 3 on port 1083
    tokio::spawn(async {
        let server = ServerBuilder::new()
            .bind_addr("127.0.0.1:1083".parse().unwrap())
            .no_auth()
            .build();

        if let Err(e) = server.run().await {
            eprintln!("Proxy 3 error: {}", e);
        }
    });

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("✓ Proxy 1 started on 127.0.0.1:1081");
    println!("✓ Proxy 2 started on 127.0.0.1:1082");
    println!("✓ Proxy 3 started on 127.0.0.1:1083\n");

    // Now create a chain: Client -> Proxy1 -> Proxy2 -> Proxy3 -> example.com
    println!("Building proxy chain:");
    println!("  Client -> Proxy1 (1081) -> Proxy2 (1082) -> Proxy3 (1083) -> example.com:80\n");

    // Step 1: Connect to Proxy1
    let client1 = SocksClient::new("127.0.0.1:1081".to_string());

    // Step 2: Through Proxy1, connect to Proxy2
    let target_proxy2 = TargetAddr::new(
        SocksAddr::Domain("127.0.0.1".to_string()),
        1082,
    );

    println!("→ Connecting to Proxy2 via Proxy1...");
    let mut stream_to_proxy2 = client1.connect(target_proxy2).await?;
    println!("✓ Connected to Proxy2 through Proxy1");

    // Step 3: Now use this connection as a SOCKS5 client to connect to Proxy3
    println!("→ Connecting to Proxy3 via Proxy2...");

    // Manually perform SOCKS5 handshake to connect to Proxy3
    // Send auth request (no auth)
    stream_to_proxy2.write_all(&[0x05, 0x01, 0x00]).await?;

    // Read auth response
    let mut buf = [0u8; 2];
    stream_to_proxy2.read_exact(&mut buf).await?;

    if buf[1] != 0x00 {
        eprintln!("Proxy2 authentication failed");
        return Ok(());
    }
    println!("✓ Authenticated with Proxy2");

    // Send CONNECT request to Proxy3 (127.0.0.1:1083)
    stream_to_proxy2.write_all(&[
        0x05, // Version
        0x01, // CONNECT
        0x00, // Reserved
        0x01, // IPv4
        127, 0, 0, 1, // 127.0.0.1
        0x04, 0x3B, // Port 1083 (big-endian)
    ]).await?;

    // Read CONNECT response
    let mut response = [0u8; 10];
    stream_to_proxy2.read_exact(&mut response).await?;

    if response[1] != 0x00 {
        eprintln!("Failed to connect to Proxy3");
        return Ok(());
    }
    println!("✓ Connected to Proxy3 through Proxy2");

    // Step 4: Now connect to final destination (example.com:80) through Proxy3
    println!("→ Connecting to example.com:80 via Proxy3...");

    // Send auth request to Proxy3
    stream_to_proxy2.write_all(&[0x05, 0x01, 0x00]).await?;

    // Read auth response
    stream_to_proxy2.read_exact(&mut buf).await?;
    println!("✓ Authenticated with Proxy3");

    // Send CONNECT request to example.com:80
    let domain = b"example.com";
    let mut connect_request = vec![
        0x05, // Version
        0x01, // CONNECT
        0x00, // Reserved
        0x03, // Domain name
        domain.len() as u8, // Domain length
    ];
    connect_request.extend_from_slice(domain);
    connect_request.extend_from_slice(&[0x00, 0x50]); // Port 80 (big-endian)

    stream_to_proxy2.write_all(&connect_request).await?;

    // Read CONNECT response
    let mut header = [0u8; 4];
    stream_to_proxy2.read_exact(&mut header).await?;

    // Read address based on type
    match header[3] {
        0x01 => {
            // IPv4
            let mut addr = [0u8; 6]; // 4 bytes IP + 2 bytes port
            stream_to_proxy2.read_exact(&mut addr).await?;
        }
        0x03 => {
            // Domain
            let mut len = [0u8; 1];
            stream_to_proxy2.read_exact(&mut len).await?;
            let mut addr = vec![0u8; len[0] as usize + 2]; // domain + port
            stream_to_proxy2.read_exact(&mut addr).await?;
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 18]; // 16 bytes IP + 2 bytes port
            stream_to_proxy2.read_exact(&mut addr).await?;
        }
        _ => {}
    }

    if header[1] != 0x00 {
        eprintln!("Failed to connect to example.com");
        return Ok(());
    }
    println!("✓ Connected to example.com:80 through proxy chain!\n");

    // Step 5: Send HTTP request
    println!("Sending HTTP GET request...");
    let request = "GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
    stream_to_proxy2.write_all(request.as_bytes()).await?;

    // Read response
    let mut response_buf = vec![0u8; 2048];
    let n = stream_to_proxy2.read(&mut response_buf).await?;
    let response_text = String::from_utf8_lossy(&response_buf[..n]);

    println!("✓ Received response (first 2048 bytes):\n");
    println!("--- Response ---");
    println!("{}", response_text);
    println!("--- End Response ---\n");

    println!("=== Proxy Chain Summary ===");
    println!("Successfully routed traffic through 3 SOCKS5 proxies!");
    println!("Chain: Client → Proxy1 → Proxy2 → Proxy3 → example.com");
    println!("\nThis demonstrates:");
    println!("  • Chaining multiple SOCKS5 proxies");
    println!("  • Manual SOCKS5 protocol implementation");
    println!("  • Multi-hop proxy routing");

    Ok(())
}
