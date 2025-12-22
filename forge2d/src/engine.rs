use std::time::{Duration, Instant};

use anyhow::Result;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{assets::AssetManager, audio::AudioSystem, input::InputState, render::Renderer};

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
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Override the initial window size in logical pixels.
    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Enable or disable vertical sync.
    #[must_use]
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.config.vsync = vsync;
        self
    }

    /// Run the provided game until the window is closed or the game requests exit.
    pub fn run<G: Game + 'static>(self, mut game: G) -> Result<()> {
        let config = self.config;

        let event_loop = EventLoop::new()?;
        let mut window_attributes = Window::default_attributes();
        window_attributes.title = config.title.clone();
        window_attributes.inner_size = Some(LogicalSize::new(config.width, config.height).into());
        let window = event_loop.create_window(window_attributes)?;

        // Leak the window to get a 'static reference
        // This is safe because the window lives for the entire program duration
        let window: &'static Window = Box::leak(Box::new(window));

        let mut ctx = EngineContext::new(window, &config)?;
        game.init(&mut ctx)?;

        let mut last_frame = Instant::now();
        event_loop.run(move |event, elwt| {
            match event {
                Event::NewEvents(_) => {
                    ctx.begin_frame();
                }
                Event::WindowEvent { event, .. } => {
                    ctx.handle_window_event(&event);

                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if is_escape_pressed(&event) {
                                elwt.exit();
                            }
                        }
                        WindowEvent::Resized(new_size) => {
                            ctx.resize_renderer(new_size);
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            // Note: The actual resize will come through Resized event
                        }
                        WindowEvent::RedrawRequested => {
                            if let Err(err) = game.draw(&mut ctx) {
                                eprintln!("Encountered error during draw: {err:?}");
                                elwt.exit();
                                return;
                            }

                            if ctx.exit_requested {
                                elwt.exit();
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    let now = Instant::now();
                    ctx.update_time(now - last_frame);
                    last_frame = now;

                    if let Err(err) = game.update(&mut ctx) {
                        eprintln!("Encountered error during update: {err:?}");
                        elwt.exit();
                        return;
                    }

                    if ctx.exit_requested {
                        elwt.exit();
                        return;
                    }

                    ctx.window.request_redraw();
                }
                _ => {}
            }
        })?;

        Ok(())
    }
}

fn is_escape_pressed(event: &KeyEvent) -> bool {
    event.state == ElementState::Pressed
        && matches!(
            event.physical_key,
            PhysicalKey::Code(KeyCode::Escape)
        )
}

/// Shared context provided to game code each frame.
pub struct EngineContext<'window> {
    window: &'window winit::window::Window,
    delta_time: Duration,
    elapsed_time: Duration,
    fixed_delta_time: Duration,
    fixed_time_accumulator: Duration,
    exit_requested: bool,
    input: InputState,
    renderer: Renderer<'window>,
    assets: AssetManager,
    audio: AudioSystem,
}

impl<'window> EngineContext<'window> {
    fn new(window: &'window winit::window::Window, config: &EngineConfig) -> Result<Self> {
        let renderer = Renderer::new(window, config.vsync)?;
        // Audio initialization is graceful - engine continues even if audio fails
        let audio = AudioSystem::new()?;

        Ok(Self {
            window,
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            fixed_delta_time: Duration::from_secs_f64(1.0 / 60.0), // 60 FPS fixed timestep
            fixed_time_accumulator: Duration::ZERO,
            exit_requested: false,
            input: InputState::new(),
            renderer,
            assets: AssetManager::new(),
            audio,
        })
    }

    fn begin_frame(&mut self) {
        self.input.begin_frame();
    }

    fn update_time(&mut self, delta: Duration) {
        self.delta_time = delta;
        self.elapsed_time += delta;
        // Accumulate time for fixed timestep
        self.fixed_time_accumulator += delta;
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => self.input.handle_key(event),
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

    /// Fixed timestep duration (typically 1/60 second for 60 FPS).
    pub fn fixed_delta_time(&self) -> Duration {
        self.fixed_delta_time
    }

    /// Check if a fixed timestep update should run and consume accumulated time.
    ///
    /// Returns `true` if enough time has accumulated for a fixed update.
    /// Call this in a loop until it returns `false` to handle multiple fixed updates per frame.
    ///
    /// Example:
    /// ```rust,no_run
    /// # use forge2d::EngineContext;
    /// # fn example(mut ctx: &mut EngineContext) {
    /// while ctx.should_run_fixed_update() {
    ///     // Run physics, collision, etc. with fixed timestep
    ///     let fixed_dt = ctx.fixed_delta_time();
    ///     // physics_system.update(fixed_dt);
    /// }
    /// # }
    /// ```
    pub fn should_run_fixed_update(&mut self) -> bool {
        if self.fixed_time_accumulator >= self.fixed_delta_time {
            self.fixed_time_accumulator -= self.fixed_delta_time;
            true
        } else {
            false
        }
    }

    /// Get the interpolation factor for rendering between fixed timestep updates.
    ///
    /// Returns a value between 0.0 and 1.0 indicating how far through the current
    /// fixed timestep interval we are. Useful for smooth interpolation in rendering.
    pub fn fixed_update_alpha(&self) -> f32 {
        if self.fixed_delta_time.as_secs_f32() > 0.0 {
            (self.fixed_time_accumulator.as_secs_f32() / self.fixed_delta_time.as_secs_f32()).min(1.0)
        } else {
            0.0
        }
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
    pub fn renderer(&mut self) -> &mut Renderer<'window> {
        &mut self.renderer
    }

    /// Access the asset manager for loading and caching assets.
    pub fn assets(&mut self) -> &mut AssetManager {
        &mut self.assets
    }

    /// Load a texture using the asset manager (convenience method).
    ///
    /// This is equivalent to `ctx.assets().load_texture(ctx.renderer(), path)`
    /// but avoids borrowing issues.
    pub fn load_texture(&mut self, path: &str) -> Result<crate::render::TextureHandle> {
        self.assets.load_texture(&mut self.renderer, path)
    }

    /// Load a texture from bytes using the asset manager (convenience method).
    pub fn load_texture_from_bytes(
        &mut self,
        key: &str,
        bytes: &[u8],
    ) -> Result<crate::render::TextureHandle> {
        self.assets
            .load_texture_from_bytes(&mut self.renderer, key, bytes)
    }

    /// Load a font from bytes using the asset manager (convenience method).
    ///
    /// Fonts are cached by the provided key. Loading the same key again
    /// returns the cached `FontHandle` without re-loading the font data.
    pub fn load_font_from_bytes(
        &mut self,
        key: &str,
        bytes: &[u8],
    ) -> Result<crate::render::FontHandle> {
        self.assets
            .load_font_from_bytes(&mut self.renderer, key, bytes)
    }

    /// Get a cached font handle by key, if it exists.
    pub fn get_font(&self, key: &str) -> Option<crate::render::FontHandle> {
        self.assets.get_font(key)
    }

    /// Load a built-in engine font via the asset system.
    ///
    /// This uses the `BuiltinFont` enum and `AssetManager` under the hood.
    /// Until you configure actual font files in `forge2d::fonts`, this will
    /// return an error which you can gracefully ignore.
    pub fn builtin_font(
        &mut self,
        which: crate::fonts::BuiltinFont,
    ) -> Result<crate::render::FontHandle> {
        which.load(&mut self.assets, &mut self.renderer)
    }

    /// Get mouse position in world coordinates using the current camera.
    ///
    /// This converts screen-space mouse coordinates to world-space coordinates
    /// using the provided camera's view projection.
    pub fn mouse_world(&self, camera: &crate::math::Camera2D) -> crate::math::Vec2 {
        let mouse_screen = self.input.mouse_position_vec2();
        let (screen_w, screen_h) = self.renderer.surface_size();
        camera.screen_to_world(mouse_screen, screen_w, screen_h)
    }

    /// Access the audio system for playing sounds and music.
    pub fn audio(&mut self) -> &mut AudioSystem {
        &mut self.audio
    }
}

/// Trait implemented by user code to hook into the engine lifecycle.
pub trait Game {
    /// Called once after the window is created but before the first frame.
    fn init(&mut self, _ctx: &mut EngineContext<'_>) -> Result<()> {
        Ok(())
    }

    /// Update game state. Called once per frame before drawing.
    fn update(&mut self, ctx: &mut EngineContext<'_>) -> Result<()>;

    /// Draw the current frame. Called after update when a redraw is requested.
    fn draw(&mut self, ctx: &mut EngineContext<'_>) -> Result<()>;
}

/// Adapter to use StateMachine as a Game.
/// This allows StateMachine to be used directly with Engine::run().
impl Game for crate::state::StateMachine {
    fn init(&mut self, ctx: &mut EngineContext<'_>) -> Result<()> {
        // Call on_enter for the initial state (if any)
        self.init_top_state(ctx)?;
        // Apply any initial state transitions
        self.apply_transitions(ctx)?;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext<'_>) -> Result<()> {
        // Apply pending state transitions first
        self.apply_transitions(ctx)?;

        // Update the top state (if any)
        // This method handles borrow checker issues internally
        self.update_top(ctx)?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext<'_>) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        
        // Draw all states from bottom to top (oldest to newest)
        // This allows background states to be visible behind foreground states
        self.draw_all(renderer, &mut frame)?;
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}
