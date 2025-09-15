use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

// 缓存项结构
#[derive(Debug, Clone)]
pub struct CacheItem<T> {
    value: T,
    expiry: Option<SystemTime>,
}

// 简单缓存实现
pub struct SimpleCache {
    inner: Mutex<HashMap<String, CacheItem<String>>>,
}

impl SimpleCache {
    // 创建新的缓存实例
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    // 设置缓存项，可选设置过期时间（秒）
    pub fn set(&self, key: &str, value: String, ttl: Option<u64>) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| format!("Failed to lock cache: {:?}", e))?;
        
        let expiry = ttl.map(|seconds| {
            SystemTime::now() + Duration::from_secs(seconds)
        });
        
        inner.insert(key.to_string(), CacheItem {
            value,
            expiry,
        });
        
        Ok(())
    }

    // 获取缓存项
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let mut inner = self.inner.lock().map_err(|e| format!("Failed to lock cache: {:?}", e))?;
        
        // 清理过期的项目
        self.cleanup_expired(&mut inner);
        
        if let Some(item) = inner.get(key) {
            Ok(Some(item.value.clone()))
        } else {
            Ok(None)
        }
    }

    // 删除缓存项
    pub fn remove(&self, key: &str) -> Result<bool, String> {
        let mut inner = self.inner.lock().map_err(|e| format!("Failed to lock cache: {:?}", e))?;
        Ok(inner.remove(key).is_some())
    }

    // 清理过期的缓存项
    fn cleanup_expired(&self, inner: &mut HashMap<String, CacheItem<String>>) {
        let now = SystemTime::now();
        inner.retain(|_, item| {
            if let Some(expiry) = item.expiry {
                match expiry.duration_since(now) {
                    Ok(_) => true,  // 未过期
                    Err(_) => false, // 已过期
                }
            } else {
                true  // 永不过期
            }
        });
    }

    // 获取缓存项数量
    pub fn len(&self) -> Result<usize, String> {
        let inner = self.inner.lock().map_err(|e| format!("Failed to lock cache: {:?}", e))?;
        Ok(inner.len())
    }

    // 清空缓存
    pub fn clear(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| format!("Failed to lock cache: {:?}", e))?;
        inner.clear();
        Ok(())
    }
}

// 缓存类型别名，方便使用
pub type Cache = Arc<SimpleCache>;

// 初始化缓存
pub fn init_cache() -> Cache {
    Arc::new(SimpleCache::new())
}