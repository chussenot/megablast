//! Window/event loop wiring only (cloned from arkanoid's `main.rs`; see
//! docs/megablast.md Architecture). Owner: bootstrap -- fixed for the
//! whole build; no wave task edits this file.

mod events;
mod game;
mod levels;
mod render;

use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use game::{Game, Input};
use render::{RenderState, Renderer};

const LOGICAL_WIDTH: u32 = 600;
const LOGICAL_HEIGHT: u32 = 800;

/// Fixed simulation tick rate, independent of the display's refresh rate.
const FIXED_DT: f32 = 1.0 / 120.0;

/// Cap on ticks run per frame -- avoids the "spiral of death" after a
/// long stall (window drag, debugger pause) by dropping the extra time
/// instead of trying to catch up indefinitely.
const MAX_TICKS_PER_FRAME: u32 = 10;

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.02,
    g: 0.02,
    b: 0.05,
    a: 1.0,
};

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: Game,
    input: Input,
    accumulator: f32,
    last_update: Instant,
    render_prev: RenderState,
    render_alpha: f32,
    /// Events from every tick run this frame, for the renderer's juice
    /// (Wave 5) -- cleared after each `RedrawRequested`, unlike arkanoid
    /// (which has no visual event consumer yet).
    frame_events: Vec<events::GameEvent>,
}

impl Default for App {
    fn default() -> Self {
        let game = Game::new();
        Self {
            window: None,
            renderer: None,
            render_prev: RenderState::from(&game),
            game,
            input: Input::default(),
            accumulator: 0.0,
            last_update: Instant::now(),
            render_alpha: 0.0,
            frame_events: Vec::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // resumed can fire more than once (e.g. suspend/resume on some
        // platforms); only create the window/renderer the first time.
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("megablast")
            .with_inner_size(LogicalSize::new(LOGICAL_WIDTH, LOGICAL_HEIGHT))
            .with_resizable(true);
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("failed to create window"),
        );

        self.renderer = Some(Renderer::new(Arc::clone(&window)));
        self.window = Some(window);
        self.last_update = Instant::now();

        // Poll (not Wait): the loop must keep ticking/rendering on its
        // own schedule rather than only reacting to OS input events.
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                is_synthetic: false,
                ..
            } => self.set_input_key(physical_key, state),
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.render(
                        CLEAR_COLOR,
                        &self.render_prev,
                        &self.game,
                        self.render_alpha,
                        &self.frame_events,
                    );
                }
                self.frame_events.clear();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };

        let now = Instant::now();
        self.accumulator += now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        // Anchor for this frame's interpolation: state as of before any
        // of this frame's ticks run.
        self.render_prev = RenderState::from(&self.game);

        let mut ticks = 0;
        while self.accumulator >= FIXED_DT && ticks < MAX_TICKS_PER_FRAME {
            self.game.tick(&self.input, FIXED_DT);
            // Accumulate rather than clear-and-drop: Wave 5 `juice-fx`
            // reads `frame_events` at render time (see the doc comment
            // on this struct's field).
            self.frame_events.append(&mut self.game.events);
            self.accumulator -= FIXED_DT;
            ticks += 1;
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = 0.0;
        }
        self.render_alpha = (self.accumulator / FIXED_DT).clamp(0.0, 1.0);

        // One render per redraw regardless of how many ticks ran above --
        // this is what decouples the 120 Hz sim from the display's vsync.
        window.request_redraw();
    }
}

impl App {
    /// 8-directional movement (Arrow keys + WASD) + Space to fire + P/Esc
    /// to pause, matched on `PhysicalKey` (key position) rather than the
    /// logical `Key` so the bindings keep working on non-QWERTY layouts.
    fn set_input_key(&mut self, physical_key: PhysicalKey, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        let PhysicalKey::Code(code) = physical_key else {
            return;
        };
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => self.input.up = pressed,
            KeyCode::ArrowDown | KeyCode::KeyS => self.input.down = pressed,
            KeyCode::ArrowLeft | KeyCode::KeyA => self.input.left = pressed,
            KeyCode::ArrowRight | KeyCode::KeyD => self.input.right = pressed,
            KeyCode::Space => self.input.fire = pressed,
            KeyCode::KeyP | KeyCode::Escape => self.input.pause = pressed,
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .expect("event loop terminated with an error");
}
