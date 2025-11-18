//! # Rust SOCKS5 Library
//!
//! A comprehensive, async SOCKS5 implementation in Rust providing both server and client functionality.
//!
//! ## Features
//!
//! - **Full SOCKS5 Protocol Support**: CONNECT, BIND, and UDP ASSOCIATE commands
//! - **Authentication Methods**: No authentication and Username/Password authentication
//! - **Async/Await**: Built on Tokio for high-performance async I/O
//! - **Flexible Architecture**: Modular design with trait-based authentication
//! - **Server and Client**: Complete implementation of both server and client
//!
//! ## Quick Start
//!
//! ### Server Example
//!
//! ```rust,no_run
//! use rust_socks::server::ServerBuilder;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = ServerBuilder::new()
//!         .bind_addr("127.0.0.1:1080".parse()?)
//!         .no_auth()
//!         .build();
//!
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Client Example
//!
//! ```rust,no_run
//! use rust_socks::client::SocksClient;
//! use rust_socks::protocol::{SocksAddr, TargetAddr};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = SocksClient::new("127.0.0.1:1080".to_string());
//!     let target = TargetAddr::new(
//!         SocksAddr::Domain("example.com".to_string()),
//!         80
//!     );
//!
//!     let mut stream = client.connect(target).await?;
//!     // Use stream for communication
//!     Ok(())
//! }
//! ```
//!
//! ## Authentication
//!
//! The library supports multiple authentication methods:
//!
//! ### No Authentication
//!
//! ```rust,no_run
//! use rust_socks::server::ServerBuilder;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let server = ServerBuilder::new()
//!     .no_auth()
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! ### Username/Password Authentication
//!
//! ```rust,no_run
//! use rust_socks::auth::{UserPassAuth, MultiAuth};
//! use rust_socks::server::ServerBuilder;
//! use std::sync::Arc;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let auth = UserPassAuth::single_user(
//!     "username".to_string(),
//!     "password".to_string()
//! );
//!
//! let server = ServerBuilder::new()
//!     .authenticator(Arc::new(auth))
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! ## Client with Authentication
//!
//! ```rust,no_run
//! use rust_socks::client::ClientBuilder;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClientBuilder::new("127.0.0.1:1080".to_string())
//!     .with_credentials("username".to_string(), "password".to_string())
//!     .build();
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod auth;
pub mod client;
pub mod error;
pub mod message;
pub mod protocol;
pub mod server;

// Re-export commonly used types
pub use error::{Result, SocksError};
pub use protocol::{
    AuthMethod, Command, ReplyCode, SocksAddr, TargetAddr,
    SOCKS_VERSION,
};
pub use server::{ServerBuilder, ServerConfig, SocksServer};
pub use client::{ClientBuilder, SocksClient};
pub use auth::{
    Authenticator, ClientAuth, MultiAuth, NoAuth, UserPassAuth,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::auth::{Authenticator, ClientAuth, MultiAuth, NoAuth, UserPassAuth};
    pub use crate::client::{ClientBuilder, SocksClient};
    pub use crate::error::{Result, SocksError};
    pub use crate::protocol::{
        AuthMethod, Command, ReplyCode, SocksAddr, TargetAddr,
    };
    pub use crate::server::{ServerBuilder, ServerConfig, SocksServer};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_socks_addr_display() {
        use std::net::Ipv4Addr;
        let addr = SocksAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(format!("{}", addr), "127.0.0.1");

        let addr = SocksAddr::Domain("example.com".to_string());
        assert_eq!(format!("{}", addr), "example.com");
    }

    #[test]
    fn test_target_addr_display() {
        use std::net::Ipv4Addr;
        let target = TargetAddr::new(
            SocksAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        );
        assert_eq!(format!("{}", target), "127.0.0.1:8080");
    }

    #[test]
    fn test_auth_method_conversion() {
        assert_eq!(AuthMethod::NoAuth.to_u8(), 0x00);
        assert_eq!(AuthMethod::UserPass.to_u8(), 0x02);
        assert_eq!(AuthMethod::from_u8(0x00), AuthMethod::NoAuth);
        assert_eq!(AuthMethod::from_u8(0x02), AuthMethod::UserPass);
    }

    #[test]
    fn test_command_conversion() {
        use crate::protocol::Command;
        assert_eq!(Command::Connect.to_u8(), 0x01);
        assert_eq!(Command::Bind.to_u8(), 0x02);
        assert_eq!(Command::UdpAssociate.to_u8(), 0x03);
    }
}
