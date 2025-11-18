use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

use crate::auth::{Authenticator, MultiAuth, NoAuth};
use crate::error::{Result, SocksError};
use crate::message;
use crate::protocol::*;

/// SOCKS5 server configuration
#[derive(Clone)]
pub struct ServerConfig {
    /// Bind address for the server
    pub bind_addr: SocketAddr,
    /// Authenticator for handling authentication
    pub authenticator: Arc<dyn Authenticator>,
    /// Enable CONNECT command
    pub enable_connect: bool,
    /// Enable BIND command
    pub enable_bind: bool,
    /// Enable UDP ASSOCIATE command
    pub enable_udp: bool,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:1080".parse().unwrap(),
            authenticator: Arc::new(NoAuth),
            enable_connect: true,
            enable_bind: true,
            enable_udp: true,
            timeout_secs: 300,
        }
    }
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            ..Default::default()
        }
    }

    /// Set the authenticator
    pub fn with_authenticator(mut self, auth: Arc<dyn Authenticator>) -> Self {
        self.authenticator = auth;
        self
    }

    /// Enable/disable CONNECT command
    pub fn with_connect(mut self, enable: bool) -> Self {
        self.enable_connect = enable;
        self
    }

    /// Enable/disable BIND command
    pub fn with_bind(mut self, enable: bool) -> Self {
        self.enable_bind = enable;
        self
    }

    /// Enable/disable UDP ASSOCIATE command
    pub fn with_udp(mut self, enable: bool) -> Self {
        self.enable_udp = enable;
        self
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// SOCKS5 server
pub struct SocksServer {
    config: ServerConfig,
}

impl SocksServer {
    /// Create a new SOCKS5 server with configuration
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    /// Create a new SOCKS5 server with default configuration
    pub fn with_bind_addr(bind_addr: SocketAddr) -> Self {
        Self::new(ServerConfig::new(bind_addr))
    }

    /// Run the server
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind_addr).await?;
        info!("SOCKS5 server listening on {}", self.config.bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New connection from {}", addr);
                    let config = self.config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, addr, config).await {
                            error!("Error handling client {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

/// Handle a client connection
async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    config: ServerConfig,
) -> Result<()> {
    // 1. Authentication negotiation
    let methods = message::read_auth_request(&mut stream).await?;
    debug!("Client {} offered auth methods: {:?}", addr, methods);

    let supported = config.authenticator.supported_methods();
    let selected_method = methods
        .iter()
        .find(|m| supported.contains(m))
        .copied()
        .unwrap_or(AuthMethod::NoAcceptable);

    if selected_method == AuthMethod::NoAcceptable {
        message::write_auth_response(&mut stream, AuthMethod::NoAcceptable).await?;
        return Err(SocksError::NoAcceptableMethods);
    }

    message::write_auth_response(&mut stream, selected_method).await?;
    debug!("Selected auth method for {}: {:?}", addr, selected_method);

    // 2. Perform authentication
    match selected_method {
        AuthMethod::NoAuth => {
            // No authentication needed
            debug!("No authentication required for {}", addr);
        }
        AuthMethod::UserPass => {
            let (username, password) = message::read_userpass_request(&mut stream).await?;
            debug!("Username/password auth attempt from {} as {}", addr, username);

            let authenticated = config.authenticator.authenticate(&username, &password).await?;

            if authenticated {
                message::write_userpass_response(&mut stream, AuthStatus::Success).await?;
                info!("User {} authenticated successfully from {}", username, addr);
            } else {
                message::write_userpass_response(&mut stream, AuthStatus::Failure).await?;
                return Err(SocksError::AuthenticationFailed(format!(
                    "Invalid credentials for user {}",
                    username
                )));
            }
        }
        _ => {
            return Err(SocksError::InvalidAuthMethod(selected_method.to_u8()));
        }
    }

    // 3. Process SOCKS5 request
    let (command, target) = message::read_request(&mut stream).await?;
    info!("Client {} requested {:?} to {}", addr, command, target);

    match command {
        Command::Connect if config.enable_connect => {
            handle_connect(stream, addr, target).await
        }
        Command::Bind if config.enable_bind => {
            handle_bind(stream, addr, target).await
        }
        Command::UdpAssociate if config.enable_udp => {
            handle_udp_associate(stream, addr, target).await
        }
        _ => {
            let reply = ReplyCode::CommandNotSupported;
            let bind_addr = TargetAddr::new(SocksAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0);
            message::write_reply(&mut stream, reply, &bind_addr).await?;
            Err(SocksError::CommandNotSupported(format!("{:?}", command)))
        }
    }
}

/// Handle CONNECT command
async fn handle_connect(
    mut client: TcpStream,
    client_addr: SocketAddr,
    target: TargetAddr,
) -> Result<()> {
    debug!("Connecting to target: {}", target);

    // Resolve and connect to target
    let target_stream = match &target.addr {
        SocksAddr::V4(ip) => {
            let addr = SocketAddr::new(std::net::IpAddr::V4(*ip), target.port);
            TcpStream::connect(addr).await
        }
        SocksAddr::V6(ip) => {
            let addr = SocketAddr::new(std::net::IpAddr::V6(*ip), target.port);
            TcpStream::connect(addr).await
        }
        SocksAddr::Domain(domain) => {
            let addr_str = format!("{}:{}", domain, target.port);
            TcpStream::connect(addr_str).await
        }
    };

    let mut target_stream = match target_stream {
        Ok(stream) => stream,
        Err(e) => {
            warn!("Failed to connect to {}: {}", target, e);
            let reply = match e.kind() {
                io::ErrorKind::ConnectionRefused => ReplyCode::ConnectionRefused,
                io::ErrorKind::TimedOut => ReplyCode::TtlExpired,
                _ => ReplyCode::HostUnreachable,
            };
            let bind_addr = TargetAddr::new(SocksAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0);
            message::write_reply(&mut client, reply, &bind_addr).await?;
            return Err(SocksError::from(e));
        }
    };

    // Get the local address we connected from
    let local_addr = target_stream.local_addr()?;
    let bind_addr = TargetAddr::from_socket_addr(local_addr);

    // Send success reply
    message::write_reply(&mut client, ReplyCode::Succeeded, &bind_addr).await?;
    info!("Connected {} to {}", client_addr, target);

    // Relay data between client and target
    let (client_read, client_write) = client.split();
    let (target_read, target_write) = target_stream.split();

    let client_to_target = copy_bidirectional(client_read, target_write);
    let target_to_client = copy_bidirectional(target_read, client_write);

    tokio::select! {
        result = client_to_target => {
            if let Err(e) = result {
                debug!("Client to target relay error: {}", e);
            }
        }
        result = target_to_client => {
            if let Err(e) = result {
                debug!("Target to client relay error: {}", e);
            }
        }
    }

    info!("Connection closed: {} <-> {}", client_addr, target);
    Ok(())
}

/// Handle BIND command
async fn handle_bind(
    mut client: TcpStream,
    client_addr: SocketAddr,
    target: TargetAddr,
) -> Result<()> {
    debug!("BIND request from {} for {}", client_addr, target);

    // Create a listener on a random port
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let bind_addr = listener.local_addr()?;
    let bind_target = TargetAddr::from_socket_addr(bind_addr);

    // Send first reply with bind address
    message::write_reply(&mut client, ReplyCode::Succeeded, &bind_target).await?;
    info!("BIND listening on {} for {}", bind_addr, target);

    // Wait for incoming connection
    let (mut target_stream, incoming_addr) = listener.accept().await?;
    let incoming_target = TargetAddr::from_socket_addr(incoming_addr);

    // Send second reply with incoming connection address
    message::write_reply(&mut client, ReplyCode::Succeeded, &incoming_target).await?;
    info!("BIND accepted connection from {}", incoming_addr);

    // Relay data
    let (client_read, client_write) = client.split();
    let (target_read, target_write) = target_stream.split();

    let client_to_target = copy_bidirectional(client_read, target_write);
    let target_to_client = copy_bidirectional(target_read, client_write);

    tokio::select! {
        result = client_to_target => {
            if let Err(e) = result {
                debug!("Client to target relay error: {}", e);
            }
        }
        result = target_to_client => {
            if let Err(e) = result {
                debug!("Target to client relay error: {}", e);
            }
        }
    }

    info!("BIND connection closed");
    Ok(())
}

/// Handle UDP ASSOCIATE command
async fn handle_udp_associate(
    mut client: TcpStream,
    client_addr: SocketAddr,
    target: TargetAddr,
) -> Result<()> {
    debug!("UDP ASSOCIATE request from {} for {}", client_addr, target);

    // For UDP ASSOCIATE, we need to bind a UDP socket
    // This is a simplified implementation that just keeps the TCP connection alive
    let udp_addr = SocketAddr::new(client_addr.ip(), 0);
    let _udp_socket = tokio::net::UdpSocket::bind(udp_addr).await?;
    let local_addr = _udp_socket.local_addr()?;
    let bind_target = TargetAddr::from_socket_addr(local_addr);

    // Send success reply with UDP relay address
    message::write_reply(&mut client, ReplyCode::Succeeded, &bind_target).await?;
    info!("UDP ASSOCIATE relay on {} for {}", local_addr, target);

    // Keep the TCP connection alive - when it closes, UDP association ends
    let mut buf = [0u8; 1];
    loop {
        match client.read(&mut buf).await {
            Ok(0) => break, // Connection closed
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    info!("UDP ASSOCIATE closed for {}", client_addr);
    Ok(())
}

/// Copy data bidirectionally between reader and writer
async fn copy_bidirectional<R, W>(mut reader: R, mut writer: W) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    io::copy(&mut reader, &mut writer).await?;
    Ok(())
}

/// Builder for creating a SOCKS5 server
pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
        }
    }

    /// Set bind address
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.config.bind_addr = addr;
        self
    }

    /// Set authenticator
    pub fn authenticator(mut self, auth: Arc<dyn Authenticator>) -> Self {
        self.config.authenticator = auth;
        self
    }

    /// Set multi-authenticator
    pub fn multi_auth(mut self, auth: MultiAuth) -> Self {
        self.config.authenticator = Arc::new(auth);
        self
    }

    /// Enable no authentication
    pub fn no_auth(mut self) -> Self {
        self.config.authenticator = Arc::new(NoAuth);
        self
    }

    /// Enable CONNECT command
    pub fn enable_connect(mut self, enable: bool) -> Self {
        self.config.enable_connect = enable;
        self
    }

    /// Enable BIND command
    pub fn enable_bind(mut self, enable: bool) -> Self {
        self.config.enable_bind = enable;
        self
    }

    /// Enable UDP ASSOCIATE command
    pub fn enable_udp(mut self, enable: bool) -> Self {
        self.config.enable_udp = enable;
        self
    }

    /// Set connection timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.config.timeout_secs = secs;
        self
    }

    /// Build the server
    pub fn build(self) -> SocksServer {
        SocksServer::new(self.config)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
