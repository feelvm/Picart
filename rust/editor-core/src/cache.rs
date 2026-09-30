use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// Generic LRU with byte accounting + pressure callback.
/// Used for decoded tiles, preview textures, GPU resources.
pub struct LruCache<K, V> {
    cap_bytes: usize,
    used_bytes: usize,
    map: HashMap<K, (V, usize)>,
    order: VecDeque<K>,
    pub evictions: u64,
    pub hits: u64,
    pub misses: u64,
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    pub fn new(cap_bytes: usize) -> Self {
        Self { cap_bytes, used_bytes: 0, map: HashMap::new(), order: VecDeque::new(), evictions: 0, hits: 0, misses: 0 }
    }
    pub fn hit_rate(&self) -> f32 {
        let t = self.hits + self.misses;
        if t == 0 { 1.0 } else { self.hits as f32 / t as f32 }
    }
    pub fn used_bytes(&self) -> usize { self.used_bytes }

    pub fn get(&mut self, k: &K) -> Option<&V> {
        if self.map.contains_key(k) {
            self.hits += 1;
            self.promote(k);
            self.map.get(k).map(|(v, _)| v)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn put(&mut self, k: K, v: V, bytes: usize) {
        if let Some((_, old)) = self.map.remove(&k) {
            self.used_bytes = self.used_bytes.saturating_sub(old);
            self.order.retain(|x| x != &k);
        }
        while self.used_bytes + bytes > self.cap_bytes && !self.order.is_empty() {
            if let Some(old_k) = self.order.pop_front() {
                if let Some((_, b)) = self.map.remove(&old_k) {
                    self.used_bytes = self.used_bytes.saturating_sub(b);
                    self.evictions += 1;
                }
            }
        }
        self.used_bytes += bytes;
        self.order.push_back(k.clone());
        self.map.insert(k, (v, bytes));
    }

    fn promote(&mut self, k: &K) {
        self.order.retain(|x| x != k);
        self.order.push_back(k.clone());
    }
}

/// Cache keys
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileKey { pub asset: u64, pub tx: u32, pub ty: u32, pub level: u8 }
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextureKey(pub String);
