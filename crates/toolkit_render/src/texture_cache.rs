use std::collections::HashMap;
use toolkit_core::TextureId;

struct CacheEntry {
    texture: crate::texture::GpuTexture,
    last_used_frame: u64,
    byte_size: u64,
}

pub struct TextureCache {
    entries: HashMap<TextureId, CacheEntry>,
    max_bytes: u64,
    current_bytes: u64,
    current_frame: u64,
}

impl TextureCache {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            entries: HashMap::new(),
            max_bytes,
            current_bytes: 0,
            current_frame: 0,
        }
    }

    pub fn insert(&mut self, texture: crate::texture::GpuTexture) {
        let byte_size = texture.byte_size();
        let id = texture.id;

        if let Some(old) = self.entries.remove(&id) {
            self.current_bytes -= old.byte_size;
        }

        while self.current_bytes + byte_size > self.max_bytes && !self.entries.is_empty() {
            self.evict_lru();
        }

        self.current_bytes += byte_size;
        self.entries.insert(
            id,
            CacheEntry {
                texture,
                last_used_frame: self.current_frame,
                byte_size,
            },
        );
    }

    pub fn get(&mut self, id: TextureId) -> Option<&crate::texture::GpuTexture> {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.last_used_frame = self.current_frame;
            Some(&entry.texture)
        } else {
            None
        }
    }

    pub fn remove(&mut self, id: TextureId) -> bool {
        if let Some(entry) = self.entries.remove(&id) {
            self.current_bytes -= entry.byte_size;
            true
        } else {
            false
        }
    }

    pub fn contains(&self, id: TextureId) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    pub fn current_bytes(&self) -> u64 {
        self.current_bytes
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_bytes = 0;
    }

    fn evict_lru(&mut self) {
        let lru_id = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_used_frame)
            .map(|(id, _)| *id);

        if let Some(id) = lru_id {
            if let Some(entry) = self.entries.remove(&id) {
                self.current_bytes -= entry.byte_size;
                log::debug!("Evicted texture {} ({} bytes)", id, entry.byte_size);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cache_no_gpu() -> TextureCache {
        TextureCache::new(1024 * 1024)
    }

    #[test]
    fn new_cache_is_empty() {
        let cache = make_cache_no_gpu();
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.current_bytes(), 0);
    }

    #[test]
    fn advance_frame_increments() {
        let mut cache = make_cache_no_gpu();
        assert_eq!(cache.current_frame, 0);
        cache.advance_frame();
        assert_eq!(cache.current_frame, 1);
    }

    #[test]
    fn clear_resets() {
        let mut cache = make_cache_no_gpu();
        cache.clear();
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.current_bytes(), 0);
    }
}
