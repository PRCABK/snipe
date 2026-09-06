use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub const MUTEX_NAME: &str = "Local\\Snipe_SingleInstance_Mutex";
pub const PIPE_NAME: &str = "\\\\.\\pipe\\Snipe_IPC_Pipe";

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

pub fn send_command_to_primary(command: &str) -> bool {
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        if let Ok(mut file) = OpenOptions::new().write(true).read(true).open(PIPE_NAME) {
            let _ = file.write_all(format!("{}\n", command).as_bytes());
            let mut response = [0u8; 128];
            let _ = file.read(&mut response);
            info!("Sent command '{}' to existing instance", command);
            return true;
        }
    }
    warn!("Failed to connect to existing instance pipe");
    false
}

pub fn start_ipc_server(cmd_tx: UnboundedSender<String>, shutdown_flag: Arc<AtomicBool>) {
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
                Ok(s) => {
                    is_first = false;
                    s
                }
                Err(e) => {
                    warn!("Failed to create named pipe instance: {}", e);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
            };

            if server.connect().await.is_ok() {
                let (reader, mut writer) = tokio::io::split(server);
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();
                if buf_reader.read_line(&mut line).await.is_ok() {
                    let cmd = line.trim().to_string();
                    info!("Received IPC command: {}", cmd);
                    let _ = cmd_tx.send(cmd);
                    let _ = writer.write_all(b"OK\n").await;
                }
            }
        }
    });

    #[cfg(not(windows))]
    {
        let _ = cmd_tx;
        let _ = shutdown_flag;
    }
}
