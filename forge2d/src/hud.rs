use anyhow::Result;

use crate::{
    math::{Camera2D, Vec2},
    render::{Frame, FontHandle, Renderer, Sprite, TextureHandle},
};

/// Text element to be drawn in screen-space HUD coordinates (pixels).
pub struct HudText {
    pub text: String,
    pub font: FontHandle,
    pub size: f32,
    pub position: Vec2,      // screen-space pixels (0,0 = top-left)
    pub color: [f32; 4],
}

/// Sprite element to be drawn in screen-space HUD coordinates (pixels).
pub struct HudSprite {
    pub sprite: Sprite,
    pub position: Vec2,      // screen-space pixels (0,0 = top-left)
}

/// Simple rectangle element for panels/bars, drawn using a 1x1 white texture.
pub struct HudRect {
    pub position: Vec2,      // top-left in screen-space pixels
    pub size: Vec2,          // width/height in pixels
    pub color: [f32; 4],     // RGBA
}

enum HudElement {
    Text(HudText),
    Sprite(HudSprite),
    Rect(HudRect),
}

/// A layer of HUD elements rendered in screen space on top of the world.
pub struct HudLayer {
    elements: Vec<HudElement>,
    rect_texture: Option<TextureHandle>,
}

impl HudLayer {
    /// Create an empty HUD layer.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            rect_texture: None,
        }
    }

    /// Remove all HUD elements.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Add a text element to the HUD.
    pub fn add_text(&mut self, text: HudText) {
        self.elements.push(HudElement::Text(text));
    }

    /// Add a sprite element to the HUD.
    pub fn add_sprite(&mut self, sprite: HudSprite) {
        self.elements.push(HudElement::Sprite(sprite));
    }

    /// Add a rectangle element to the HUD.
    pub fn add_rect(&mut self, rect: HudRect) {
        self.elements.push(HudElement::Rect(rect));
    }

    /// Draw all HUD elements in screen space.
    ///
    /// This should typically be called after world rendering, using the same
    /// frame but with a fixed "HUD camera" that maps pixels directly.
    pub fn draw(&mut self, renderer: &mut Renderer, frame: &mut Frame) -> Result<()> {
        let hud_camera = Camera2D::default();

        // Lazily create a 1x1 white texture if we need to draw any rects.
        if self.rect_texture.is_none()
            && self
                .elements
                .iter()
                .any(|e| matches!(e, HudElement::Rect(_)))
        {
            let data = [255u8, 255, 255, 255];
            // Rect texture is not a font, use linear filtering
            let tex = renderer.load_texture_from_rgba(&data, 1, 1)?;
            self.rect_texture = Some(tex);
        }

        for element in &self.elements {
            match element {
                HudElement::Text(ht) => {
                    // Let renderer handle glyph rasterization internally.
                    renderer.draw_text(
                        frame,
                        &ht.text,
                        ht.font,
                        ht.size,
                        // Screen-space position: x,y as pixels from top-left.
                        // With Camera2D::default and our orthographic projection,
                        // world = screen, so we can pass directly.
                        ht.position,
                        ht.color,
                        &hud_camera,
                    )?;
                }
                HudElement::Sprite(hs) => {
                    let mut sprite = hs.sprite.clone();
                    sprite.transform.position = hs.position;
                    renderer.draw_sprite(frame, &sprite, &hud_camera)?;
                }
                HudElement::Rect(hr) => {
                    if let Some(tex) = self.rect_texture {
                        let mut sprite = Sprite::new(tex);
                        sprite.tint = hr.color;
                        sprite.transform.position = hr.position;
                        // 1x1 base texture; scale directly to pixel size.
                        sprite.transform.scale = hr.size;
                        renderer.draw_sprite(frame, &sprite, &hud_camera)?;
                    }
                }
            }
        }

        Ok(())
    }
}


