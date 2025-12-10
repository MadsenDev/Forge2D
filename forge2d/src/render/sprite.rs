use crate::math::Transform2D;

/// Opaque handle used to reference textures owned by the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub(crate) u32);

/// Simple sprite combining a texture and transform metadata.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub texture: TextureHandle,
    pub transform: Transform2D,
    /// Multiplicative tint applied to the sampled texture color.
    pub tint: [f32; 4],
}

impl Sprite {
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            transform: Transform2D::default(),
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }
}
