use std::collections::HashMap;
use anyhow::{anyhow, Result};
use ab_glyph::{Font, FontArc, Glyph, ScaleFont};

use crate::render::TextureHandle;

/// A font loaded and ready for text rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle(pub(crate) u32);

/// Cached glyph information.
pub struct GlyphCacheEntry {
    pub texture: TextureHandle,
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

/// Text renderer that manages fonts and glyph caching.
pub struct TextRenderer {
    fonts: HashMap<FontHandle, FontArc>,
    next_font_id: u32,
    glyph_cache: HashMap<(FontHandle, char, u32), GlyphCacheEntry>, // (font, char, size) -> texture
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            next_font_id: 1,
            glyph_cache: HashMap::new(),
        }
    }

    /// Load a font from bytes (TTF/OTF format).
    pub fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle> {
        // Clone bytes to ensure 'static lifetime for FontArc
        let bytes = bytes.to_vec();
        let font = FontArc::try_from_vec(bytes)
            .map_err(|e| anyhow!("Failed to load font: {}", e))?;
        
        let handle = FontHandle(self.next_font_id);
        self.next_font_id += 1;
        self.fonts.insert(handle, font);
        
        Ok(handle)
    }

    /// Get a cached glyph texture or return None if not cached.
    pub fn get_glyph(&self, font_handle: FontHandle, ch: char, size: f32) -> Option<&GlyphCacheEntry> {
        let cache_key = (font_handle, ch, size as u32);
        self.glyph_cache.get(&cache_key)
    }
    
    pub(crate) fn has_glyph(&self, font: FontHandle, ch: char, size: f32) -> bool {
        let cache_key = (font, ch, size as u32);
        self.glyph_cache.contains_key(&cache_key)
    }
    
    pub(crate) fn get_font(&self, font: FontHandle) -> Option<&FontArc> {
        self.fonts.get(&font)
    }
    
    pub(crate) fn cache_glyph(&mut self, font: FontHandle, ch: char, size: f32, entry: GlyphCacheEntry) {
        let cache_key = (font, ch, size as u32);
        self.glyph_cache.insert(cache_key, entry);
    }

    /// Rasterize and cache a glyph.
    pub fn rasterize_glyph(
        &mut self,
        font_handle: FontHandle,
        ch: char,
        size: f32,
        load_texture_fn: impl FnOnce(&[u8], u32, u32) -> Result<TextureHandle>,
    ) -> Result<&GlyphCacheEntry> {
        let cache_key = (font_handle, ch, size as u32);
        
        if !self.glyph_cache.contains_key(&cache_key) {
            // Rasterize the glyph
            let font = self.fonts.get(&font_handle)
                .ok_or_else(|| anyhow!("Font handle not found"))?;
            
            let scale = ab_glyph::PxScale::from(size);
            let scaled_font = font.as_scaled(scale);
            
            let glyph_id = font.glyph_id(ch);
            let glyph = Glyph {
                id: glyph_id,
                scale,
                position: ab_glyph::point(0.0, 0.0),
            };
            
            // Rasterize the glyph
            if let Some(outlined) = scaled_font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let width = bounds.width().ceil() as u32;
                let height = bounds.height().ceil() as u32;
                
                if width > 0 && height > 0 {
                    // Create RGBA image
                    let mut image_data = vec![0u8; (width * height * 4) as usize];
                    
                    outlined.draw(|x, y, c| {
                        let x = x as u32;
                        let y = y as u32;
                        if x < width && y < height {
                            let idx = ((y * width + x) * 4) as usize;
                            let alpha = (c * 255.0) as u8;
                            image_data[idx] = 255;     // R
                            image_data[idx + 1] = 255; // G
                            image_data[idx + 2] = 255; // B
                            image_data[idx + 3] = alpha; // A
                        }
                    });
                    
                    // Load as texture using the provided function
                    let texture = load_texture_fn(&image_data, width, height)?;
                    
                    // Get advance width from the font (proper character spacing)
                    let mut advance = scaled_font.h_advance(glyph_id);
                    
                    // Calculate bearing from glyph bounds
                    // bearing_x: horizontal offset from origin to left edge
                    let bearing_x = bounds.min.x;
                    // bearing_y: vertical offset from baseline to top (inverted for screen coords)
                    let bearing_y = -bounds.min.y;
                    
                    // Robustness: Ensure advance is reasonable to prevent character overlap
                    let glyph_width = bounds.width();
                    if advance <= 0.0 || advance < glyph_width * 0.5 {
                        // Fallback: use glyph width + small padding if advance is too small
                        advance = glyph_width.max(1.0) + 2.0; // Small padding for safety
                    }
                    
                    self.glyph_cache.insert(cache_key, GlyphCacheEntry {
                        texture,
                        width: width as f32,
                        height: height as f32,
                        bearing_x,
                        bearing_y,
                        advance,
                    });
                }
            }
        }
        
        self.glyph_cache.get(&cache_key)
            .ok_or_else(|| anyhow!("Failed to rasterize glyph"))
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}
