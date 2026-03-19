use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use portable_pty::{PtySize, native_pty_system};

use super::types::{BUFFER_LIMIT, PtyInfo, PtyStatus};

pub struct PtyState {
    pub info: PtyInfo,
    pub buffer: String,
    pub buffer_cursor: usize,
    pub output_cursor: usize,
    pub output_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
    exited: bool,
}

impl PtyState {
    pub fn new(info: PtyInfo) -> Self {
        let (output_tx, _) = tokio::sync::broadcast::channel(256);
        Self {
            info,
            buffer: String::new(),
            buffer_cursor: 0,
            output_cursor: 0,
            output_tx,
            exited: false,
        }
    }
}

pub struct PtyHandle {
    state: Arc<Mutex<PtyState>>,
    exited: Arc<AtomicBool>,
    writer: std::sync::Mutex<Option<Box<dyn Write + Send>>>,
}

impl PtyHandle {
    pub fn new(state: Arc<Mutex<PtyState>>, writer: Box<dyn Write + Send>) -> Self {
        Self {
            state,
            exited: Arc::new(AtomicBool::new(false)),
            writer: std::sync::Mutex::new(Some(writer)),
        }
    }

    pub async fn get_info(&self) -> PtyInfo {
        self.state.lock().await.info.clone()
    }

    pub async fn write(&self, data: &[u8]) -> Result<(), super::types::PtyError> {
        if self.exited.load(Ordering::SeqCst) {
            return Err(super::types::PtyError::AlreadyExited(
                self.state.lock().await.info.id.clone(),
            ));
        }

        let mut writer = self.writer.lock().map_err(|_| {
            super::types::PtyError::WriteFailed("Failed to lock writer".to_string())
        })?;

        if let Some(w) = writer.as_mut() {
            w.write_all(data)
                .map_err(|e| super::types::PtyError::WriteFailed(e.to_string()))?;
        }

        Ok(())
    }

    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    pub async fn mark_exited(&self, exit_code: i32) {
        self.exited.store(true, Ordering::SeqCst);

        let mut state = self.state.lock().await;
        state.info.status = PtyStatus::Exited;
        state.info.exit_code = Some(exit_code);
        state.exited = true;
    }

    pub fn subscribe_output(&self) -> tokio::sync::broadcast::Receiver<Vec<u8>> {
        let state = self.state.blocking_lock();
        state.output_tx.subscribe()
    }
}

pub fn spawn_pty(
    id: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    env: Option<std::collections::HashMap<String, String>>,
    rows: u16,
    cols: u16,
    title: Option<String>,
) -> Result<(PtyHandle, std::thread::JoinHandle<()>), super::types::PtyError> {
    let pty_system = native_pty_system();

    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .map_err(|e| super::types::PtyError::SpawnFailed(e.to_string()))?;

    let mut cmd = portable_pty::cmdbuilder::CommandBuilder::new(&command);
    cmd.args(args.iter().map(|s| s.as_str()));
    cmd.cwd(&cwd);

    let mut final_env = std::collections::HashMap::new();
    final_env.insert("TERM".to_string(), "xterm-256color".to_string());
    final_env.insert("OPENCODE_TERMINAL".to_string(), "1".to_string());

    if let Some(custom_env) = env {
        final_env.extend(custom_env);
    }

    for (key, value) in &final_env {
        cmd.env(key, value);
    }

    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| super::types::PtyError::SpawnFailed(e.to_string()))?;

    drop(pair.slave);

    let pid: u32 = 0;

    let mut info = PtyInfo::new(id, command, args, cwd, pid);
    if let Some(t) = title {
        info = info.with_title(t);
    }

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| super::types::PtyError::SpawnFailed(e.to_string()))?;

    let state = Arc::new(Mutex::new(PtyState::new(info)));
    let handle = PtyHandle::new(Arc::clone(&state), writer);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| super::types::PtyError::ReadFailed(e.to_string()))?;

    let reader_handle = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        let mut s = state.lock().await;
                        let data_str = String::from_utf8_lossy(&data).to_string();
                        s.output_cursor += data.len();
                        s.buffer.push_str(&data_str);

                        if s.buffer.len() > BUFFER_LIMIT {
                            let excess = s.buffer.len() - BUFFER_LIMIT;
                            s.buffer = s.buffer[excess..].to_string();
                            s.buffer_cursor += excess;
                        }

                        let _ = s.output_tx.send(data);
                    });
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        break;
                    }
                }
            }
        }

        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut s = state.lock().await;
            s.info.status = PtyStatus::Exited;
            s.exited = true;
        });
    });

    Ok((handle, reader_handle))
}
