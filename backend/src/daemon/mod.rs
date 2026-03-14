use lianli_shared::ipc::{IpcRequest, IpcResponse};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no response from daemon")]
    NoResponse,
}

#[derive(Clone, Debug)]
pub struct DaemonClient {
    socket_path: PathBuf,
    timeout: Duration,
}

impl DaemonClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn send(&self, request: &IpcRequest) -> Result<IpcResponse, DaemonError> {
        let stream = UnixStream::connect(&self.socket_path)?;
        stream.set_read_timeout(Some(self.timeout)).ok();
        stream.set_write_timeout(Some(self.timeout)).ok();

        let mut writer = &stream;
        let json = serde_json::to_string(request)?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        stream.shutdown(std::net::Shutdown::Write)?;

        let reader = BufReader::new(&stream);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let response: IpcResponse = serde_json::from_str(&line)?;
            return Ok(response);
        }

        Err(DaemonError::NoResponse)
    }
}
