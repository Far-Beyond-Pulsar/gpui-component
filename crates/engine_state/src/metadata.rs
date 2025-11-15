//! Metadata Storage
//!
//! Thread-safe key-value storage for engine metadata

use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Metadata {
    store: Arc<DashMap<String, String>>,
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.store.get(key).map(|v| v.clone())
    }

    pub fn set(&self, key: String, value: String) {
        self.store.insert(key, value);
    }

    pub fn remove(&self, key: &str) -> Option<String> {
        self.store.remove(key).map(|(_, v)| v)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}
