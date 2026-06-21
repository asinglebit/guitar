use crate::helpers::localisation::{errors, modal, network};
use git2::{Config, Cred, CredentialType, Error, ErrorClass, ErrorCode};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const AUTH_REQUIRED_MESSAGE: &str = "guitar authentication required";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuthProtocol {
    Https,
    Http,
    Ssh,
    Local,
    Other,
}

impl AuthProtocol {
    pub fn label(self) -> &'static str {
        match self {
            AuthProtocol::Https => network::PROTOCOL_HTTPS(),
            AuthProtocol::Http => network::PROTOCOL_HTTP(),
            AuthProtocol::Ssh => network::PROTOCOL_SSH(),
            AuthProtocol::Local => network::PROTOCOL_LOCAL(),
            AuthProtocol::Other => network::PROTOCOL_REMOTE(),
        }
    }

    pub fn is_http(self) -> bool {
        matches!(self, AuthProtocol::Https | AuthProtocol::Http)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteAuthInfo {
    pub protocol: AuthProtocol,
    pub host: Option<String>,
    pub username: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthChallenge {
    pub url: String,
    pub username: Option<String>,
    pub protocol: AuthProtocol,
    pub operation: String,
    pub key_path: Option<PathBuf>,
}

impl AuthChallenge {
    pub fn title(&self) -> String {
        modal::auth_title(self.protocol.label())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthSecret {
    Https { username: String, password: String },
    SshKeyPassphrase { passphrase: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AuthCacheKey {
    protocol: AuthProtocol,
    url: String,
    username: Option<String>,
    key_path: Option<PathBuf>,
}

impl AuthCacheKey {
    fn https(url: &str, username: &str, protocol: AuthProtocol) -> Self {
        Self { protocol, url: url.to_string(), username: Some(username.to_string()), key_path: None }
    }

    fn ssh(url: &str, username: &str, key_path: &Path) -> Self {
        Self { protocol: AuthProtocol::Ssh, url: url.to_string(), username: Some(username.to_string()), key_path: Some(key_path.to_path_buf()) }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AuthSession {
    secrets: HashMap<AuthCacheKey, AuthSecret>,
}

impl AuthSession {
    pub fn store(&mut self, challenge: &AuthChallenge, secret: AuthSecret) {
        match (&challenge.protocol, &secret) {
            (protocol, AuthSecret::Https { username, .. }) if protocol.is_http() => {
                self.secrets.insert(AuthCacheKey::https(&challenge.url, username, challenge.protocol), secret);
            },
            (AuthProtocol::Ssh, AuthSecret::SshKeyPassphrase { .. }) => {
                if let (Some(username), Some(key_path)) = (challenge.username.as_deref(), challenge.key_path.as_deref()) {
                    self.secrets.insert(AuthCacheKey::ssh(&challenge.url, username, key_path), secret);
                }
            },
            _ => {},
        }
    }

    pub fn evict(&mut self, keys: &[AuthCacheKey]) {
        for key in keys {
            self.secrets.remove(key);
        }
    }

    pub fn has_secret_for(&self, challenge: &AuthChallenge, username: Option<&str>) -> bool {
        match challenge.protocol {
            AuthProtocol::Https | AuthProtocol::Http => self.https_secret(&challenge.url, username.or(challenge.username.as_deref()), challenge.protocol).is_some(),
            AuthProtocol::Ssh => match (challenge.username.as_deref(), challenge.key_path.as_deref()) {
                (Some(username), Some(key_path)) => self.ssh_secret(&challenge.url, username, key_path).is_some(),
                _ => false,
            },
            AuthProtocol::Local | AuthProtocol::Other => false,
        }
    }

    fn https_secret(&self, url: &str, username_hint: Option<&str>, protocol: AuthProtocol) -> Option<(AuthCacheKey, String, String)> {
        if let Some(username) = username_hint {
            let key = AuthCacheKey::https(url, username, protocol);
            if let Some(AuthSecret::Https { username, password }) = self.secrets.get(&key) {
                return Some((key, username.clone(), password.clone()));
            }
        }

        self.secrets.iter().find_map(|(key, secret)| match secret {
            AuthSecret::Https { username, password } if key.url == url && key.protocol == protocol => Some((key.clone(), username.clone(), password.clone())),
            _ => None,
        })
    }

    fn ssh_secret(&self, url: &str, username: &str, key_path: &Path) -> Option<(AuthCacheKey, String)> {
        let key = AuthCacheKey::ssh(url, username, key_path);
        match self.secrets.get(&key) {
            Some(AuthSecret::SshKeyPassphrase { passphrase }) => Some((key, passphrase.clone())),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthRequired {
    pub challenge: AuthChallenge,
    pub rejected: Vec<AuthCacheKey>,
}

#[derive(Debug)]
pub enum NetworkResult {
    Success,
    AuthRequired(AuthRequired),
    Failure(String),
}

#[derive(Clone)]
pub struct AuthAttempt {
    session: AuthSession,
    operation: String,
    state: Arc<Mutex<AuthAttemptState>>,
}

#[derive(Default)]
struct AuthAttemptState {
    challenge: Option<AuthChallenge>,
    promptable: Option<AuthChallenge>,
    used_session_keys: Vec<AuthCacheKey>,
}

impl AuthAttempt {
    pub fn new(session: AuthSession, operation: impl Into<String>) -> Self {
        Self { session, operation: operation.into(), state: Arc::new(Mutex::new(AuthAttemptState::default())) }
    }

    pub fn credentials(&self, config: &Config, url: &str, username_from_url: Option<&str>, allowed: CredentialType) -> Result<Cred, Error> {
        let info = classify_remote_url(url);
        match info.protocol {
            AuthProtocol::Https | AuthProtocol::Http => self.http_credentials(config, url, username_from_url.or(info.username.as_deref()), info.protocol, allowed),
            AuthProtocol::Ssh => self.ssh_credentials(url, username_from_url.or(info.username.as_deref()), allowed),
            AuthProtocol::Local | AuthProtocol::Other => {
                if allowed.is_default() {
                    Cred::default()
                } else {
                    Err(Error::new(ErrorCode::Auth, ErrorClass::Callback, "unsupported credential request for this remote"))
                }
            },
        }
    }

    fn http_credentials(&self, config: &Config, url: &str, username_hint: Option<&str>, protocol: AuthProtocol, allowed: CredentialType) -> Result<Cred, Error> {
        let challenge = AuthChallenge { url: url.to_string(), username: username_hint.map(ToString::to_string), protocol, operation: self.operation.clone(), key_path: None };
        self.set_promptable(challenge.clone());

        if let Some((key, username, password)) = self.session.https_secret(url, username_hint, protocol) {
            self.record_used(key);
            return Cred::userpass_plaintext(&username, &password);
        }

        if allowed.is_user_pass_plaintext()
            && let Ok(cred) = Cred::credential_helper(config, url, username_hint)
        {
            return Ok(cred);
        }

        if allowed.is_username()
            && let Some(username) = username_hint
        {
            return Cred::username(username);
        }

        self.set_challenge(challenge);
        Err(auth_required_error())
    }

    fn ssh_credentials(&self, url: &str, username_hint: Option<&str>, allowed: CredentialType) -> Result<Cred, Error> {
        let username = username_hint.unwrap_or("git");
        let key_path = default_ssh_private_key();
        let challenge = AuthChallenge { url: url.to_string(), username: Some(username.to_string()), protocol: AuthProtocol::Ssh, operation: self.operation.clone(), key_path: key_path.clone() };
        if key_path.is_some() {
            self.set_promptable(challenge.clone());
        }

        if allowed.is_ssh_key() {
            if let Some(path) = key_path.as_deref()
                && let Some((key, passphrase)) = self.session.ssh_secret(url, username, path)
            {
                self.record_used(key);
                return Cred::ssh_key(username, None, path, Some(&passphrase));
            }

            if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                return Ok(cred);
            }

            if let Some(path) = key_path.as_deref()
                && let Ok(cred) = Cred::ssh_key(username, None, path, None)
            {
                return Ok(cred);
            }
        }

        if allowed.is_username() {
            return Cred::username(username);
        }

        if challenge.key_path.is_some() {
            self.set_challenge(challenge);
            return Err(auth_required_error());
        }

        Err(Error::new(ErrorCode::Auth, ErrorClass::Ssh, "SSH authentication requires an ssh-agent or a default private key at ~/.ssh/id_ed25519, ~/.ssh/id_ecdsa, or ~/.ssh/id_rsa"))
    }

    fn set_promptable(&self, challenge: AuthChallenge) {
        let mut state = self.state.lock().unwrap();
        state.promptable = Some(challenge);
    }

    fn set_challenge(&self, challenge: AuthChallenge) {
        let mut state = self.state.lock().unwrap();
        state.challenge = Some(challenge);
    }

    fn record_used(&self, key: AuthCacheKey) {
        let mut state = self.state.lock().unwrap();
        if !state.used_session_keys.contains(&key) {
            state.used_session_keys.push(key);
        }
    }

    fn auth_required(&self, error: &Error) -> Option<AuthRequired> {
        let state = self.state.lock().unwrap();
        let challenge = state.challenge.clone().or_else(|| (error.code() == ErrorCode::Auth).then(|| state.promptable.clone()).flatten())?;
        Some(AuthRequired { challenge, rejected: state.used_session_keys.clone() })
    }
}

pub fn network_result(label: &str, attempt: &AuthAttempt, result: Result<(), Error>) -> NetworkResult {
    match result {
        Ok(_) => NetworkResult::Success,
        Err(error) => match attempt.auth_required(&error) {
            Some(auth) => NetworkResult::AuthRequired(auth),
            None => NetworkResult::Failure(errors::operation_failed(label, error)),
        },
    }
}

pub fn auth_required_error() -> Error {
    Error::new(ErrorCode::Auth, ErrorClass::Callback, AUTH_REQUIRED_MESSAGE)
}

pub fn classify_remote_url(url: &str) -> RemoteAuthInfo {
    if let Some(rest) = url.strip_prefix("https://") {
        return RemoteAuthInfo { protocol: AuthProtocol::Https, host: host_from_url_rest(rest), username: username_from_url_rest(rest) };
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return RemoteAuthInfo { protocol: AuthProtocol::Http, host: host_from_url_rest(rest), username: username_from_url_rest(rest) };
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        return RemoteAuthInfo { protocol: AuthProtocol::Ssh, host: host_from_url_rest(rest), username: username_from_url_rest(rest) };
    }
    if url.starts_with("file://") || looks_like_local_path(url) {
        return RemoteAuthInfo { protocol: AuthProtocol::Local, host: None, username: None };
    }
    if is_scp_like_ssh(url) {
        let (user, host) = scp_user_host(url);
        return RemoteAuthInfo { protocol: AuthProtocol::Ssh, host, username: user };
    }
    if url.contains("://") {
        return RemoteAuthInfo { protocol: AuthProtocol::Other, host: None, username: None };
    }

    RemoteAuthInfo { protocol: AuthProtocol::Local, host: None, username: None }
}

pub fn default_ssh_private_key() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    default_ssh_private_key_in(&home)
}

fn default_ssh_private_key_in(home: &Path) -> Option<PathBuf> {
    ["id_ed25519", "id_ecdsa", "id_rsa"].into_iter().map(|name| home.join(".ssh").join(name)).find(|path| path.is_file())
}

fn host_from_url_rest(rest: &str) -> Option<String> {
    let authority = rest.split('/').next().unwrap_or(rest);
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = authority.split(':').next().unwrap_or(authority);
    (!host.is_empty()).then(|| host.to_string())
}

fn username_from_url_rest(rest: &str) -> Option<String> {
    let authority = rest.split('/').next().unwrap_or(rest);
    authority.split_once('@').and_then(|(username, _)| (!username.is_empty()).then(|| username.to_string()))
}

fn looks_like_local_path(url: &str) -> bool {
    url.starts_with('/') || url.starts_with("./") || url.starts_with("../") || (url.len() > 2 && url.as_bytes()[1] == b':' && url.as_bytes()[0].is_ascii_alphabetic())
}

fn is_scp_like_ssh(url: &str) -> bool {
    let Some(colon) = url.find(':') else {
        return false;
    };
    colon > 1 && !url[..colon].contains('/') && url[..colon].contains('@')
}

fn scp_user_host(url: &str) -> (Option<String>, Option<String>) {
    let before_colon = url.split(':').next().unwrap_or_default();
    match before_colon.split_once('@') {
        Some((user, host)) => ((!user.is_empty()).then(|| user.to_string()), (!host.is_empty()).then(|| host.to_string())),
        None => (None, (!before_colon.is_empty()).then(|| before_colon.to_string())),
    }
}

#[cfg(test)]
#[path = "../tests/git/auth.rs"]
mod tests;
