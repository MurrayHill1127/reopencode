//! Memory cache implementation for storage module
//!
//! Provides an in-memory LRU cache with TTL support for reducing disk I/O.
//! Corresponds to TypeScript cache implementations across the codebase.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Cache configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheConfig {
    /// Maximum memory usage in bytes
    pub max_memory: usize,
    
    /// Entry time-to-live in seconds
    pub ttl_seconds: u64,
    
    /// Maximum number of entries
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64MB
            ttl_seconds: 300,              // 5 minutes
            max_entries: 1000,
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Serialized value
    data: Vec<u8>,
    /// Creation timestamp
    created_at: Instant,
    /// Last access timestamp
    accessed_at: Instant,
    /// Entry size in bytes
    size: usize,
}

/// Cache statistics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    /// Number of entries
    pub entries: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Maximum allowed size
    pub max_size: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
}

/// In-memory cache with LRU eviction and TTL support
/// 
/// Thread-safe through `Arc<RwLock<>>`. Uses serialized bytes internally
/// to support any serializable type.
/// 
/// # Example
/// 
/// ```rust
/// use reopencode::storage::{MemoryCache, CacheConfig};
/// 
/// let cache = MemoryCache::new(CacheConfig::default());
/// 
/// cache.set("user:123", serde_json::json!({"name": "Alice"}));
/// let value: Option<serde_json::Value> = cache.get("user:123");
/// assert!(value.is_some());
/// ```
#[derive(Clone)]
pub struct MemoryCache {
    inner: Arc<RwLock<CacheInner>>,
    config: CacheConfig,
}

struct CacheInner {
    entries: HashMap<String, CacheEntry>,
    total_size: usize,
    stats: CacheStats,
}

impl MemoryCache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                entries: HashMap::new(),
                total_size: 0,
                stats: CacheStats {
                    max_size: config.max_memory,
                    ..Default::default()
                },
            })),
            config,
        }
    }

    /// Set a cache entry
    /// 
    /// Serializes the value to JSON and stores it with the given key.
    /// Evicts entries if memory or count limits are exceeded.
    pub fn set<T: serde::Serialize + Clone>(&self, key: impl Into<String>, value: T) {
        let key = key.into();
        let serialized = serde_json::to_vec(&value).unwrap_or_default();
        let size = serialized.len();
        
        let entry = CacheEntry {
            data: serialized,
            created_at: Instant::now(),
            accessed_at: Instant::now(),
            size,
        };
        
        // Synchronous write for simplicity
        if let Ok(mut inner) = self.inner.try_write() {
            // Evict entries if needed
            while inner.total_size + size > self.config.max_memory && !inner.entries.is_empty() {
                self.evict_lru(&mut inner);
            }
            
            // Evict if too many entries
            while inner.entries.len() >= self.config.max_entries && !inner.entries.is_empty() {
                self.evict_lru(&mut inner);
            }
            
            // Insert new entry
            if let Some(old) = inner.entries.insert(key.clone(), entry) {
                inner.total_size -= old.size;
            }
            inner.total_size += size;
            inner.stats.entries = inner.entries.len();
            inner.stats.total_size = inner.total_size;
        }
    }

    /// Get a cache entry
    /// 
    /// Returns `None` if the key doesn't exist or the entry has expired.
    /// Updates the access time on hit.
    pub fn get<T: serde::de::DeserializeOwned + Clone>(&self, key: &str) -> Option<T> {
        if let Ok(mut inner) = self.inner.try_write() {
            let entry_info = inner.entries.get(key).map(|e| {
                (e.data.clone(), e.created_at.elapsed(), e.size)
            });
            
            match entry_info {
                Some((data, elapsed, size)) => {
                    if elapsed > Duration::from_secs(self.config.ttl_seconds) {
                        inner.total_size -= size;
                        inner.entries.remove(key);
                        inner.stats.misses += 1;
                        inner.stats.entries = inner.entries.len();
                        inner.stats.total_size = inner.total_size;
                        return None;
                    }
                    
                    if let Some(entry) = inner.entries.get_mut(key) {
                        entry.accessed_at = Instant::now();
                    }
                    inner.stats.hits += 1;
                    
                    return serde_json::from_slice(&data).ok();
                }
                None => {
                    inner.stats.misses += 1;
                }
            }
        }
        None
    }

    /// Remove a cache entry
    pub fn remove(&self, key: &str) {
        if let Ok(mut inner) = self.inner.try_write() {
            if let Some(removed) = inner.entries.remove(key) {
                inner.total_size -= removed.size;
                inner.stats.entries = inner.entries.len();
                inner.stats.total_size = inner.total_size;
            }
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        if let Ok(mut inner) = self.inner.try_write() {
            inner.entries.clear();
            inner.total_size = 0;
            inner.stats.entries = 0;
            inner.stats.total_size = 0;
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(inner) = self.inner.try_read() {
            inner.stats.clone()
        } else {
            CacheStats::default()
        }
    }

    /// Check if a key exists (without updating access time)
    pub fn contains(&self, key: &str) -> bool {
        if let Ok(inner) = self.inner.try_read() {
            if let Some(entry) = inner.entries.get(key) {
                // Check if expired
                if entry.created_at.elapsed() > Duration::from_secs(self.config.ttl_seconds) {
                    return false;
                }
                return true;
            }
        }
        false
    }

    /// Evict the least recently used entry
    fn evict_lru(&self, inner: &mut CacheInner) {
        if let Some(lru_key) = inner
            .entries
            .iter()
            .min_by_key(|(_, e)| e.accessed_at)
            .map(|(k, _)| k.clone())
        {
            if let Some(removed) = inner.entries.remove(&lru_key) {
                inner.total_size -= removed.size;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_get() {
        let cache = MemoryCache::new(CacheConfig::default());
        
        cache.set("test-key", "test-value");
        let value: Option<String> = cache.get("test-key");
        assert_eq!(value, Some("test-value".to_string()));
    }

    #[test]
    fn test_cache_remove() {
        let cache = MemoryCache::new(CacheConfig::default());
        
        cache.set("test-key", "test-value");
        assert!(cache.contains("test-key"));
        
        cache.remove("test-key");
        assert!(!cache.contains("test-key"));
    }

    #[test]
    fn test_cache_clear() {
        let cache = MemoryCache::new(CacheConfig::default());
        
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        
        cache.clear();
        
        assert!(!cache.contains("key1"));
        assert!(!cache.contains("key2"));
    }

    #[test]
    fn test_cache_stats() {
        let cache = MemoryCache::new(CacheConfig::default());
        
        cache.set("key1", "value1");
        cache.set("key2", "value2");
        
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        assert!(stats.total_size > 0);
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let config = CacheConfig {
            ttl_seconds: 0, // Immediate expiration
            ..Default::default()
        };
        let cache = MemoryCache::new(config);
        
        cache.set("test-key", "test-value");
        
        // Wait a tiny bit for expiration
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let value: Option<String> = cache.get("test-key");
        assert!(value.is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig {
            max_memory: 100,
            max_entries: 2,
            ..Default::default()
        };
        let cache = MemoryCache::new(config);
        
        // Add entries that exceed limits
        cache.set("key1", "x".repeat(50));
        cache.set("key2", "x".repeat(50));
        cache.set("key3", "x".repeat(50)); // Should trigger eviction
        
        let stats = cache.stats();
        assert!(stats.entries <= 2);
    }

    #[test]
    fn test_cache_json_values() {
        let cache = MemoryCache::new(CacheConfig::default());
        
        let value = serde_json::json!({
            "name": "Alice",
            "age": 30,
            "active": true
        });
        
        cache.set("user:123", value.clone());
        let retrieved: Option<serde_json::Value> = cache.get("user:123");
        
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_cache_hit_miss_stats() {
        let cache = MemoryCache::new(CacheConfig::default());
        
        cache.set("key1", "value1");
        
        // Hit
        let _: Option<String> = cache.get("key1");
        
        // Miss
        let _: Option<String> = cache.get("nonexistent");
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }
}