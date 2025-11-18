use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::error::{Result, SocksError};
use crate::protocol::*;

/// Read authentication method selection message from client
/// Format: [VER(1)] [NMETHODS(1)] [METHODS(1-255)]
pub async fn read_auth_request<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Vec<AuthMethod>> {
    let version = reader.read_u8().await?;
    if version != SOCKS_VERSION {
        return Err(SocksError::InvalidVersion(version));
    }

    let nmethods = reader.read_u8().await?;
    if nmethods == 0 {
        return Err(SocksError::InvalidData("No auth methods provided".to_string()));
    }

    let mut methods = vec![0u8; nmethods as usize];
    reader.read_exact(&mut methods).await?;

    Ok(methods.into_iter().map(AuthMethod::from_u8).collect())
}

/// Write authentication method selection response to client
/// Format: [VER(1)] [METHOD(1)]
pub async fn write_auth_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    method: AuthMethod,
) -> Result<()> {
    writer.write_u8(SOCKS_VERSION).await?;
    writer.write_u8(method.to_u8()).await?;
    writer.flush().await?;
    Ok(())
}

/// Read username/password authentication request
/// Format: [VER(1)] [ULEN(1)] [UNAME(1-255)] [PLEN(1)] [PASSWD(1-255)]
pub async fn read_userpass_request<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<(String, String)> {
    let version = reader.read_u8().await?;
    if version != USERPASS_VERSION {
        return Err(SocksError::InvalidVersion(version));
    }

    let ulen = reader.read_u8().await?;
    let mut username = vec![0u8; ulen as usize];
    reader.read_exact(&mut username).await?;
    let username = String::from_utf8(username)?;

    let plen = reader.read_u8().await?;
    let mut password = vec![0u8; plen as usize];
    reader.read_exact(&mut password).await?;
    let password = String::from_utf8(password)?;

    Ok((username, password))
}

/// Write username/password authentication response
/// Format: [VER(1)] [STATUS(1)]
pub async fn write_userpass_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    status: AuthStatus,
) -> Result<()> {
    writer.write_u8(USERPASS_VERSION).await?;
    writer.write_u8(status.to_u8()).await?;
    writer.flush().await?;
    Ok(())
}

/// Read SOCKS5 address from stream
/// Format: [ATYP(1)] [DST.ADDR(variable)] [DST.PORT(2)]
pub async fn read_address<R: AsyncRead + Unpin>(reader: &mut R) -> Result<TargetAddr> {
    let atyp = reader.read_u8().await?;

    let addr = match atyp {
        0x01 => {
            // IPv4
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf).await?;
            SocksAddr::V4(Ipv4Addr::from(buf))
        }
        0x03 => {
            // Domain name
            let len = reader.read_u8().await?;
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf).await?;
            let domain = String::from_utf8(buf)?;
            SocksAddr::Domain(domain)
        }
        0x04 => {
            // IPv6
            let mut buf = [0u8; 16];
            reader.read_exact(&mut buf).await?;
            SocksAddr::V6(Ipv6Addr::from(buf))
        }
        _ => return Err(SocksError::InvalidAddressType(atyp)),
    };

    let port = reader.read_u16().await?;

    Ok(TargetAddr::new(addr, port))
}

/// Write SOCKS5 address to stream
/// Format: [ATYP(1)] [ADDR(variable)] [PORT(2)]
pub async fn write_address<W: AsyncWrite + Unpin>(
    writer: &mut W,
    target: &TargetAddr,
) -> Result<()> {
    writer.write_u8(target.addr.addr_type()).await?;

    match &target.addr {
        SocksAddr::V4(ip) => {
            writer.write_all(&ip.octets()).await?;
        }
        SocksAddr::V6(ip) => {
            writer.write_all(&ip.octets()).await?;
        }
        SocksAddr::Domain(domain) => {
            writer.write_u8(domain.len() as u8).await?;
            writer.write_all(domain.as_bytes()).await?;
        }
    }

    writer.write_u16(target.port).await?;
    Ok(())
}

/// Read SOCKS5 request from client
/// Format: [VER(1)] [CMD(1)] [RSV(1)] [ATYP(1)] [DST.ADDR(variable)] [DST.PORT(2)]
pub async fn read_request<R: AsyncRead + Unpin>(reader: &mut R) -> Result<(Command, TargetAddr)> {
    let version = reader.read_u8().await?;
    if version != SOCKS_VERSION {
        return Err(SocksError::InvalidVersion(version));
    }

    let cmd = reader.read_u8().await?;
    let cmd = Command::from_u8(cmd)?;

    let _rsv = reader.read_u8().await?; // Reserved byte

    let target = read_address(reader).await?;

    Ok((cmd, target))
}

/// Write SOCKS5 reply to client
/// Format: [VER(1)] [REP(1)] [RSV(1)] [ATYP(1)] [BND.ADDR(variable)] [BND.PORT(2)]
pub async fn write_reply<W: AsyncWrite + Unpin>(
    writer: &mut W,
    reply: ReplyCode,
    bind_addr: &TargetAddr,
) -> Result<()> {
    writer.write_u8(SOCKS_VERSION).await?;
    writer.write_u8(reply.to_u8()).await?;
    writer.write_u8(RESERVED).await?;

    write_address(writer, bind_addr).await?;

    writer.flush().await?;
    Ok(())
}

/// Read SOCKS5 reply from server (client-side)
pub async fn read_reply<R: AsyncRead + Unpin>(reader: &mut R) -> Result<(ReplyCode, TargetAddr)> {
    let version = reader.read_u8().await?;
    if version != SOCKS_VERSION {
        return Err(SocksError::InvalidVersion(version));
    }

    let rep = reader.read_u8().await?;
    let reply = ReplyCode::from_u8(rep)?;

    let _rsv = reader.read_u8().await?; // Reserved byte

    let bind_addr = read_address(reader).await?;

    Ok((reply, bind_addr))
}

/// Write SOCKS5 request to server (client-side)
pub async fn write_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    cmd: Command,
    target: &TargetAddr,
) -> Result<()> {
    writer.write_u8(SOCKS_VERSION).await?;
    writer.write_u8(cmd.to_u8()).await?;
    writer.write_u8(RESERVED).await?;

    write_address(writer, target).await?;

    writer.flush().await?;
    Ok(())
}

/// Write username/password authentication request (client-side)
pub async fn write_userpass_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    username: &str,
    password: &str,
) -> Result<()> {
    if username.len() > 255 || password.len() > 255 {
        return Err(SocksError::InvalidData("Username or password too long".to_string()));
    }

    writer.write_u8(USERPASS_VERSION).await?;
    writer.write_u8(username.len() as u8).await?;
    writer.write_all(username.as_bytes()).await?;
    writer.write_u8(password.len() as u8).await?;
    writer.write_all(password.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

/// Read username/password authentication response (client-side)
pub async fn read_userpass_response<R: AsyncRead + Unpin>(reader: &mut R) -> Result<AuthStatus> {
    let version = reader.read_u8().await?;
    if version != USERPASS_VERSION {
        return Err(SocksError::InvalidVersion(version));
    }

    let status = reader.read_u8().await?;
    Ok(AuthStatus::from_u8(status))
}

/// Write authentication method selection request (client-side)
pub async fn write_auth_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    methods: &[AuthMethod],
) -> Result<()> {
    if methods.is_empty() || methods.len() > 255 {
        return Err(SocksError::InvalidData("Invalid number of auth methods".to_string()));
    }

    writer.write_u8(SOCKS_VERSION).await?;
    writer.write_u8(methods.len() as u8).await?;
    for method in methods {
        writer.write_u8(method.to_u8()).await?;
    }
    writer.flush().await?;
    Ok(())
}

/// Read authentication method selection response (client-side)
pub async fn read_auth_response<R: AsyncRead + Unpin>(reader: &mut R) -> Result<AuthMethod> {
    let version = reader.read_u8().await?;
    if version != SOCKS_VERSION {
        return Err(SocksError::InvalidVersion(version));
    }

    let method = reader.read_u8().await?;
    Ok(AuthMethod::from_u8(method))
}
