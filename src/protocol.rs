use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use crate::error::{Result, SocksError};

/// SOCKS5 protocol version
pub const SOCKS_VERSION: u8 = 0x05;

/// Reserved byte value
pub const RESERVED: u8 = 0x00;

/// SOCKS5 authentication methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// No authentication required
    NoAuth,
    /// GSSAPI authentication
    GssApi,
    /// Username/Password authentication
    UserPass,
    /// IANA assigned methods (0x03-0x7F)
    IanaAssigned(u8),
    /// Private methods (0x80-0xFE)
    Private(u8),
    /// No acceptable methods
    NoAcceptable,
}

impl AuthMethod {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => AuthMethod::NoAuth,
            0x01 => AuthMethod::GssApi,
            0x02 => AuthMethod::UserPass,
            0xFF => AuthMethod::NoAcceptable,
            0x03..=0x7F => AuthMethod::IanaAssigned(value),
            0x80..=0xFE => AuthMethod::Private(value),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            AuthMethod::NoAuth => 0x00,
            AuthMethod::GssApi => 0x01,
            AuthMethod::UserPass => 0x02,
            AuthMethod::NoAcceptable => 0xFF,
            AuthMethod::IanaAssigned(v) => v,
            AuthMethod::Private(v) => v,
        }
    }
}

/// SOCKS5 commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// CONNECT command
    Connect = 0x01,
    /// BIND command
    Bind = 0x02,
    /// UDP ASSOCIATE command
    UdpAssociate = 0x03,
}

impl Command {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Command::Connect),
            0x02 => Ok(Command::Bind),
            0x03 => Ok(Command::UdpAssociate),
            _ => Err(SocksError::InvalidCommand(value)),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Command::Connect => 0x01,
            Command::Bind => 0x02,
            Command::UdpAssociate => 0x03,
        }
    }
}

/// SOCKS5 address types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocksAddr {
    /// IPv4 address
    V4(Ipv4Addr),
    /// IPv6 address
    V6(Ipv6Addr),
    /// Domain name
    Domain(String),
}

impl SocksAddr {
    pub fn from_socket_addr(addr: &SocketAddr) -> Self {
        match addr.ip() {
            IpAddr::V4(ip) => SocksAddr::V4(ip),
            IpAddr::V6(ip) => SocksAddr::V6(ip),
        }
    }

    pub fn addr_type(&self) -> u8 {
        match self {
            SocksAddr::V4(_) => 0x01,
            SocksAddr::Domain(_) => 0x03,
            SocksAddr::V6(_) => 0x04,
        }
    }

    pub fn serialized_len(&self) -> usize {
        match self {
            SocksAddr::V4(_) => 4,
            SocksAddr::V6(_) => 16,
            SocksAddr::Domain(domain) => 1 + domain.len(),
        }
    }
}

impl std::fmt::Display for SocksAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocksAddr::V4(ip) => write!(f, "{}", ip),
            SocksAddr::V6(ip) => write!(f, "{}", ip),
            SocksAddr::Domain(domain) => write!(f, "{}", domain),
        }
    }
}

/// SOCKS5 reply codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyCode {
    /// Succeeded
    Succeeded = 0x00,
    /// General SOCKS server failure
    GeneralFailure = 0x01,
    /// Connection not allowed by ruleset
    NotAllowed = 0x02,
    /// Network unreachable
    NetworkUnreachable = 0x03,
    /// Host unreachable
    HostUnreachable = 0x04,
    /// Connection refused
    ConnectionRefused = 0x05,
    /// TTL expired
    TtlExpired = 0x06,
    /// Command not supported
    CommandNotSupported = 0x07,
    /// Address type not supported
    AddressTypeNotSupported = 0x08,
}

impl ReplyCode {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(ReplyCode::Succeeded),
            0x01 => Ok(ReplyCode::GeneralFailure),
            0x02 => Ok(ReplyCode::NotAllowed),
            0x03 => Ok(ReplyCode::NetworkUnreachable),
            0x04 => Ok(ReplyCode::HostUnreachable),
            0x05 => Ok(ReplyCode::ConnectionRefused),
            0x06 => Ok(ReplyCode::TtlExpired),
            0x07 => Ok(ReplyCode::CommandNotSupported),
            0x08 => Ok(ReplyCode::AddressTypeNotSupported),
            _ => Err(SocksError::InvalidReplyCode(value)),
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Username/Password authentication version
pub const USERPASS_VERSION: u8 = 0x01;

/// Username/Password authentication status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    /// Authentication succeeded
    Success = 0x00,
    /// Authentication failed
    Failure = 0xFF,
}

impl AuthStatus {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => AuthStatus::Success,
            _ => AuthStatus::Failure,
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Target address with port
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAddr {
    pub addr: SocksAddr,
    pub port: u16,
}

impl TargetAddr {
    pub fn new(addr: SocksAddr, port: u16) -> Self {
        Self { addr, port }
    }

    pub fn from_socket_addr(addr: SocketAddr) -> Self {
        Self {
            addr: SocksAddr::from_socket_addr(&addr),
            port: addr.port(),
        }
    }
}

impl std::fmt::Display for TargetAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.addr, self.port)
    }
}
