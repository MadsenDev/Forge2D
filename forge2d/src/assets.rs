use std::collections::HashMap;

use crate::render::{Renderer, TextureHandle};

/// Manages cached assets (textures, and future: sounds, fonts, etc.).
pub struct AssetManager {
    textures: HashMap<String, TextureHandle>,
}

impl AssetManager {
    /// Create a new asset manager with no cached assets.
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    /// Load a texture from a file path, caching it if already loaded.
    ///
    /// Returns the texture handle. If the texture was previously loaded,
    /// returns the cached handle without reloading from disk.
    pub fn load_texture(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
    ) -> anyhow::Result<TextureHandle> {
        // Check cache first
        if let Some(handle) = self.textures.get(path) {
            return Ok(*handle);
        }

        // Load and cache
        let handle = renderer.load_texture_from_file(path)?;
        self.textures.insert(path.to_string(), handle);
        Ok(handle)
    }

    /// Load a texture from bytes, caching it by a given key.
    ///
    /// Useful for embedded assets or dynamically generated textures.
    pub fn load_texture_from_bytes(
        &mut self,
        renderer: &mut Renderer,
        key: &str,
        bytes: &[u8],
    ) -> anyhow::Result<TextureHandle> {
        // Check cache first
        if let Some(handle) = self.textures.get(key) {
            return Ok(*handle);
        }

        // Load and cache
        let handle = renderer.load_texture_from_bytes(bytes)?;
        self.textures.insert(key.to_string(), handle);
        Ok(handle)
    }

    /// Get a cached texture handle by key, if it exists.
    pub fn get_texture(&self, key: &str) -> Option<TextureHandle> {
        self.textures.get(key).copied()
    }

    /// Check if a texture is already cached.
    pub fn has_texture(&self, key: &str) -> bool {
        self.textures.contains_key(key)
    }

    /// Clear all cached textures (they will be reloaded on next access).
    pub fn clear(&mut self) {
        self.textures.clear();
    }

    /// Remove a specific texture from the cache.
    pub fn unload_texture(&mut self, key: &str) {
        self.textures.remove(key);
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

