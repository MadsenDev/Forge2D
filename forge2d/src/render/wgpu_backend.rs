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
    ShaderSource, SurfaceConfiguration, SurfaceError, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    math::{Camera2D, Vec2},
    render::sprite::{Sprite, TextureHandle},
};

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

    pub fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.backend.texture_size(handle)
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.backend.surface_size()
    }
}

pub struct Frame {
    surface_texture: Option<wgpu::SurfaceTexture>,
    view: TextureView,
    encoder: Option<CommandEncoder>,
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
    view: TextureView,
    sampler: Sampler,
    size: (u32, u32),
}

struct SpritePipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group_layout: BindGroupLayout,
}

struct WgpuBackend {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    present_mode: PresentMode,
    sprite_pipeline: SpritePipeline,
    textures: HashMap<TextureHandle, TextureEntry>,
    next_texture_id: u32,
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
        uv: [0.0, 1.0],
    },
    SpriteVertex {
        position: [0.5, -0.5],
        uv: [1.0, 1.0],
    },
    SpriteVertex {
        position: [0.5, 0.5],
        uv: [1.0, 0.0],
    },
    SpriteVertex {
        position: [-0.5, -0.5],
        uv: [0.0, 1.0],
    },
    SpriteVertex {
        position: [0.5, 0.5],
        uv: [1.0, 0.0],
    },
    SpriteVertex {
        position: [-0.5, 0.5],
        uv: [0.0, 0.0],
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

        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let base_size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);
        let model = sprite.transform.to_matrix(base_size);
        let vp = camera.view_projection(self.surface_config.width, self.surface_config.height);
        let mvp = vp * model;

        let uniforms = SpriteUniforms {
            mvp: mvp.to_cols_array_2d(),
            color: sprite.tint,
        };
        self.queue.write_buffer(
            &self.sprite_pipeline.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("sprite-bind-group"),
            layout: &self.sprite_pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.sprite_pipeline.uniform_buffer.as_entire_binding(),
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
        });

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sprite-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        pass.set_pipeline(&self.sprite_pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.sprite_pipeline.vertex_buffer.slice(..));
        pass.draw(0..SPRITE_VERTICES.len() as u32, 0..1);

        Ok(())
    }

    fn end_frame(&mut self, mut frame: Frame) -> Result<()> {
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
        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("sprite-texture"),
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
            &image,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some("sprite-sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let handle = TextureHandle(self.next_texture_id);
        self.next_texture_id += 1;
        self.textures.insert(
            handle,
            TextureEntry {
                view,
                sampler,
                size: dimensions,
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
                    has_dynamic_offset: false,
                    min_binding_size: None,
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

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sprite-uniform-buffer"),
        size: std::mem::size_of::<SpriteUniforms>() as u64,
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
