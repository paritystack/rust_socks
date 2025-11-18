use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use crate::error::{Result, SocksError};
use crate::protocol::AuthMethod;

/// Trait for authentication handlers
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Authenticate with username and password
    async fn authenticate(&self, username: &str, password: &str) -> Result<bool>;

    /// Get supported authentication methods
    fn supported_methods(&self) -> Vec<AuthMethod>;
}

/// No authentication - accepts all connections
#[derive(Debug, Clone, Default)]
pub struct NoAuth;

#[async_trait]
impl Authenticator for NoAuth {
    async fn authenticate(&self, _username: &str, _password: &str) -> Result<bool> {
        Ok(true)
    }

    fn supported_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::NoAuth]
    }
}

/// Username/Password authentication with a static credential store
#[derive(Debug, Clone)]
pub struct UserPassAuth {
    credentials: Arc<HashMap<String, String>>,
}

impl UserPassAuth {
    /// Create a new username/password authenticator with credentials
    pub fn new(credentials: HashMap<String, String>) -> Self {
        Self {
            credentials: Arc::new(credentials),
        }
    }

    /// Create a new authenticator with a single user
    pub fn single_user(username: String, password: String) -> Self {
        let mut credentials = HashMap::new();
        credentials.insert(username, password);
        Self::new(credentials)
    }

    /// Create a new authenticator with multiple users
    pub fn with_users(users: Vec<(String, String)>) -> Self {
        let credentials = users.into_iter().collect();
        Self::new(credentials)
    }
}

#[async_trait]
impl Authenticator for UserPassAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        match self.credentials.get(username) {
            Some(expected_password) => Ok(expected_password == password),
            None => Ok(false),
        }
    }

    fn supported_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::UserPass]
    }
}

/// Combined authenticator supporting multiple methods
#[derive(Clone)]
pub struct MultiAuth {
    authenticators: Vec<(AuthMethod, Arc<dyn Authenticator>)>,
}

impl MultiAuth {
    /// Create a new multi-method authenticator
    pub fn new() -> Self {
        Self {
            authenticators: Vec::new(),
        }
    }

    /// Add an authentication method
    pub fn add_method(mut self, method: AuthMethod, auth: Arc<dyn Authenticator>) -> Self {
        self.authenticators.push((method, auth));
        self
    }

    /// Add no-auth support
    pub fn with_no_auth(self) -> Self {
        self.add_method(AuthMethod::NoAuth, Arc::new(NoAuth))
    }

    /// Add username/password authentication
    pub fn with_user_pass(self, auth: UserPassAuth) -> Self {
        self.add_method(AuthMethod::UserPass, Arc::new(auth))
    }

    /// Get authenticator for a specific method
    pub fn get_authenticator(&self, method: AuthMethod) -> Option<Arc<dyn Authenticator>> {
        self.authenticators
            .iter()
            .find(|(m, _)| *m == method)
            .map(|(_, auth)| Arc::clone(auth))
    }
}

impl Default for MultiAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Authenticator for MultiAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        // Try username/password auth if available
        if let Some(auth) = self.get_authenticator(AuthMethod::UserPass) {
            return auth.authenticate(username, password).await;
        }
        Ok(false)
    }

    fn supported_methods(&self) -> Vec<AuthMethod> {
        self.authenticators.iter().map(|(m, _)| *m).collect()
    }
}

/// Client-side authentication credentials
#[derive(Debug, Clone)]
pub enum ClientAuth {
    /// No authentication
    None,
    /// Username and password
    Password { username: String, password: String },
}

impl ClientAuth {
    /// Get preferred authentication methods
    pub fn preferred_methods(&self) -> Vec<AuthMethod> {
        match self {
            ClientAuth::None => vec![AuthMethod::NoAuth],
            ClientAuth::Password { .. } => vec![AuthMethod::UserPass, AuthMethod::NoAuth],
        }
    }

    /// Get username and password if available
    pub fn credentials(&self) -> Option<(&str, &str)> {
        match self {
            ClientAuth::None => None,
            ClientAuth::Password { username, password } => Some((username, password)),
        }
    }
}

impl Default for ClientAuth {
    fn default() -> Self {
        ClientAuth::None
    }
}

/// Validate username and password format
pub fn validate_credentials(username: &str, password: &str) -> Result<()> {
    if username.is_empty() {
        return Err(SocksError::AuthenticationFailed("Username cannot be empty".to_string()));
    }
    if password.is_empty() {
        return Err(SocksError::AuthenticationFailed("Password cannot be empty".to_string()));
    }
    if username.len() > 255 {
        return Err(SocksError::AuthenticationFailed("Username too long".to_string()));
    }
    if password.len() > 255 {
        return Err(SocksError::AuthenticationFailed("Password too long".to_string()));
    }
    Ok(())
}
