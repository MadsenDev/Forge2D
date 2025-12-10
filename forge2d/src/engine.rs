use std::time::{Duration, Instant};

use anyhow::Result;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

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
        let mut event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
            .build(&event_loop)?;

        let mut ctx = EngineContext::new(window);
        game.init(&mut ctx)?;

        let mut last_frame = Instant::now();
        let mut pending_error: Option<anyhow::Error> = None;

        event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if is_escape_pressed(&input) {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    _ => {}
                },
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
}

impl EngineContext {
    fn new(window: winit::window::Window) -> Self {
        Self {
            window,
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            exit_requested: false,
        }
    }

    fn update_time(&mut self, delta: Duration) {
        self.delta_time = delta;
        self.elapsed_time += delta;
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

    /// Request that the engine exit after the current frame.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
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
