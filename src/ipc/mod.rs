// Minimal niri IPC client for niri-settings (output queries only).

pub mod types;

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use types::{NiriReply, NiriRequest, NiriResponse};

use crate::error::IpcError;

/// Read/write deadline for one-shot IPC calls. Without this a hung or
/// unresponsive niri socket would block the worker thread (and leave the UI
/// spinner up) forever; a timeout surfaces as a normal `Recv` error instead.
const IPC_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_IPC_LINE_BYTES: u64 = 4 * 1024 * 1024;

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
    read_bounded_line(&mut reader, &mut line).map_err(|e| IpcError::Recv(e.to_string()))?;
    let reply: NiriReply =
        serde_json::from_str(line.trim()).map_err(|e| IpcError::Parse(e.to_string()))?;
    match reply {
        NiriReply::Ok(NiriResponse::Outputs(map)) => {
            let mut outputs: Vec<types::NiriOutput> = map.into_values().collect();
            outputs.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(outputs)
        }
        NiriReply::Err(msg) => Err(IpcError::Recv(msg)),
    }
}

fn read_bounded_line(
    reader: &mut impl BufRead,
    line: &mut String,
) -> Result<usize, std::io::Error> {
    let read = std::io::Read::take(reader, MAX_IPC_LINE_BYTES + 1).read_line(line)?;
    if read as u64 > MAX_IPC_LINE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "niri IPC frame exceeded size limit",
        ));
    }
    Ok(read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn typed_outputs_reply_deserializes() {
        let reply: NiriReply = serde_json::from_str(r#"{"Ok":{"Outputs":{}}}"#).unwrap();
        assert!(matches!(
            reply,
            NiriReply::Ok(NiriResponse::Outputs(outputs)) if outputs.is_empty()
        ));
    }

    #[test]
    fn oversized_ipc_frame_is_rejected() {
        let mut data = vec![b'x'; MAX_IPC_LINE_BYTES as usize + 1];
        data.push(b'\n');
        let mut line = String::new();
        assert!(read_bounded_line(&mut Cursor::new(data), &mut line).is_err());
    }
}
