use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{debug, info};

use crate::auth::ClientAuth;
use crate::error::{Result, SocksError};
use crate::message;
use crate::protocol::*;

/// SOCKS5 client for making proxied connections
#[derive(Debug, Clone)]
pub struct SocksClient {
    /// SOCKS5 proxy server address
    proxy_addr: String,
    /// Authentication credentials
    auth: ClientAuth,
}

impl SocksClient {
    /// Create a new SOCKS5 client
    pub fn new(proxy_addr: String) -> Self {
        Self {
            proxy_addr,
            auth: ClientAuth::None,
        }
    }

    /// Create a client with username/password authentication
    pub fn with_auth(proxy_addr: String, username: String, password: String) -> Self {
        Self {
            proxy_addr,
            auth: ClientAuth::Password { username, password },
        }
    }

    /// Set authentication credentials
    pub fn set_auth(&mut self, auth: ClientAuth) {
        self.auth = auth;
    }

    /// Connect to a target through the SOCKS5 proxy (CONNECT command)
    pub async fn connect(&self, target: TargetAddr) -> Result<TcpStream> {
        debug!("Connecting to {} via SOCKS5 proxy {}", target, self.proxy_addr);

        // Connect to proxy server
        let mut stream = TcpStream::connect(&self.proxy_addr).await?;
        info!("Connected to proxy server {}", self.proxy_addr);

        // Perform authentication negotiation
        self.authenticate(&mut stream).await?;

        // Send CONNECT request
        message::write_request(&mut stream, Command::Connect, &target).await?;
        debug!("Sent CONNECT request for {}", target);

        // Read reply
        let (reply, bind_addr) = message::read_reply(&mut stream).await?;
        debug!("Received reply: {:?}, bind address: {}", reply, bind_addr);

        if reply != ReplyCode::Succeeded {
            return Err(SocksError::from_reply_code(reply.to_u8()));
        }

        info!("Successfully connected to {} via proxy", target);
        Ok(stream)
    }

    /// Connect to a target using domain name
    pub async fn connect_domain(&self, domain: String, port: u16) -> Result<TcpStream> {
        let target = TargetAddr::new(SocksAddr::Domain(domain), port);
        self.connect(target).await
    }

    /// Connect to a target using IP address
    pub async fn connect_addr(&self, addr: SocketAddr) -> Result<TcpStream> {
        let target = TargetAddr::from_socket_addr(addr);
        self.connect(target).await
    }

    /// Setup a BIND connection through the proxy
    pub async fn bind(&self, target: TargetAddr) -> Result<(TcpStream, TargetAddr)> {
        debug!("Setting up BIND to {} via SOCKS5 proxy {}", target, self.proxy_addr);

        // Connect to proxy server
        let mut stream = TcpStream::connect(&self.proxy_addr).await?;

        // Perform authentication
        self.authenticate(&mut stream).await?;

        // Send BIND request
        message::write_request(&mut stream, Command::Bind, &target).await?;
        debug!("Sent BIND request for {}", target);

        // Read first reply with bind address
        let (reply, bind_addr) = message::read_reply(&mut stream).await?;
        if reply != ReplyCode::Succeeded {
            return Err(SocksError::from_reply_code(reply.to_u8()));
        }
        info!("BIND listening on {}", bind_addr);

        // Read second reply with incoming connection address
        let (reply, incoming_addr) = message::read_reply(&mut stream).await?;
        if reply != ReplyCode::Succeeded {
            return Err(SocksError::from_reply_code(reply.to_u8()));
        }
        info!("BIND accepted connection from {}", incoming_addr);

        Ok((stream, incoming_addr))
    }

    /// Setup UDP ASSOCIATE through the proxy
    pub async fn udp_associate(&self, addr: TargetAddr) -> Result<(TcpStream, TargetAddr)> {
        debug!("Setting up UDP ASSOCIATE via SOCKS5 proxy {}", self.proxy_addr);

        // Connect to proxy server
        let mut stream = TcpStream::connect(&self.proxy_addr).await?;

        // Perform authentication
        self.authenticate(&mut stream).await?;

        // Send UDP ASSOCIATE request
        message::write_request(&mut stream, Command::UdpAssociate, &addr).await?;
        debug!("Sent UDP ASSOCIATE request");

        // Read reply
        let (reply, relay_addr) = message::read_reply(&mut stream).await?;
        if reply != ReplyCode::Succeeded {
            return Err(SocksError::from_reply_code(reply.to_u8()));
        }
        info!("UDP relay available at {}", relay_addr);

        Ok((stream, relay_addr))
    }

    /// Perform authentication with the proxy server
    async fn authenticate(&self, stream: &mut TcpStream) -> Result<()> {
        // Send authentication methods
        let methods = self.auth.preferred_methods();
        message::write_auth_request(stream, &methods).await?;
        debug!("Sent auth methods: {:?}", methods);

        // Read selected method
        let selected = message::read_auth_response(stream).await?;
        debug!("Server selected auth method: {:?}", selected);

        if selected == AuthMethod::NoAcceptable {
            return Err(SocksError::NoAcceptableMethods);
        }

        // Perform authentication based on selected method
        match selected {
            AuthMethod::NoAuth => {
                debug!("No authentication required");
                Ok(())
            }
            AuthMethod::UserPass => {
                if let Some((username, password)) = self.auth.credentials() {
                    message::write_userpass_request(stream, username, password).await?;
                    debug!("Sent username/password authentication");

                    let status = message::read_userpass_response(stream).await?;
                    if status == AuthStatus::Success {
                        info!("Authentication successful");
                        Ok(())
                    } else {
                        Err(SocksError::AuthenticationFailed("Invalid credentials".to_string()))
                    }
                } else {
                    Err(SocksError::AuthenticationFailed(
                        "Server requires username/password but none provided".to_string(),
                    ))
                }
            }
            _ => Err(SocksError::InvalidAuthMethod(selected.to_u8())),
        }
    }
}

/// Builder for creating a SOCKS5 client
pub struct ClientBuilder {
    proxy_addr: String,
    auth: ClientAuth,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new(proxy_addr: String) -> Self {
        Self {
            proxy_addr,
            auth: ClientAuth::None,
        }
    }

    /// Set no authentication
    pub fn no_auth(mut self) -> Self {
        self.auth = ClientAuth::None;
        self
    }

    /// Set username/password authentication
    pub fn with_credentials(mut self, username: String, password: String) -> Self {
        self.auth = ClientAuth::Password { username, password };
        self
    }

    /// Set custom authentication
    pub fn with_auth(mut self, auth: ClientAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Build the client
    pub fn build(self) -> SocksClient {
        SocksClient {
            proxy_addr: self.proxy_addr,
            auth: self.auth,
        }
    }
}

/// Helper function to create a SOCKS5 connection
pub async fn connect_via_socks5(
    proxy_addr: String,
    target: TargetAddr,
    auth: Option<(String, String)>,
) -> Result<TcpStream> {
    let client = match auth {
        Some((username, password)) => SocksClient::with_auth(proxy_addr, username, password),
        None => SocksClient::new(proxy_addr),
    };

    client.connect(target).await
}

/// Helper function to create a SOCKS5 connection to a domain
pub async fn connect_domain_via_socks5(
    proxy_addr: String,
    domain: String,
    port: u16,
    auth: Option<(String, String)>,
) -> Result<TcpStream> {
    let client = match auth {
        Some((username, password)) => SocksClient::with_auth(proxy_addr, username, password),
        None => SocksClient::new(proxy_addr),
    };

    client.connect_domain(domain, port).await
}

/// Helper function to create a SOCKS5 connection to an IP address
pub async fn connect_addr_via_socks5(
    proxy_addr: String,
    addr: SocketAddr,
    auth: Option<(String, String)>,
) -> Result<TcpStream> {
    let client = match auth {
        Some((username, password)) => SocksClient::with_auth(proxy_addr, username, password),
        None => SocksClient::new(proxy_addr),
    };

    client.connect_addr(addr).await
}
