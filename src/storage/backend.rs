//! Storage backend abstraction

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::storage::{GlobalPath, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendType {
    #[default]
    Sqlite,
    Json,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    fn backend_type(&self) -> BackendType;

    async fn exists(&self, key: &[&str]) -> bool;

    async fn read_raw(&self, key: &[&str]) -> Result<Option<Vec<u8>>, StorageError>;

    async fn write_raw(&self, key: &[&str], data: &[u8]) -> Result<(), StorageError>;

    async fn remove(&self, key: &[&str]) -> Result<(), StorageError>;

    async fn list(&self, prefix: &[&str]) -> Result<Vec<Vec<String>>, StorageError>;
}

pub async fn read<T: serde::de::DeserializeOwned + Send>(
    backend: &dyn StorageBackend,
    key: &[&str],
) -> Result<Option<T>, StorageError> {
    match backend.read_raw(key).await? {
        Some(data) => {
            let value: T = serde_json::from_slice(&data)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

pub async fn write<T: serde::Serialize + Send>(
    backend: &dyn StorageBackend,
    key: &[&str],
    value: &T,
) -> Result<(), StorageError> {
    let data = serde_json::to_vec_pretty(value)?;
    backend.write_raw(key, &data).await
}

pub struct JsonBackend {
    base_dir: PathBuf,
    locks: Arc<DashMap<String, Arc<RwLock<()>>>>,
}

impl JsonBackend {
    pub fn new(base_dir: &Path) -> Result<Self, StorageError> {
        std::fs::create_dir_all(base_dir)?;

        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            locks: Arc::new(DashMap::new()),
        })
    }

    pub fn from_global_path() -> Result<Self, StorageError> {
        Self::new(&GlobalPath::get().data)
    }

    fn key_to_path(&self, key: &[&str]) -> PathBuf {
        let mut path = self.base_dir.clone();
        for part in key {
            path = path.join(part);
        }
        path
    }

    fn get_lock(&self, key: &[&str]) -> Arc<RwLock<()>> {
        let key_str = key.join("/");
        self.locks
            .entry(key_str)
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone()
    }

    fn list_recursive(
        &self,
        dir: &Path,
        prefix: &[String],
        results: &mut Vec<Vec<String>>,
    ) -> Result<(), StorageError> {
        if !dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();

            let mut key: Vec<String> = prefix.to_vec();
            key.push(name);

            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
                results.push(key);
            } else if path.is_dir() {
                self.list_recursive(&path, &key, results)?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for JsonBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Json
    }

    async fn exists(&self, key: &[&str]) -> bool {
        let path = self.key_to_path(key);
        path.exists()
    }

    async fn read_raw(&self, key: &[&str]) -> Result<Option<Vec<u8>>, StorageError> {
        if key.is_empty() {
            return Err(StorageError::InvalidKey("empty key".to_string()));
        }

        let path = self.key_to_path(key);

        if !path.exists() {
            return Ok(None);
        }

        let lock = self.get_lock(key);
        let _guard = lock.read().await;

        let content = tokio::fs::read(&path).await?;

        Ok(Some(content))
    }

    async fn write_raw(&self, key: &[&str], data: &[u8]) -> Result<(), StorageError> {
        if key.is_empty() {
            return Err(StorageError::InvalidKey("empty key".to_string()));
        }

        let path = self.key_to_path(key);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let lock = self.get_lock(key);
        let _guard = lock.write().await;

        tokio::fs::write(&path, data).await?;

        Ok(())
    }

    async fn remove(&self, key: &[&str]) -> Result<(), StorageError> {
        if key.is_empty() {
            return Err(StorageError::InvalidKey("empty key".to_string()));
        }

        let path = self.key_to_path(key);

        if path.exists() {
            let lock = self.get_lock(key);
            let _guard = lock.write().await;

            if path.is_file() {
                tokio::fs::remove_file(&path).await?;
            } else if path.is_dir() {
                tokio::fs::remove_dir_all(&path).await?;
            }
        }

        Ok(())
    }

    async fn list(&self, prefix: &[&str]) -> Result<Vec<Vec<String>>, StorageError> {
        let dir = self.key_to_path(prefix);

        if !dir.exists() || !dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let prefix_vec: Vec<String> = prefix.iter().map(|s| s.to_string()).collect();

        self.list_recursive(&dir, &prefix_vec, &mut results)?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_backend() -> (JsonBackend, TempDir) {
        let temp = TempDir::new().unwrap();
        let backend = JsonBackend::new(temp.path()).unwrap();
        (backend, temp)
    }

    #[tokio::test]
    async fn test_json_backend_write_read() {
        let (backend, _temp) = create_test_backend();

        let data = serde_json::json!({
            "name": "test",
            "value": 42
        });

        write(&backend, &["test", "data.json"], &data)
            .await
            .unwrap();

        let loaded: Option<serde_json::Value> =
            read(&backend, &["test", "data.json"]).await.unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded["name"], "test");
        assert_eq!(loaded["value"], 42);
    }

    #[tokio::test]
    async fn test_json_backend_exists() {
        let (backend, _temp) = create_test_backend();

        assert!(!backend.exists(&["nonexistent.json"]).await);

        write(&backend, &["test.json"], &"data").await.unwrap();
        assert!(backend.exists(&["test.json"]).await);
    }

    #[tokio::test]
    async fn test_json_backend_remove() {
        let (backend, _temp) = create_test_backend();

        write(&backend, &["to-delete.json"], &"data").await.unwrap();
        assert!(backend.exists(&["to-delete.json"]).await);

        backend.remove(&["to-delete.json"]).await.unwrap();
        assert!(!backend.exists(&["to-delete.json"]).await);
    }

    #[tokio::test]
    async fn test_json_backend_list() {
        let (backend, _temp) = create_test_backend();

        write(&backend, &["session", "proj1", "ses1.json"], &"s1")
            .await
            .unwrap();
        write(&backend, &["session", "proj1", "ses2.json"], &"s2")
            .await
            .unwrap();
        write(&backend, &["session", "proj2", "ses3.json"], &"s3")
            .await
            .unwrap();

        let keys = backend.list(&["session"]).await.unwrap();
        assert_eq!(keys.len(), 3);
    }

    #[tokio::test]
    async fn test_json_backend_read_nonexistent() {
        let (backend, _temp) = create_test_backend();

        let value: Option<String> = read(&backend, &["nonexistent.json"]).await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_json_backend_invalid_key() {
        let (backend, _temp) = create_test_backend();

        let result = backend.read_raw(&[]).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StorageError::InvalidKey(_)));
    }

    #[test]
    fn test_backend_type_default() {
        assert_eq!(BackendType::default(), BackendType::Sqlite);
    }
}
