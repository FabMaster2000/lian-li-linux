mod fan_controller;
mod ipc_server;
mod openrgb_server;
mod rgb_controller;
mod service;

use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

fn default_config_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
            PathBuf::from(home).join(".config")
        });
    config_dir.join("lianli").join("config.json")
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Linux daemon for Lian Li fan control and LCD streaming"
)]
struct Cli {
    /// Path to the configuration file
    #[arg(long, default_value_os_t = default_config_path())]
    config: PathBuf,

    /// Logging verbosity (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .init();

    let mut manager = service::ServiceManager::new(cli.config)?;
    let restart = manager.run()?;

    if restart {
        use std::os::unix::process::CommandExt;
        let exe = std::env::current_exe()?;
        let args: Vec<String> = std::env::args().skip(1).collect();
        tracing::info!("Re-executing daemon: {} {}", exe.display(), args.join(" "));
        let err = std::process::Command::new(exe).args(args).exec();
        // exec() only returns on error
        anyhow::bail!("Failed to re-exec daemon: {err}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::default_config_path;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn default_config_path_prefers_xdg_config_home() {
        let _guard = env_lock().lock().expect("lock env");
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_home = std::env::var("HOME").ok();

        std::env::set_var("XDG_CONFIG_HOME", "/tmp/lianli-tests/xdg");
        std::env::set_var("HOME", "/tmp/lianli-tests/home");

        assert_eq!(
            default_config_path(),
            PathBuf::from("/tmp/lianli-tests/xdg/lianli/config.json")
        );

        restore_env("XDG_CONFIG_HOME", old_xdg);
        restore_env("HOME", old_home);
    }

    #[test]
    fn default_config_path_falls_back_to_home_config() {
        let _guard = env_lock().lock().expect("lock env");
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_home = std::env::var("HOME").ok();

        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::set_var("HOME", "/tmp/lianli-tests/home");

        assert_eq!(
            default_config_path(),
            PathBuf::from("/tmp/lianli-tests/home/.config/lianli/config.json")
        );

        restore_env("XDG_CONFIG_HOME", old_xdg);
        restore_env("HOME", old_home);
    }

    fn restore_env(key: &str, value: Option<String>) {
        if let Some(value) = value {
            std::env::set_var(key, value);
        } else {
            std::env::remove_var(key);
        }
    }
}
