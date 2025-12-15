use std::{collections::HashMap, fs};

use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    vertex_attr_array, AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Extent3d, Features, FilterMode,
    FragmentState, ImageCopyTexture, ImageDataLayout, Instance, Limits, LoadOp, MultisampleState,
    Operations, Origin3d, PipelineLayoutDescriptor, PresentMode, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, Sampler, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, SurfaceConfiguration, SurfaceError, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    math::{Camera2D, Vec2},
    render::sprite::{Sprite, TextureHandle},
    render::text::{FontHandle, GlyphCacheEntry, TextRenderer},
};
use ab_glyph::{Font, Glyph, ScaleFont};

/// Queued sprite draw command (batched rendering)
struct SpriteDrawCommand {
    uniform_offset: u64,
    texture_handle: TextureHandle, // Store texture handle, look up bind group when flushing
}

/// Wrapper around wgpu surface/device setup and simple frame management.
pub struct Renderer {
    backend: WgpuBackend,
}

impl Renderer {
    pub fn new(window: &Window, vsync: bool) -> Result<Self> {
        let backend = WgpuBackend::new(window, vsync)?;
        Ok(Self { backend })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.backend.resize(new_size);
    }

    pub fn begin_frame(&mut self) -> Result<Frame> {
        self.backend.begin_frame()
    }

    pub fn clear(&mut self, frame: &mut Frame, color: [f32; 4]) -> Result<()> {
        self.backend.clear(frame, color)
    }

    pub fn draw_sprite(
        &mut self,
        frame: &mut Frame,
        sprite: &Sprite,
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend.draw_sprite(frame, sprite, camera)
    }

    pub fn end_frame(&mut self, frame: Frame) -> Result<()> {
        self.backend.end_frame(frame)
    }

    pub fn load_texture_from_file(&mut self, path: &str) -> Result<TextureHandle> {
        self.backend.load_texture_from_file(path)
    }

    pub fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle> {
        self.backend.load_texture_from_bytes(bytes)
    }

    /// Load a texture from raw RGBA8 data (no PNG decoding).
    ///
    /// This is useful for procedurally generated textures or tests.
    /// `data` must be `width * height * 4` bytes in RGBA8 format.
    pub fn load_texture_from_rgba(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<TextureHandle> {
        // Default to false (sprite texture) for backward compatibility
        self.backend
            .load_texture_from_rgba(data, width, height, false)
    }

    pub fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.backend.texture_size(handle)
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.backend.surface_size()
    }

    /// Load a font from bytes (TTF/OTF format).
    pub fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle> {
        self.backend.load_font_from_bytes(bytes)
    }

    /// Rasterize all glyphs needed for a text string.
    /// Call this before draw_text() to ensure glyphs are cached.
    pub fn rasterize_text_glyphs(&mut self, text: &str, font: FontHandle, size: f32) -> Result<()> {
        self.backend.ensure_glyphs_rasterized(text, font, size)
    }

    /// Draw text at the specified position in world coordinates.
    ///
    /// # Arguments
    /// * `frame` - The current frame being rendered
    /// * `text` - The text string to render
    /// * `font` - The font handle to use
    /// * `size` - Font size in pixels
    /// * `position` - World position (bottom-left of first character)
    /// * `color` - RGBA color tint
    /// * `camera` - Camera for view projection
    ///
    /// # Note
    /// All glyphs must be pre-rasterized using `rasterize_text_glyphs()` before calling this.
    pub fn draw_text(
        &mut self,
        frame: &mut Frame,
        text: &str,
        font: FontHandle,
        size: f32,
        position: Vec2,
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend
            .draw_text(frame, text, font, size, position, color, camera)
    }
}

pub struct Frame {
    surface_texture: Option<wgpu::SurfaceTexture>,
    view: TextureView,
    encoder: Option<CommandEncoder>,
    sprite_draws: Vec<SpriteDrawCommand>, // Queue of sprite draws for batching
}

impl Drop for Frame {
    fn drop(&mut self) {
        // If frame wasn't properly ended, we still need to present the surface texture
        // to avoid leaking resources. The encoder will be dropped automatically.
        if let Some(surface_texture) = self.surface_texture.take() {
            surface_texture.present();
        }
    }
}

struct TextureEntry {
    /// The underlying GPU texture. Must be kept alive for the view/sampler to be valid.
    texture: Texture,
    view: TextureView,
    sampler: Sampler,
    size: (u32, u32),
}

struct SpritePipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group_layout: BindGroupLayout,
    #[allow(dead_code)]
    uniform_buffer_size: u64,
    uniform_alignment: u64,
}

// Maximum number of sprites we can draw per frame (64KB buffer / 256 byte alignment = 256 sprites)
const MAX_SPRITES_PER_FRAME: usize = 256;
const UNIFORM_BUFFER_SIZE: u64 = MAX_SPRITES_PER_FRAME as u64 * 256; // 64KB

struct WgpuBackend {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    present_mode: PresentMode,
    sprite_pipeline: SpritePipeline,
    textures: HashMap<TextureHandle, TextureEntry>,
    next_texture_id: u32,
    uniform_write_offset: u64, // Current offset for writing uniforms
    bind_group_cache: HashMap<(TextureHandle, u64), wgpu::BindGroup>, // Cache bind groups per (texture, offset)
    text_renderer: TextRenderer,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteUniforms {
    mvp: [[f32; 4]; 4],
    color: [f32; 4],
}

const SPRITE_VERTICES: [SpriteVertex; 6] = [
    SpriteVertex {
        position: [-0.5, -0.5],
        uv: [0.0, 0.0], // Top-left (WebGPU convention)
    },
    SpriteVertex {
        position: [0.5, -0.5],
        uv: [1.0, 0.0], // Top-right
    },
    SpriteVertex {
        position: [0.5, 0.5],
        uv: [1.0, 1.0], // Bottom-right
    },
    SpriteVertex {
        position: [-0.5, -0.5],
        uv: [0.0, 0.0], // Top-left
    },
    SpriteVertex {
        position: [0.5, 0.5],
        uv: [1.0, 1.0], // Bottom-right
    },
    SpriteVertex {
        position: [-0.5, 0.5],
        uv: [0.0, 1.0], // Bottom-left
    },
];

impl WgpuBackend {
    fn new(window: &Window, vsync: bool) -> Result<Self> {
        let instance = Instance::default();
        let surface = unsafe { instance.create_surface(window)? };

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow!("No suitable GPU adapters found"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("forge2d-device"),
                features: Features::empty(),
                limits: Limits::default(),
            },
            None,
        ))?;

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        let present_mode = choose_present_mode(&capabilities.present_modes, vsync);
        let alpha_mode = choose_alpha_mode(&capabilities.alpha_modes);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        let sprite_pipeline = create_sprite_pipeline(&device, format);

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            present_mode,
            sprite_pipeline,
            textures: HashMap::new(),
            next_texture_id: 1,
            uniform_write_offset: 0,
            bind_group_cache: HashMap::new(),
            text_renderer: TextRenderer::new(),
        })
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface_config.present_mode = self.present_mode;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn begin_frame(&mut self) -> Result<Frame> {
        // Reset uniform buffer offset at the start of each frame
        self.uniform_write_offset = 0;
        // Clear bind group cache each frame (they're frame-specific)
        self.bind_group_cache.clear();

        loop {
            match self.surface.get_current_texture() {
                Ok(surface_texture) => {
                    let view = surface_texture
                        .texture
                        .create_view(&TextureViewDescriptor::default());
                    let encoder = self
                        .device
                        .create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("frame-encoder"),
                        });

                    return Ok(Frame {
                        surface_texture: Some(surface_texture),
                        view,
                        encoder: Some(encoder),
                        sprite_draws: Vec::new(),
                    });
                }
                Err(SurfaceError::Lost) | Err(SurfaceError::Outdated) => {
                    self.surface.configure(&self.device, &self.surface_config);
                }
                Err(SurfaceError::Timeout) => {
                    continue;
                }
                Err(SurfaceError::OutOfMemory) => return Err(anyhow!("Surface ran out of memory")),
            }
        }
    }

    fn clear(&mut self, frame: &mut Frame, color: [f32; 4]) -> Result<()> {
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: color[0] as f64,
                            g: color[1] as f64,
                            b: color[2] as f64,
                            a: color[3] as f64,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            drop(_pass);
        }

        Ok(())
    }

    fn draw_sprite(&mut self, frame: &mut Frame, sprite: &Sprite, camera: &Camera2D) -> Result<()> {
        let texture = self
            .textures
            .get(&sprite.texture)
            .ok_or_else(|| anyhow!("Unknown texture handle"))?;

        // Check if we've exceeded the maximum sprites per frame
        if self.uniform_write_offset >= UNIFORM_BUFFER_SIZE {
            return Err(anyhow!(
                "Too many sprites drawn in one frame (max: {})",
                MAX_SPRITES_PER_FRAME
            ));
        }

        let base_size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);
        let model = sprite.transform.to_matrix(base_size);
        let vp = camera.view_projection(self.surface_config.width, self.surface_config.height);
        let mvp = vp * model;

        let uniforms = SpriteUniforms {
            mvp: mvp.to_cols_array_2d(),
            color: sprite.tint,
        };

        // Write uniforms at the current offset (aligned to required alignment)
        let aligned_offset = if self.uniform_write_offset == 0 {
            0
        } else {
            (self.uniform_write_offset + self.sprite_pipeline.uniform_alignment - 1)
                & !(self.sprite_pipeline.uniform_alignment - 1)
        };

        self.queue.write_buffer(
            &self.sprite_pipeline.uniform_buffer,
            aligned_offset,
            bytemuck::bytes_of(&uniforms),
        );

        // Get or create bind group for this texture (cache per texture)
        // We ensure it exists here, then look it up again when flushing
        let cache_key = (sprite.texture, 0);
        let uniform_size = std::mem::size_of::<SpriteUniforms>() as u64;
        let _bind_group = self.bind_group_cache.entry(cache_key).or_insert_with(|| {
            self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("sprite-bind-group"),
                layout: &self.sprite_pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.sprite_pipeline.uniform_buffer,
                            offset: 0,
                            size: std::num::NonZeroU64::new(uniform_size),
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&texture.view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&texture.sampler),
                    },
                ],
            })
        });

        // Queue the sprite draw instead of executing immediately
        frame.sprite_draws.push(SpriteDrawCommand {
            uniform_offset: aligned_offset,
            texture_handle: sprite.texture,
        });

        // Advance offset for next sprite
        self.uniform_write_offset = aligned_offset + self.sprite_pipeline.uniform_alignment;

        Ok(())
    }

    /// Flush all queued sprite draws in a single render pass (called by end_frame)
    fn flush_sprites(&mut self, frame: &mut Frame) -> Result<()> {
        if frame.sprite_draws.is_empty() {
            return Ok(());
        }

        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        // Create ONE render pass for all sprites
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sprite-batch-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // Load previous content (from clear)
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        pass.set_pipeline(&self.sprite_pipeline.pipeline);
        pass.set_vertex_buffer(0, self.sprite_pipeline.vertex_buffer.slice(..));

        // Draw all queued sprites in the same pass
        for draw_cmd in &frame.sprite_draws {
            // Look up bind group for this texture (should be cached)
            let cache_key = (draw_cmd.texture_handle, 0);
            if let Some(bind_group) = self.bind_group_cache.get(&cache_key) {
                pass.set_bind_group(0, bind_group, &[draw_cmd.uniform_offset as u32]);
                pass.draw(0..SPRITE_VERTICES.len() as u32, 0..1);
            } else {
                return Err(anyhow!("Bind group not found for texture handle"));
            }
        }

        // Pass is dropped here, commands are recorded in encoder
        Ok(())
    }

    fn end_frame(&mut self, mut frame: Frame) -> Result<()> {
        // Flush all queued sprite draws in a single render pass
        self.flush_sprites(&mut frame)?;

        let encoder = frame
            .encoder
            .take()
            .ok_or_else(|| anyhow!("Frame already ended"))?;
        self.queue.submit(Some(encoder.finish()));

        let surface_texture = frame
            .surface_texture
            .take()
            .ok_or_else(|| anyhow!("Frame already ended"))?;
        surface_texture.present();
        Ok(())
    }

    fn load_texture_from_file(&mut self, path: &str) -> Result<TextureHandle> {
        let data = fs::read(path)?;
        self.load_texture_from_bytes(&data)
    }

    fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle> {
        let image = image::load_from_memory(bytes)?.to_rgba8();
        let dimensions = image.dimensions();
        // Regular image textures use linear filtering
        self.load_texture_from_rgba(&image, dimensions.0, dimensions.1, false)
    }

    /// Load a texture from raw RGBA8 data (for glyphs, etc.)
    /// `is_font_texture`: if true, uses Nearest filtering for crisp text rendering
    pub(crate) fn load_texture_from_rgba(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        is_font_texture: bool,
    ) -> Result<TextureHandle> {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            data,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());

        // Filtering mode selection:
        // - Font textures: Nearest for crisp, pixel-perfect rendering
        // - Regular sprites: Linear for smooth scaling
        let (mag_filter, min_filter) = if is_font_texture {
            (FilterMode::Nearest, FilterMode::Nearest)
        } else {
            (FilterMode::Linear, FilterMode::Linear)
        };

        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some(if is_font_texture {
                "font-sampler"
            } else {
                "sprite-sampler"
            }),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter,
            min_filter,
            mipmap_filter: FilterMode::Nearest, // No mipmaps for fonts
            ..Default::default()
        });

        let handle = TextureHandle(self.next_texture_id);
        self.next_texture_id += 1;
        self.textures.insert(
            handle,
            TextureEntry {
                texture,
                view,
                sampler,
                size: (width, height),
            },
        );

        Ok(handle)
    }

    fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.textures.get(&handle).map(|t| t.size)
    }

    fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle> {
        self.text_renderer.load_font_from_bytes(bytes)
    }

    /// Ensure all characters in the text are rasterized and cached.
    /// Call this before draw_text() to pre-rasterize glyphs.
    fn ensure_glyphs_rasterized(&mut self, text: &str, font: FontHandle, size: f32) -> Result<()> {
        // Collect characters that need rasterization
        let mut to_rasterize: Vec<char> = text
            .chars()
            .filter(|&ch| !self.text_renderer.has_glyph(font, ch, size))
            .collect();

        // Remove duplicates
        to_rasterize.sort();
        to_rasterize.dedup();

        // Get font reference first (immutable borrow)
        let font_ref = self
            .text_renderer
            .get_font(font)
            .ok_or_else(|| anyhow!("Font not found"))?;

        // Rasterize each glyph and collect image data
        // Store all data we need so we can release the font_ref borrow
        let mut glyph_data: Vec<(char, Vec<u8>, u32, u32, f32, f32, f32, f32, f32)> = Vec::new();

        for ch in to_rasterize {
            let scale = ab_glyph::PxScale::from(size);
            let scaled_font = font_ref.as_scaled(scale);
            let glyph_id = font_ref.glyph_id(ch);
            let glyph = Glyph {
                id: glyph_id,
                scale,
                position: ab_glyph::point(0.0, 0.0),
            };

            // Rasterize the glyph - use the trait method (available via ScaleFont import)
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
                            image_data[idx] = 255;
                            image_data[idx + 1] = 255;
                            image_data[idx + 2] = 255;
                            image_data[idx + 3] = alpha;
                        }
                    });

                    // Get advance width from the font (distance to next character origin)
                    // This is the proper spacing between characters as defined by the font
                    let mut advance = scaled_font.h_advance(glyph_id);

                    // Calculate proper bearing from glyph bounds
                    // bearing_x: horizontal offset from origin to left edge of glyph
                    // bounds.min.x is the left edge of the glyph's bounding box relative to origin
                    // This can be negative for characters that extend left (like italic 'f')
                    let bearing_x = bounds.min.x;

                    // bearing_y: vertical offset from baseline to top of glyph
                    // bounds.min.y is typically negative (above baseline), so we negate it
                    // to get a positive offset downward from the baseline for screen coordinates
                    let bearing_y = -bounds.min.y;

                    // Robustness: Ensure advance is reasonable to prevent character overlap
                    // Some fonts might have very small or zero advances, which causes overlap
                    let glyph_width = bounds.width();
                    if advance <= 0.0 || advance < glyph_width * 0.5 {
                        // Fallback: use glyph width + small padding if advance is too small
                        advance = glyph_width.max(1.0) + 2.0; // Small padding for safety
                    }

                    glyph_data.push((
                        ch,
                        image_data,
                        width,
                        height,
                        bearing_x,
                        bearing_y,
                        width as f32,
                        height as f32,
                        advance,
                    ));
                }
            }
        }

        // font_ref borrow ends here, now we can mutably borrow self

        // Now load textures and cache glyphs (mutable borrow of self, no conflict)
        for (ch, image_data, width, height, bearing_x, bearing_y, width_f, height_f, advance) in
            glyph_data
        {
            // Font textures use Nearest filtering for crisp rendering
            let texture = self.load_texture_from_rgba(&image_data, width, height, true)?;

            // Cache the glyph
            self.text_renderer.cache_glyph(
                font,
                ch,
                size,
                GlyphCacheEntry {
                    texture,
                    width: width_f,
                    height: height_f,
                    bearing_x,
                    bearing_y,
                    advance,
                },
            );
        }

        Ok(())
    }

    fn draw_text(
        &mut self,
        frame: &mut Frame,
        text: &str,
        font: FontHandle,
        size: f32,
        position: Vec2,
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        let font_ref = self
            .text_renderer
            .get_font(font)
            .ok_or_else(|| anyhow!("Font not found"))?;
        let scaled_font = font_ref.as_scaled(ab_glyph::PxScale::from(size));

        // Ensure all glyphs for this text are rasterized and cached before drawing.
        // This makes the API easier to use: callers don't need to remember to
        // call `rasterize_text_glyphs`/`ensure_glyphs_rasterized` separately.
        self.ensure_glyphs_rasterized(text, font, size)?;

        // Start with the exact position - Nearest filtering will handle crisp rendering
        let mut x = position.x;
        let y = position.y;

        // Track previous glyph for kerning adjustments
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

        // Now draw all glyphs
        for ch in text.chars() {
            let glyph_id = font_ref.glyph_id(ch);

            // Apply kerning between the previous glyph and this one
            if let Some(prev) = prev_glyph {
                x += scaled_font.kern(prev, glyph_id);
            }

            // Get glyph data (immutable borrow)
            let (texture_handle, width, height, bearing_x, bearing_y, advance) = {
                // If for some reason the glyph is still missing from the cache,
                // skip drawing this character instead of erroring every frame.
                let Some(glyph) = self.text_renderer.get_glyph(font, ch, size) else {
                    continue;
                };

                // Get texture size for proper scaling
                let texture_size = self.texture_size(glyph.texture).unwrap_or((32, 32)); // Fallback if not found

                (
                    glyph.texture,
                    glyph.width / texture_size.0 as f32,
                    glyph.height / texture_size.1 as f32,
                    glyph.bearing_x,
                    glyph.bearing_y,
                    glyph.advance,
                )
            };

            // Create sprite for this glyph (now we can mutably borrow self)
            let mut sprite = Sprite::new(texture_handle);

            // Position glyph relative to baseline origin
            // bearing_x offsets the sprite left/right from the origin
            // bearing_y offsets the sprite up/down from the baseline (inverted for screen coords)
            // Nearest filtering ensures crisp rendering without needing integer snapping
            sprite.transform.position = Vec2::new(x + bearing_x, y - bearing_y);
            sprite.transform.scale = Vec2::new(width, height);
            sprite.tint = color;

            // Draw the glyph sprite
            self.draw_sprite(frame, &sprite, camera)?;

            // Advance to next character's origin position
            // The advance value from the font already accounts for proper spacing
            x += advance;

            prev_glyph = Some(glyph_id);
        }

        Ok(())
    }
}

fn create_sprite_pipeline(device: &wgpu::Device, surface_format: TextureFormat) -> SpritePipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("sprite-shader"),
        source: ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("sprite-bind-group-layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true, // Enable dynamic offsets
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<SpriteUniforms>() as u64,
                    ),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("sprite-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite-vertices"),
        contents: bytemuck::cast_slice(&SPRITE_VERTICES),
        usage: BufferUsages::VERTEX,
    });

    // Get the required uniform buffer alignment (usually 256 bytes)
    let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
    let uniform_size = std::mem::size_of::<SpriteUniforms>() as u64;
    // Round up to alignment (not used directly, but kept for reference)
    let _aligned_uniform_size = (uniform_size + uniform_alignment - 1) & !(uniform_alignment - 1);

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sprite-uniform-buffer"),
        size: UNIFORM_BUFFER_SIZE,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("sprite-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            }],
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    });

    SpritePipeline {
        pipeline,
        vertex_buffer,
        uniform_buffer,
        bind_group_layout,
        uniform_buffer_size: UNIFORM_BUFFER_SIZE,
        uniform_alignment,
    }
}

fn choose_present_mode(modes: &[PresentMode], vsync: bool) -> PresentMode {
    if vsync {
        modes
            .iter()
            .copied()
            .find(|mode| matches!(mode, PresentMode::Fifo | PresentMode::FifoRelaxed))
            .unwrap_or(PresentMode::Fifo)
    } else {
        modes
            .iter()
            .copied()
            .find(|mode| matches!(mode, PresentMode::Immediate | PresentMode::Mailbox))
            .unwrap_or(PresentMode::Immediate)
    }
}

fn choose_alpha_mode(modes: &[CompositeAlphaMode]) -> CompositeAlphaMode {
    modes
        .iter()
        .copied()
        .find(|mode| matches!(mode, CompositeAlphaMode::Auto))
        .unwrap_or_else(|| modes.first().copied().unwrap_or(CompositeAlphaMode::Opaque))
}
