use anyhow::{anyhow, bail, Context};
use axum::http::header::HeaderName;
use serde::Deserialize;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub host: IpAddr,
    pub port: u16,
    pub socket_path: PathBuf,
    pub log_level: String,
    pub backend_config_path: PathBuf,
    pub daemon_config_path: PathBuf,
    pub profile_store_path: PathBuf,
    pub auth: AuthConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub basic_username: Option<String>,
    pub basic_password: Option<String>,
    pub bearer_token: Option<String>,
    pub proxy_header: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthMode {
    None,
    Basic,
    Bearer,
    ReverseProxy,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileAppConfig {
    host: Option<IpAddr>,
    port: Option<u16>,
    socket_path: Option<PathBuf>,
    log_level: Option<String>,
    daemon_config_path: Option<PathBuf>,
    profile_store_path: Option<PathBuf>,
    auth: Option<FileAuthConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FileAuthConfig {
    enabled: Option<bool>,
    mode: Option<String>,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
    proxy_header: Option<String>,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::defaults();
        config.backend_config_path = backend_config_path();

        let file_config = FileAppConfig::load(&config.backend_config_path)?;
        config.apply_file_config(file_config)?;
        config.apply_env_overrides()?;
        config.validate()?;

        Ok(config)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::defaults();
        config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }

    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    fn defaults() -> Self {
        let daemon_config_path = default_daemon_config_path();
        Self {
            host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 9000,
            socket_path: PathBuf::from(xdg_runtime_dir()).join("lianli-daemon.sock"),
            log_level: "info".to_string(),
            backend_config_path: default_backend_config_path(),
            daemon_config_path: daemon_config_path.clone(),
            profile_store_path: default_profile_storage_path(&daemon_config_path),
            auth: AuthConfig::disabled(),
        }
    }

    fn apply_file_config(&mut self, file_config: FileAppConfig) -> anyhow::Result<()> {
        if let Some(host) = file_config.host {
            self.host = host;
        }
        if let Some(port) = file_config.port {
            self.port = port;
        }
        if let Some(socket_path) = file_config.socket_path {
            self.socket_path = socket_path;
        }
        if let Some(log_level) = non_empty(file_config.log_level) {
            self.log_level = log_level;
        }
        if let Some(daemon_config_path) = file_config.daemon_config_path {
            self.daemon_config_path = daemon_config_path;
            if self.profile_store_path == default_profile_storage_path(&default_daemon_config_path())
            {
                self.profile_store_path = default_profile_storage_path(&self.daemon_config_path);
            }
        }
        if let Some(profile_store_path) = file_config.profile_store_path {
            self.profile_store_path = profile_store_path;
        }
        if let Some(auth) = file_config.auth {
            self.auth = AuthConfig::from_file(auth)?;
        }
        Ok(())
    }

    fn apply_env_overrides(&mut self) -> anyhow::Result<()> {
        if let Some(host) = parse_ip_addr_env("LIANLI_BACKEND_HOST")? {
            self.host = host;
        }
        if let Some(port) = parse_u16_env("LIANLI_BACKEND_PORT")? {
            self.port = port;
        }
        if let Some(socket_path) = read_non_empty_env("LIANLI_DAEMON_SOCKET").map(PathBuf::from) {
            self.socket_path = socket_path;
        }
        if let Some(log_level) = read_non_empty_env("LIANLI_BACKEND_LOG_LEVEL")
            .or_else(|| read_non_empty_env("RUST_LOG"))
        {
            self.log_level = log_level;
        }
        if let Some(daemon_config_path) =
            read_non_empty_env("LIANLI_DAEMON_CONFIG").map(PathBuf::from)
        {
            self.daemon_config_path = daemon_config_path;
        }
        if let Some(profile_store_path) =
            read_non_empty_env("LIANLI_BACKEND_PROFILE_STORE_PATH").map(PathBuf::from)
        {
            self.profile_store_path = profile_store_path;
        }
        self.auth.apply_env_overrides()?;
        Ok(())
    }

    fn validate(&mut self) -> anyhow::Result<()> {
        validate_log_level(&self.log_level)?;
        self.auth = self.auth.clone().normalized()?;
        Ok(())
    }
}

impl AuthConfig {
    fn disabled() -> Self {
        Self {
            mode: AuthMode::None,
            basic_username: None,
            basic_password: None,
            bearer_token: None,
            proxy_header: None,
        }
    }

    fn from_file(file_auth: FileAuthConfig) -> anyhow::Result<Self> {
        let username = non_empty(file_auth.username);
        let password = non_empty(file_auth.password);
        let token = non_empty(file_auth.token);
        let proxy_header = non_empty(file_auth.proxy_header);
        let explicit_mode = file_auth
            .mode
            .as_deref()
            .map(AuthMode::parse)
            .transpose()?;
        let enabled = file_auth.enabled.unwrap_or(
            explicit_mode.is_some()
                || username.is_some()
                || password.is_some()
                || token.is_some()
                || proxy_header.is_some(),
        );

        if !enabled {
            return Ok(Self::disabled());
        }

        let mode = explicit_mode.unwrap_or_else(|| {
            if token.is_some() {
                AuthMode::Bearer
            } else if proxy_header.is_some() {
                AuthMode::ReverseProxy
            } else {
                AuthMode::Basic
            }
        });

        Self {
            mode,
            basic_username: username,
            basic_password: password,
            bearer_token: token,
            proxy_header,
        }
        .normalized()
    }

    fn apply_env_overrides(&mut self) -> anyhow::Result<()> {
        let auth_mode_override = read_non_empty_env("LIANLI_BACKEND_AUTH_MODE")
            .map(|mode| AuthMode::parse(&mode))
            .transpose()?;
        if let Some(mode) = auth_mode_override {
            self.mode = mode;
            if mode == AuthMode::None {
                self.basic_username = None;
                self.basic_password = None;
                self.bearer_token = None;
                self.proxy_header = None;
            }
        }
        if let Some(username) = read_non_empty_env("LIANLI_BACKEND_AUTH_USERNAME") {
            self.basic_username = Some(username);
        }
        if let Some(password) = read_non_empty_env("LIANLI_BACKEND_AUTH_PASSWORD") {
            self.basic_password = Some(password);
        }
        if let Some(token) = read_non_empty_env("LIANLI_BACKEND_AUTH_TOKEN") {
            self.bearer_token = Some(token);
        }
        if let Some(proxy_header) = read_non_empty_env("LIANLI_BACKEND_AUTH_PROXY_HEADER") {
            self.proxy_header = Some(proxy_header);
        }

        if auth_mode_override.is_none() && self.mode == AuthMode::None {
            if self.bearer_token.is_some()
                && self.basic_username.is_none()
                && self.basic_password.is_none()
            {
                self.mode = AuthMode::Bearer;
            } else if self.proxy_header.is_some()
                && self.basic_username.is_none()
                && self.basic_password.is_none()
                && self.bearer_token.is_none()
            {
                self.mode = AuthMode::ReverseProxy;
            } else if self.basic_username.is_some() || self.basic_password.is_some() {
                self.mode = AuthMode::Basic;
            }
        }

        Ok(())
    }

    fn normalized(self) -> anyhow::Result<Self> {
        match self.mode {
            AuthMode::None => Ok(Self::disabled()),
            AuthMode::Basic => {
                let Some(username) = non_empty(self.basic_username) else {
                    bail!("basic auth requires a username");
                };
                let Some(password) = non_empty(self.basic_password) else {
                    bail!("basic auth requires a password");
                };

                Ok(Self {
                    mode: AuthMode::Basic,
                    basic_username: Some(username),
                    basic_password: Some(password),
                    bearer_token: None,
                    proxy_header: None,
                })
            }
            AuthMode::Bearer => {
                let Some(token) = non_empty(self.bearer_token) else {
                    bail!("bearer auth requires a token");
                };

                Ok(Self {
                    mode: AuthMode::Bearer,
                    basic_username: None,
                    basic_password: None,
                    bearer_token: Some(token),
                    proxy_header: None,
                })
            }
            AuthMode::ReverseProxy => {
                let header = non_empty(self.proxy_header)
                    .unwrap_or_else(|| "x-forwarded-user".to_string());
                validate_header_name(&header)?;

                Ok(Self {
                    mode: AuthMode::ReverseProxy,
                    basic_username: None,
                    basic_password: None,
                    bearer_token: None,
                    proxy_header: Some(header),
                })
            }
        }
    }
}

impl AuthMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthMode::None => "none",
            AuthMode::Basic => "basic",
            AuthMode::Bearer => "bearer",
            AuthMode::ReverseProxy => "reverse_proxy",
        }
    }

    fn parse(input: &str) -> anyhow::Result<Self> {
        match input.trim().to_lowercase().as_str() {
            "" | "none" => Ok(Self::None),
            "basic" => Ok(Self::Basic),
            "bearer" | "token" => Ok(Self::Bearer),
            "reverse_proxy" | "reverse-proxy" | "proxy" => Ok(Self::ReverseProxy),
            other => Err(anyhow!("unsupported auth mode: {other}")),
        }
    }
}

impl FileAppConfig {
    fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read backend config '{}'", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse backend config '{}'", path.display()))
    }
}

pub fn xdg_runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string())
}

pub fn xdg_config_home() -> String {
    std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        format!("{home}/.config")
    })
}

pub fn backend_config_path() -> PathBuf {
    read_non_empty_env("LIANLI_BACKEND_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(default_backend_config_path)
}

fn default_backend_config_path() -> PathBuf {
    PathBuf::from(xdg_config_home()).join("lianli").join("backend.json")
}

fn default_daemon_config_path() -> PathBuf {
    PathBuf::from(xdg_config_home()).join("lianli").join("config.json")
}

fn default_profile_storage_path(daemon_config_path: &Path) -> PathBuf {
    daemon_config_path
        .parent()
        .map(|path| path.join("profiles.json"))
        .unwrap_or_else(|| PathBuf::from("profiles.json"))
}

fn non_empty(value: impl Into<Option<String>>) -> Option<String> {
    value
        .into()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_non_empty_env(name: &str) -> Option<String> {
    non_empty(std::env::var(name).ok())
}

fn parse_ip_addr_env(name: &str) -> anyhow::Result<Option<IpAddr>> {
    read_non_empty_env(name)
        .map(|value| {
            value
                .parse::<IpAddr>()
                .with_context(|| format!("invalid IP address in {name}: {value}"))
        })
        .transpose()
}

fn parse_u16_env(name: &str) -> anyhow::Result<Option<u16>> {
    read_non_empty_env(name)
        .map(|value| {
            value
                .parse::<u16>()
                .with_context(|| format!("invalid u16 value in {name}: {value}"))
        })
        .transpose()
}

fn validate_log_level(log_level: &str) -> anyhow::Result<()> {
    tracing_subscriber::EnvFilter::try_new(log_level)
        .map(|_| ())
        .map_err(|err| anyhow!("invalid log level/filter '{log_level}': {err}"))
}

fn validate_header_name(header_name: &str) -> anyhow::Result<()> {
    HeaderName::from_bytes(header_name.as_bytes())
        .map(|_| ())
        .with_context(|| format!("invalid HTTP header name for reverse proxy auth: {header_name}"))
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, AuthMode};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    const ENV_KEYS: &[&str] = &[
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_RUNTIME_DIR",
        "RUST_LOG",
        "LIANLI_BACKEND_CONFIG",
        "LIANLI_BACKEND_HOST",
        "LIANLI_BACKEND_PORT",
        "LIANLI_DAEMON_SOCKET",
        "LIANLI_BACKEND_LOG_LEVEL",
        "LIANLI_DAEMON_CONFIG",
        "LIANLI_BACKEND_PROFILE_STORE_PATH",
        "LIANLI_BACKEND_AUTH_MODE",
        "LIANLI_BACKEND_AUTH_USERNAME",
        "LIANLI_BACKEND_AUTH_PASSWORD",
        "LIANLI_BACKEND_AUTH_TOKEN",
        "LIANLI_BACKEND_AUTH_PROXY_HEADER",
    ];

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        saved: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn apply(pairs: &[(&str, &str)]) -> Self {
            let saved = ENV_KEYS
                .iter()
                .map(|key| ((*key).to_string(), std::env::var(key).ok()))
                .collect::<Vec<_>>();

            for key in ENV_KEYS {
                std::env::remove_var(key);
            }

            for (key, value) in pairs {
                std::env::set_var(key, value);
            }

            Self { saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn from_env_uses_defaults() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _guard = EnvGuard::apply(&[
            ("HOME", "/home/tester"),
            ("XDG_CONFIG_HOME", "/home/tester/.config"),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
        ]);

        let cfg = AppConfig::from_env().expect("config from env");

        assert_eq!(cfg.host.to_string(), "0.0.0.0");
        assert_eq!(cfg.port, 9000);
        assert_eq!(cfg.socket_path, PathBuf::from("/run/user/1000/lianli-daemon.sock"));
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.backend_config_path, PathBuf::from("/home/tester/.config/lianli/backend.json"));
        assert_eq!(
            cfg.daemon_config_path,
            PathBuf::from("/home/tester/.config/lianli/config.json")
        );
        assert_eq!(
            cfg.profile_store_path,
            PathBuf::from("/home/tester/.config/lianli/profiles.json")
        );
        assert_eq!(cfg.auth.mode, AuthMode::None);
    }

    #[test]
    fn from_env_applies_overrides_and_basic_auth() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _guard = EnvGuard::apply(&[
            ("LIANLI_BACKEND_HOST", "127.0.0.1"),
            ("LIANLI_BACKEND_PORT", "9100"),
            ("LIANLI_DAEMON_SOCKET", "/tmp/custom-daemon.sock"),
            ("LIANLI_BACKEND_LOG_LEVEL", "debug,hyper=warn"),
            ("LIANLI_DAEMON_CONFIG", "/srv/lianli/config.json"),
            ("LIANLI_BACKEND_PROFILE_STORE_PATH", "/srv/lianli/profiles.json"),
            ("LIANLI_BACKEND_AUTH_MODE", "basic"),
            ("LIANLI_BACKEND_AUTH_USERNAME", "admin"),
            ("LIANLI_BACKEND_AUTH_PASSWORD", "secret"),
        ]);

        let cfg = AppConfig::from_env().expect("config from env");

        assert_eq!(cfg.host.to_string(), "127.0.0.1");
        assert_eq!(cfg.port, 9100);
        assert_eq!(cfg.socket_path, PathBuf::from("/tmp/custom-daemon.sock"));
        assert_eq!(cfg.log_level, "debug,hyper=warn");
        assert_eq!(cfg.daemon_config_path, PathBuf::from("/srv/lianli/config.json"));
        assert_eq!(cfg.profile_store_path, PathBuf::from("/srv/lianli/profiles.json"));
        assert_eq!(cfg.auth.mode, AuthMode::Basic);
        assert_eq!(cfg.auth.basic_username.as_deref(), Some("admin"));
        assert_eq!(cfg.auth.basic_password.as_deref(), Some("secret"));
    }

    #[test]
    fn load_reads_backend_config_file_and_auth() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let config_path = tempdir.path().join("backend.json");
        fs::write(
            &config_path,
            r#"{
  "host": "127.0.0.1",
  "port": 9443,
  "socket_path": "/tmp/from-file.sock",
  "log_level": "warn",
  "daemon_config_path": "/srv/lianli/config.json",
  "profile_store_path": "/srv/lianli/profiles.json",
  "auth": {
    "enabled": true,
    "mode": "basic",
    "username": "admin",
    "password": "config-secret"
  }
}"#,
        )
        .expect("write backend config");
        let _guard = EnvGuard::apply(&[
            ("LIANLI_BACKEND_CONFIG", config_path.to_str().expect("config path")),
        ]);

        let cfg = AppConfig::load().expect("load config");

        assert_eq!(cfg.backend_config_path, config_path);
        assert_eq!(cfg.host.to_string(), "127.0.0.1");
        assert_eq!(cfg.port, 9443);
        assert_eq!(cfg.socket_path, PathBuf::from("/tmp/from-file.sock"));
        assert_eq!(cfg.log_level, "warn");
        assert_eq!(cfg.daemon_config_path, PathBuf::from("/srv/lianli/config.json"));
        assert_eq!(cfg.profile_store_path, PathBuf::from("/srv/lianli/profiles.json"));
        assert_eq!(cfg.auth.mode, AuthMode::Basic);
        assert_eq!(cfg.auth.basic_username.as_deref(), Some("admin"));
        assert_eq!(cfg.auth.basic_password.as_deref(), Some("config-secret"));
    }

    #[test]
    fn load_allows_env_to_disable_file_auth() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let config_path = tempdir.path().join("backend.json");
        fs::write(
            &config_path,
            r#"{
  "auth": {
    "enabled": true,
    "mode": "basic",
    "username": "admin",
    "password": "config-secret"
  }
}"#,
        )
        .expect("write backend config");
        let _guard = EnvGuard::apply(&[
            ("LIANLI_BACKEND_CONFIG", config_path.to_str().expect("config path")),
            ("LIANLI_BACKEND_AUTH_MODE", "none"),
        ]);

        let cfg = AppConfig::load().expect("load config");

        assert_eq!(cfg.auth.mode, AuthMode::None);
        assert!(cfg.auth.basic_username.is_none());
        assert!(cfg.auth.basic_password.is_none());
    }

    #[test]
    fn from_env_rejects_invalid_basic_auth_configuration() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _guard = EnvGuard::apply(&[
            ("LIANLI_BACKEND_AUTH_MODE", "basic"),
            ("LIANLI_BACKEND_AUTH_USERNAME", "admin"),
        ]);

        let err = AppConfig::from_env().expect_err("missing password should fail");

        assert!(err.to_string().contains("basic auth requires a password"));
    }

    #[test]
    fn from_env_rejects_invalid_log_filter() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _guard = EnvGuard::apply(&[("LIANLI_BACKEND_LOG_LEVEL", "info[")]);

        let err = AppConfig::from_env().expect_err("invalid log filter should fail");

        assert!(err.to_string().contains("invalid log level/filter"));
    }

    #[test]
    fn load_rejects_invalid_reverse_proxy_header_name() {
        let _lock = env_lock()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let config_path = tempdir.path().join("backend.json");
        fs::write(
            &config_path,
            r#"{
  "auth": {
    "enabled": true,
    "mode": "reverse_proxy",
    "proxy_header": "not valid!"
  }
}"#,
        )
        .expect("write backend config");
        let _guard = EnvGuard::apply(&[
            ("LIANLI_BACKEND_CONFIG", config_path.to_str().expect("config path")),
        ]);

        let err = AppConfig::load().expect_err("invalid reverse proxy header should fail");

        assert!(err
            .to_string()
            .contains("invalid HTTP header name for reverse proxy auth"));
    }
}
