use anyhow::{anyhow, Result};
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Features, Instance, Limits,
    LoadOp, Operations, PresentMode, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, SurfaceConfiguration, SurfaceError, TextureUsages,
};
use winit::{dpi::PhysicalSize, window::Window};

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

    pub fn clear(&mut self, frame: &Frame, color: [f32; 4]) -> Result<()> {
        self.backend.clear(frame, color)
    }

    pub fn end_frame(&mut self, frame: Frame) -> Result<()> {
        self.backend.end_frame(frame)
    }
}

pub struct Frame {
    surface_texture: Option<wgpu::SurfaceTexture>,
}

struct WgpuBackend {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    present_mode: PresentMode,
}

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

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            present_mode,
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
                    return Ok(Frame {
                        surface_texture: Some(surface_texture),
                    })
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

    fn clear(&mut self, frame: &Frame, color: [f32; 4]) -> Result<()> {
        let surface_texture = frame
            .surface_texture
            .as_ref()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("clear"),
            });

        {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
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

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    fn end_frame(&mut self, mut frame: Frame) -> Result<()> {
        let surface_texture = frame
            .surface_texture
            .take()
            .ok_or_else(|| anyhow!("Frame already ended"))?;
        surface_texture.present();
        Ok(())
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
