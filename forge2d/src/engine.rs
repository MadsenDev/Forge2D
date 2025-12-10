use std::time::{Duration, Instant};

use anyhow::Result;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

use crate::{input::InputState, render::Renderer};

/// Configuration values for the engine window and runtime behavior.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Forge2D Game".into(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

/// Main entrypoint for running a Forge2D game.
pub struct Engine {
    config: EngineConfig,
}

impl Engine {
    /// Create a new engine instance with default configuration.
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
        }
    }

    /// Override the window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Override the initial window size in logical pixels.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Enable or disable vertical sync.
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.config.vsync = vsync;
        self
    }

    /// Run the provided game until the window is closed or the game requests exit.
    pub fn run<G: Game + 'static>(self, mut game: G) -> Result<()> {
        let config = self.config;

        let mut event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(config.title.clone())
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .build(&event_loop)?;

        let mut ctx = EngineContext::new(window, &config)?;
        game.init(&mut ctx)?;

        let mut last_frame = Instant::now();
        let mut pending_error: Option<anyhow::Error> = None;

        event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::NewEvents(_) => {
                    ctx.begin_frame();
                }
                Event::WindowEvent { event, .. } => {
                    ctx.handle_window_event(&event);

                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::KeyboardInput { ref input, .. } => {
                            if is_escape_pressed(input) {
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                        WindowEvent::Resized(new_size) => {
                            ctx.resize_renderer(new_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            ctx.resize_renderer(*new_inner_size);
                        }
                        _ => {}
                    }
                }
                Event::MainEventsCleared => {
                    let now = Instant::now();
                    ctx.update_time(now - last_frame);
                    last_frame = now;

                    if let Err(err) = game.update(&mut ctx) {
                        pending_error = Some(err);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    if ctx.exit_requested {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    ctx.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    if let Err(err) = game.draw(&mut ctx) {
                        pending_error = Some(err);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    if ctx.exit_requested {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            }
        });

        if let Some(err) = pending_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}

fn is_escape_pressed(input: &KeyboardInput) -> bool {
    input.state == ElementState::Pressed
        && matches!(
            input.virtual_keycode,
            Some(winit::event::VirtualKeyCode::Escape)
        )
}

/// Shared context provided to game code each frame.
pub struct EngineContext {
    window: winit::window::Window,
    delta_time: Duration,
    elapsed_time: Duration,
    exit_requested: bool,
    input: InputState,
    renderer: Renderer,
}

impl EngineContext {
    fn new(window: winit::window::Window, config: &EngineConfig) -> Result<Self> {
        let renderer = Renderer::new(&window, config.vsync)?;

        Ok(Self {
            window,
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            exit_requested: false,
            input: InputState::new(),
            renderer,
        })
    }

    fn begin_frame(&mut self) {
        self.input.begin_frame();
    }

    fn update_time(&mut self, delta: Duration) {
        self.delta_time = delta;
        self.elapsed_time += delta;
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { input, .. } => self.input.handle_key(*input),
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.handle_mouse_button(*button, *state)
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.handle_cursor_moved(position.x, position.y)
            }
            _ => {}
        }
    }

    fn resize_renderer(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    /// Duration between the current and previous frames.
    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    /// Total time elapsed since the engine started running.
    pub fn elapsed_time(&self) -> Duration {
        self.elapsed_time
    }

    /// Access the underlying winit window.
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    /// Access the current input state.
    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Request that the engine exit after the current frame.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Access the renderer for drawing operations.
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }
}

/// Trait implemented by user code to hook into the engine lifecycle.
pub trait Game {
    /// Called once after the window is created but before the first frame.
    fn init(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    /// Update game state. Called once per frame before drawing.
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()>;

    /// Draw the current frame. Called after update when a redraw is requested.
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()>;
}
