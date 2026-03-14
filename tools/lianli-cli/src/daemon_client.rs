use anyhow::{anyhow, Context, Result};
use lianli_shared::ipc::{IpcRequest, IpcResponse};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn send(&self, request: &IpcRequest) -> Result<IpcResponse> {
        let stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("connect to {}", self.socket_path.display()))?;

        stream.set_read_timeout(Some(DEFAULT_TIMEOUT)).ok();
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT)).ok();

        let mut writer = &stream;
        let json = serde_json::to_string(request)?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|e| anyhow!("shutdown error: {e}"))?;

        let reader = BufReader::new(&stream);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let response: IpcResponse = serde_json::from_str(&line)
                .with_context(|| format!("parse response: {line}"))?;
            return Ok(response);
        }

        Err(anyhow!("no response from daemon"))
    }
}
