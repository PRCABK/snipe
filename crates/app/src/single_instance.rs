use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub const MUTEX_NAME: &str = "Local\\Snipe_SingleInstance_Mutex";
pub const PIPE_NAME: &str = "\\\\.\\pipe\\Snipe_IPC_Pipe";
pub const IPC_PROTOCOL_VERSION: u32 = 1;
const MAX_IPC_MESSAGE_BYTES: usize = 1024;
const IPC_RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcCommand {
    OpenSettings,
    Ping,
    ShutdownForUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcResponse {
    Ready,
    Busy,
    Cancelled,
    Incompatible,
    Rejected,
}

impl IpcResponse {
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Busy => "busy",
            Self::Cancelled => "cancelled",
            Self::Incompatible => "incompatible",
            Self::Rejected => "rejected",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "ready" => Some(Self::Ready),
            "busy" => Some(Self::Busy),
            "cancelled" => Some(Self::Cancelled),
            "incompatible" => Some(Self::Incompatible),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

pub struct IpcRequest {
    pub command: IpcCommand,
    pub response: Sender<IpcResponse>,
}

pub struct SingleInstanceGuard {
    #[cfg(windows)]
    _handle: windows::Win32::Foundation::HANDLE,
}

pub fn try_acquire_instance() -> Option<SingleInstanceGuard> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows::Win32::System::Threading::CreateMutexW;

        let name_wide: Vec<u16> = MUTEX_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let handle = CreateMutexW(None, true, PCWSTR(name_wide.as_ptr())).ok()?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return None;
            }
            Some(SingleInstanceGuard { _handle: handle })
        }
    }

    #[cfg(not(windows))]
    Some(SingleInstanceGuard {})
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self._handle);
        }
    }
}

pub fn request_from_primary(command: IpcCommand) -> Option<IpcResponse> {
    request_from_primary_with_timeout(command, IPC_RESPONSE_TIMEOUT)
}

pub fn request_from_primary_with_timeout(
    command: IpcCommand,
    timeout: std::time::Duration,
) -> Option<IpcResponse> {
    #[cfg(windows)]
    {
        let (response_tx, response_rx) = channel();
        std::thread::spawn(move || {
            let _ = response_tx.send(request_from_primary_blocking(command));
        });
        return response_rx.recv_timeout(timeout).ok().flatten();
    }

    #[cfg(not(windows))]
    {
        let _ = command;
        let _ = timeout;
        None
    }
}

#[cfg(windows)]
fn request_from_primary_blocking(command: IpcCommand) -> Option<IpcResponse> {
    use std::fs::OpenOptions;

    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(PIPE_NAME)
        .ok()?;
    let wire_command = match command {
        IpcCommand::OpenSettings => "OPEN_SETTINGS",
        IpcCommand::Ping => "PING",
        IpcCommand::ShutdownForUpdate => "SHUTDOWN_FOR_UPDATE",
    };
    let request = format!("SNIPE/{} {}\n", IPC_PROTOCOL_VERSION, wire_command);
    file.write_all(request.as_bytes()).ok()?;
    file.flush().ok()?;

    let mut response = [0u8; 64];
    let count = file.read(&mut response).ok()?;
    let response = std::str::from_utf8(&response[..count]).ok()?;
    IpcResponse::parse(response)
}

pub fn start_ipc_server(request_tx: UnboundedSender<IpcRequest>, shutdown_flag: Arc<AtomicBool>) {
    #[cfg(windows)]
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::windows::named_pipe::ServerOptions;

        info!("Starting IPC named pipe server on {}", PIPE_NAME);
        let mut is_first = true;
        while !shutdown_flag.load(Ordering::Relaxed) {
            let server = match ServerOptions::new()
                .first_pipe_instance(is_first)
                .create(PIPE_NAME)
            {
                Ok(server) => {
                    is_first = false;
                    server
                }
                Err(error) => {
                    warn!("Failed to create named pipe instance: {error}");
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            };

            if server.connect().await.is_err() {
                continue;
            }

            let (reader, mut writer) = tokio::io::split(server);
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let response = match reader.read_line(&mut line).await {
                Ok(count) if count > 0 && count <= MAX_IPC_MESSAGE_BYTES => {
                    match parse_command(line.trim()) {
                        Ok(command) => {
                            let (response_tx, response_rx) = channel();
                            if request_tx
                                .send(IpcRequest {
                                    command,
                                    response: response_tx,
                                })
                                .is_err()
                            {
                                IpcResponse::Busy
                            } else {
                                response_rx
                                    .recv_timeout(IPC_RESPONSE_TIMEOUT)
                                    .unwrap_or(IpcResponse::Busy)
                            }
                        }
                        Err(response) => response,
                    }
                }
                Ok(_) => IpcResponse::Rejected,
                Err(error) => {
                    warn!("Failed to read IPC request: {error}");
                    IpcResponse::Rejected
                }
            };
            let _ = writer
                .write_all(format!("{}\n", response.as_wire()).as_bytes())
                .await;
        }
    });

    #[cfg(not(windows))]
    {
        let _ = request_tx;
        let _ = shutdown_flag;
    }
}

fn parse_command(line: &str) -> Result<IpcCommand, IpcResponse> {
    let (version, command) = line.split_once(' ').ok_or(IpcResponse::Rejected)?;
    let version = version
        .strip_prefix("SNIPE/")
        .ok_or(IpcResponse::Rejected)?
        .parse::<u32>()
        .map_err(|_| IpcResponse::Rejected)?;
    if version != IPC_PROTOCOL_VERSION {
        return Err(IpcResponse::Incompatible);
    }
    match command {
        "OPEN_SETTINGS" => Ok(IpcCommand::OpenSettings),
        "PING" => Ok(IpcCommand::Ping),
        "SHUTDOWN_FOR_UPDATE" => Ok(IpcCommand::ShutdownForUpdate),
        _ => Err(IpcResponse::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_versioned_whitelist_commands() {
        assert_eq!(parse_command("SNIPE/1 PING"), Ok(IpcCommand::Ping));
        assert_eq!(
            parse_command("SNIPE/1 SHUTDOWN_FOR_UPDATE"),
            Ok(IpcCommand::ShutdownForUpdate)
        );
        assert_eq!(
            parse_command("SNIPE/2 PING"),
            Err(IpcResponse::Incompatible)
        );
        assert_eq!(
            parse_command("SNIPE/1 DELETE_ALL"),
            Err(IpcResponse::Rejected)
        );
        assert_eq!(
            parse_command("SHUTDOWN_FOR_UPDATE"),
            Err(IpcResponse::Rejected)
        );
    }
}
