// Minimal niri IPC client for niri-settings (output queries only).

pub mod types;

use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use types::{NiriReply, NiriRequest};

use crate::error::IpcError;

/// Read/write deadline for one-shot IPC calls. Without this a hung or
/// unresponsive niri socket would block the worker thread (and leave the UI
/// spinner up) forever; a timeout surfaces as a normal `Recv` error instead.
const IPC_TIMEOUT: Duration = Duration::from_secs(3);

/// Query the list of connected outputs from niri in a blocking one-shot call.
/// Should be called off the GTK main thread.
pub fn query_outputs() -> Result<Vec<types::NiriOutput>, IpcError> {
    let socket_path = env::var("NIRI_SOCKET").map_err(|_| IpcError::SocketEnvMissing)?;
    let stream = UnixStream::connect(&socket_path).map_err(IpcError::Connect)?;
    stream
        .set_read_timeout(Some(IPC_TIMEOUT))
        .map_err(IpcError::Connect)?;
    stream
        .set_write_timeout(Some(IPC_TIMEOUT))
        .map_err(IpcError::Connect)?;
    let mut stream = stream;
    let req =
        serde_json::to_string(&NiriRequest::Outputs).map_err(|e| IpcError::Send(e.to_string()))?;
    writeln!(stream, "{req}").map_err(|e| IpcError::Send(e.to_string()))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| IpcError::Recv(e.to_string()))?;
    let reply: NiriReply =
        serde_json::from_str(line.trim()).map_err(|e| IpcError::Parse(e.to_string()))?;
    match reply {
        NiriReply::Ok(val) => {
            let map_val = val
                .get("Outputs")
                .cloned()
                .ok_or_else(|| IpcError::Parse("response missing Outputs key".into()))?;
            let map: HashMap<String, types::NiriOutput> =
                serde_json::from_value(map_val).map_err(|e| IpcError::Parse(e.to_string()))?;
            let mut outputs: Vec<types::NiriOutput> = map.into_values().collect();
            outputs.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(outputs)
        }
        NiriReply::Err(msg) => Err(IpcError::Recv(msg)),
    }
}
