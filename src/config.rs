use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use anyhow::{Context, Result};

/// Server configuration loaded from TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server settings
    pub server: ServerConfig,
    /// Authentication settings
    #[serde(default)]
    pub auth: AuthConfig,
    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Server configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address to bind the server to
    pub bind_addr: String,
    /// Enable CONNECT command
    #[serde(default = "default_true")]
    pub enable_connect: bool,
    /// Enable BIND command
    #[serde(default = "default_true")]
    pub enable_bind: bool,
    /// Enable UDP ASSOCIATE command
    #[serde(default = "default_true")]
    pub enable_udp: bool,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

/// Authentication configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication mode: "none", "password", or "multi"
    #[serde(default = "default_auth_mode")]
    pub mode: String,
    /// Username/password pairs for password authentication
    #[serde(default)]
    pub users: HashMap<String, String>,
}

/// Logging configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: "trace", "debug", "info", "warn", "error"
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Enable JSON formatting
    #[serde(default)]
    pub json: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:1080".to_string(),
            enable_connect: true,
            enable_bind: true,
            enable_udp: true,
            timeout_secs: 300,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: "none".to_string(),
            users: HashMap::new(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read configuration file")?;
        let config: Config = toml::from_str(&content)
            .context("Failed to parse configuration file")?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        std::fs::write(path.as_ref(), content)
            .context("Failed to write configuration file")?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate bind address
        self.server.bind_addr.parse::<SocketAddr>()
            .context("Invalid bind_addr in configuration")?;

        // Validate auth mode
        match self.auth.mode.as_str() {
            "none" | "password" | "multi" => {},
            _ => anyhow::bail!("Invalid auth mode: must be 'none', 'password', or 'multi'"),
        }

        // Validate log level
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => anyhow::bail!("Invalid log level: must be 'trace', 'debug', 'info', 'warn', or 'error'"),
        }

        Ok(())
    }

    /// Generate a default configuration file
    pub fn generate_default() -> Self {
        let mut config = Config::default();

        // Add some example users for demonstration
        config.auth.users.insert("admin".to_string(), "admin123".to_string());
        config.auth.users.insert("user".to_string(), "password".to_string());

        config
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    300
}

fn default_auth_mode() -> String {
    "none".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.bind_addr, "127.0.0.1:1080");
        assert_eq!(config.auth.mode, "none");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());

        let mut bad_config = Config::default();
        bad_config.server.bind_addr = "invalid".to_string();
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_toml_serialization() {
        let config = Config::generate_default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("bind_addr"));
        assert!(toml_str.contains("[auth]"));
        assert!(toml_str.contains("[logging]"));
    }
}
