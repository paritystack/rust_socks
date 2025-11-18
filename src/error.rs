use std::io;
use thiserror::Error;

/// Result type for SOCKS5 operations
pub type Result<T> = std::result::Result<T, SocksError>;

/// SOCKS5 error types
#[derive(Error, Debug)]
pub enum SocksError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid SOCKS version: expected 5, got {0}")]
    InvalidVersion(u8),

    #[error("Invalid command: {0}")]
    InvalidCommand(u8),

    #[error("Invalid address type: {0}")]
    InvalidAddressType(u8),

    #[error("Invalid authentication method: {0}")]
    InvalidAuthMethod(u8),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("No acceptable authentication methods")]
    NoAcceptableMethods,

    #[error("Connection refused by SOCKS server")]
    ConnectionRefused,

    #[error("Network unreachable")]
    NetworkUnreachable,

    #[error("Host unreachable")]
    HostUnreachable,

    #[error("Connection refused by destination")]
    ConnectionRefusedByDestination,

    #[error("TTL expired")]
    TtlExpired,

    #[error("Command not supported: {0}")]
    CommandNotSupported(String),

    #[error("Address type not supported: {0}")]
    AddressTypeNotSupported(String),

    #[error("General SOCKS server failure")]
    GeneralFailure,

    #[error("Invalid reply code: {0}")]
    InvalidReplyCode(u8),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Incomplete data")]
    IncompleteData,

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Address parse error: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),

    #[error("Custom error: {0}")]
    Custom(String),
}

impl SocksError {
    /// Convert error to SOCKS5 reply code
    pub fn to_reply_code(&self) -> u8 {
        match self {
            SocksError::ConnectionRefused | SocksError::ConnectionRefusedByDestination => 0x05,
            SocksError::NetworkUnreachable => 0x03,
            SocksError::HostUnreachable => 0x04,
            SocksError::TtlExpired => 0x06,
            SocksError::CommandNotSupported(_) => 0x07,
            SocksError::AddressTypeNotSupported(_) => 0x08,
            _ => 0x01, // General SOCKS server failure
        }
    }

    /// Create error from SOCKS5 reply code
    pub fn from_reply_code(code: u8) -> Self {
        match code {
            0x01 => SocksError::GeneralFailure,
            0x02 => SocksError::Custom("Connection not allowed by ruleset".to_string()),
            0x03 => SocksError::NetworkUnreachable,
            0x04 => SocksError::HostUnreachable,
            0x05 => SocksError::ConnectionRefused,
            0x06 => SocksError::TtlExpired,
            0x07 => SocksError::CommandNotSupported("Unknown".to_string()),
            0x08 => SocksError::AddressTypeNotSupported("Unknown".to_string()),
            _ => SocksError::InvalidReplyCode(code),
        }
    }
}
