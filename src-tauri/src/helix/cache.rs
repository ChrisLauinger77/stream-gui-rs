use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheClass {
    Live,
    Metadata,
    Categories,
}
impl CacheClass {
    pub fn ttl(self) -> Duration {
        Duration::from_secs(match self {
            Self::Live => 15,
            Self::Metadata => 300,
            Self::Categories => 3600,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CachePolicy {
    Fresh,
    Refresh,
    AllowStale,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Network,
    Cached,
    Stale,
}
#[derive(Clone, Debug)]
pub struct Cached<T> {
    pub value: T,
    pub freshness: Freshness,
    pub age: Duration,
}
struct Entry {
    bytes: Vec<u8>,
    inserted: Instant,
    class: CacheClass,
}
#[derive(Default)]
pub struct TwitchCache {
    entries: HashMap<String, Entry>,
    bytes: usize,
}
impl TwitchCache {
    pub(crate) fn get(&self, key: &str, policy: CachePolicy) -> Option<Cached<Vec<u8>>> {
        if policy == CachePolicy::Refresh {
            return None;
        }
        let entry = self.entries.get(key)?;
        let age = entry.inserted.elapsed();
        let stale = age >= entry.class.ttl();
        if stale && policy != CachePolicy::AllowStale {
            return None;
        }
        Some(Cached {
            value: entry.bytes.clone(),
            freshness: if stale {
                Freshness::Stale
            } else {
                Freshness::Cached
            },
            age,
        })
    }
    pub(crate) fn insert(&mut self, key: String, bytes: Vec<u8>, class: CacheClass) {
        if bytes.len() > 2 * 1024 * 1024 {
            return;
        }
        if let Some(old) = self.entries.remove(&key) {
            self.bytes -= old.bytes.len();
        }
        while self.entries.len() >= 128 || self.bytes + bytes.len() > 8 * 1024 * 1024 {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.inserted)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            self.bytes -= self
                .entries
                .remove(&oldest)
                .expect("oldest entry")
                .bytes
                .len();
        }
        self.bytes += bytes.len();
        self.entries.insert(
            key,
            Entry {
                bytes,
                inserted: Instant::now(),
                class,
            },
        );
    }
    pub fn invalidate(&mut self, class: CacheClass) {
        self.entries.retain(|_, entry| entry.class != class);
        self.bytes = self.entries.values().map(|e| e.bytes.len()).sum();
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
