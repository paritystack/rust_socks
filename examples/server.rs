use rust_socks::auth::{MultiAuth, UserPassAuth};
use rust_socks::server::ServerBuilder;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Example 1: Simple server with no authentication
    println!("Starting SOCKS5 server examples...\n");

    // Get configuration from command line or use defaults
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("noauth");

    match mode {
        "noauth" => {
            println!("=== Mode: No Authentication ===");
            println!("Starting SOCKS5 server on 127.0.0.1:1080");
            println!("No authentication required\n");

            let server = ServerBuilder::new()
                .bind_addr("127.0.0.1:1080".parse()?)
                .no_auth()
                .enable_connect(true)
                .enable_bind(true)
                .enable_udp(true)
                .build();

            server.run().await?;
        }

        "userpass" => {
            println!("=== Mode: Username/Password Authentication ===");
            println!("Starting SOCKS5 server on 127.0.0.1:1080");
            println!("Credentials: testuser / testpass\n");

            // Create username/password authenticator
            let auth = UserPassAuth::with_users(vec![
                ("testuser".to_string(), "testpass".to_string()),
                ("admin".to_string(), "admin123".to_string()),
            ]);

            let server = ServerBuilder::new()
                .bind_addr("127.0.0.1:1080".parse()?)
                .authenticator(std::sync::Arc::new(auth))
                .build();

            server.run().await?;
        }

        "multi" => {
            println!("=== Mode: Multiple Authentication Methods ===");
            println!("Starting SOCKS5 server on 127.0.0.1:1080");
            println!("Supports both no-auth and username/password");
            println!("Credentials: testuser / testpass\n");

            // Create multi-auth supporting both no-auth and username/password
            let auth = UserPassAuth::single_user(
                "testuser".to_string(),
                "testpass".to_string(),
            );

            let multi_auth = MultiAuth::new()
                .with_no_auth()
                .with_user_pass(auth);

            let server = ServerBuilder::new()
                .bind_addr("127.0.0.1:1080".parse()?)
                .multi_auth(multi_auth)
                .build();

            server.run().await?;
        }

        "custom" => {
            println!("=== Mode: Custom Configuration ===");
            println!("Starting SOCKS5 server on 0.0.0.0:8080");
            println!("CONNECT: enabled, BIND: disabled, UDP: disabled\n");

            let server = ServerBuilder::new()
                .bind_addr("0.0.0.0:8080".parse()?)
                .no_auth()
                .enable_connect(true)
                .enable_bind(false)
                .enable_udp(false)
                .timeout(60)
                .build();

            server.run().await?;
        }

        _ => {
            println!("Usage: {} [mode]", args[0]);
            println!("\nAvailable modes:");
            println!("  noauth   - No authentication (default)");
            println!("  userpass - Username/password authentication");
            println!("  multi    - Multiple authentication methods");
            println!("  custom   - Custom configuration");
            println!("\nExamples:");
            println!("  {} noauth", args[0]);
            println!("  {} userpass", args[0]);
        }
    }

    Ok(())
}
