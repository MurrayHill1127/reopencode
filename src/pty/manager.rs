use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::bus::{
    Bus, PTY_CREATED, PTY_DELETED, PTY_EXITED, PTY_UPDATED, PtyCreatedProperties,
    PtyDeletedProperties, PtyExitedProperties, PtyInfo as BusPtyInfo, PtyUpdatedProperties,
};

use super::session::{PtyHandle, spawn_pty};
use super::types::{
    CreatePtyRequest, DEFAULT_COLS, DEFAULT_ROWS, PtyError, PtyInfo, PtyOutput, UpdatePtyRequest,
    generate_pty_id,
};

fn to_bus_pty_info(info: &PtyInfo) -> BusPtyInfo {
    BusPtyInfo {
        id: info.id.clone(),
        title: info.title.clone(),
        command: info.command.clone(),
        args: info.args.clone(),
        cwd: info.cwd.clone(),
        status: info.status.to_string(),
        pid: info.pid,
    }
}

pub struct PtyManager {
    handles: DashMap<String, PtyHandle>,
    readers: Mutex<HashMap<String, std::thread::JoinHandle<()>>>,
    bus: Option<Arc<Bus>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            handles: DashMap::new(),
            readers: Mutex::new(HashMap::new()),
            bus: None,
        }
    }

    pub fn with_bus(mut self, bus: Arc<Bus>) -> Self {
        self.bus = Some(bus);
        self
    }

    pub async fn create(&self, req: CreatePtyRequest) -> Result<PtyInfo, PtyError> {
        let id = generate_pty_id();

        let command = req.command.clone().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "cmd.exe".to_string()
            } else {
                std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
            }
        });

        let cwd = req.cwd.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "/".to_string())
        });

        let mut args = req.args.clone().unwrap_or_default();

        if command.ends_with("sh") && !args.contains(&"-l".to_string()) {
            args.push("-l".to_string());
        }

        let rows = req.rows.unwrap_or(DEFAULT_ROWS);
        let cols = req.cols.unwrap_or(DEFAULT_COLS);

        let (handle, reader_handle) = spawn_pty(
            id.clone(),
            command.clone(),
            args.clone(),
            cwd.clone(),
            req.env.clone(),
            rows,
            cols,
            req.title.clone(),
        )?;

        let info = handle.get_info().await;

        self.handles.insert(id.clone(), handle);
        self.readers.lock().await.insert(id.clone(), reader_handle);

        if let Some(bus) = &self.bus {
            bus.publish(
                &PTY_CREATED,
                PtyCreatedProperties {
                    info: to_bus_pty_info(&info),
                },
            )
            .await;
        }

        Ok(info)
    }

    pub fn list(&self) -> Vec<PtyInfo> {
        let rt = tokio::runtime::Handle::current();
        self.handles
            .iter()
            .map(|entry| {
                let handle = entry.value();
                rt.block_on(handle.get_info())
            })
            .collect()
    }

    pub async fn get(&self, id: &str) -> Option<PtyInfo> {
        let handle = self.handles.get(id)?;
        Some(handle.get_info().await)
    }

    pub async fn update(&self, id: &str, _req: UpdatePtyRequest) -> Result<PtyInfo, PtyError> {
        let handle = self
            .handles
            .get(id)
            .ok_or_else(|| PtyError::NotFound(id.to_string()))?;

        if handle.is_exited() {
            return Err(PtyError::AlreadyExited(id.to_string()));
        }

        let info = handle.get_info().await;

        if let Some(bus) = &self.bus {
            bus.publish(
                &PTY_UPDATED,
                PtyUpdatedProperties {
                    info: to_bus_pty_info(&info),
                },
            )
            .await;
        }

        Ok(info)
    }

    pub async fn resize(&self, _id: &str, _rows: u16, _cols: u16) -> Result<(), PtyError> {
        Ok(())
    }

    pub async fn write(&self, id: &str, data: &[u8]) -> Result<(), PtyError> {
        let handle = self
            .handles
            .get(id)
            .ok_or_else(|| PtyError::NotFound(id.to_string()))?;

        handle.write(data).await
    }

    pub async fn read(&self, id: &str, _cursor: Option<usize>) -> Result<PtyOutput, PtyError> {
        let _handle = self
            .handles
            .get(id)
            .ok_or_else(|| PtyError::NotFound(id.to_string()))?;

        Ok(PtyOutput {
            data: String::new(),
            cursor: 0,
        })
    }

    pub async fn kill(&self, id: &str) -> Result<(), PtyError> {
        let handle = self
            .handles
            .get(id)
            .ok_or_else(|| PtyError::NotFound(id.to_string()))?;

        handle.mark_exited(0).await;

        if let Some(bus) = &self.bus {
            let info = handle.get_info().await;
            bus.publish(
                &PTY_EXITED,
                PtyExitedProperties {
                    id: id.to_string(),
                    exit_code: info.exit_code.unwrap_or(0),
                },
            )
            .await;
        }

        Ok(())
    }

    pub async fn remove(&self, id: &str) -> Result<bool, PtyError> {
        self.handles.remove(id);

        if let Some(reader) = self.readers.lock().await.remove(id) {
            drop(reader);
        }

        if let Some(bus) = &self.bus {
            bus.publish(&PTY_DELETED, PtyDeletedProperties { id: id.to_string() })
                .await;
        }

        Ok(true)
    }

    pub fn subscribe_output(&self, id: &str) -> Option<tokio::sync::broadcast::Receiver<Vec<u8>>> {
        let handle = self.handles.get(id)?;
        Some(handle.subscribe_output())
    }

    pub async fn cleanup(&self) {
        let ids: Vec<String> = self.handles.iter().map(|e| e.key().clone()).collect();

        for id in ids {
            let _ = self.remove(&id).await;
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_PTY_MANAGER: once_cell::sync::Lazy<PtyManager> =
    once_cell::sync::Lazy::new(PtyManager::new);

pub fn global() -> &'static PtyManager {
    &GLOBAL_PTY_MANAGER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_manager_new() {
        let manager = PtyManager::new();
        let list = manager.list();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_global_manager() {
        let manager = global();
        let list = manager.list();
        assert!(list.is_empty());
    }
}
