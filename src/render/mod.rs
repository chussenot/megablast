//! wgpu pipeline setup: instanced quads, one pipeline, sRGB, <= 3 draw
//! calls per frame (background, entities, text). Every wgpu/winit type
//! in the crate lives under `src/render/`.
//!
//! Owner: Wave 1 `render-pipeline` task (this file + `background.rs` +
//! `sprites.rs`). `main.rs` already calls `Renderer::new` / `resize` /
//! `render` and builds `RenderState` via `From<&Game>` -- keep these
//! signatures; everything inside their bodies is yours to design.

mod background;
mod hud;
mod sprites;

use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::events::GameEvent;
use crate::game::Game;

/// Snapshot of drawable state as of a tick boundary, so `Renderer::render`
/// can interpolate between this and the live `Game` using `alpha` (same
/// pattern as arkanoid's `RenderState`).
///
/// TODO(wave1 `render-pipeline`): capture whatever `sprites` /
/// `background` / `hud` need to draw (player/enemy/shot/pickup
/// positions, scroll_y, HUD numbers).
pub struct RenderState;

impl From<&Game> for RenderState {
    fn from(_game: &Game) -> Self {
        RenderState
    }
}

pub struct Renderer {
    window: Arc<Window>,
}

impl Renderer {
    /// TODO(wave1 `render-pipeline`): real wgpu init (instance / adapter /
    /// device / queue / surface / pipeline), glyphon text renderer,
    /// starfield + terrain buffers.
    pub fn new(window: Arc<Window>) -> Self {
        Self { window }
    }

    /// TODO(wave1 `render-pipeline`): reconfigure the surface.
    pub fn resize(&mut self, _new_size: PhysicalSize<u32>) {
        let _ = &self.window;
    }

    /// TODO(wave1 `render-pipeline`): draw background (`background`
    /// module), entities (`sprites` module) interpolated between
    /// `prev`/`current`/`alpha`, and HUD text (`hud` module -- real
    /// content lands Wave 3+). Wave 5 `juice-fx` reads `frame_events` for
    /// muzzle flash / death quads / screen shake / HP-bar flash -- keep
    /// this trailing parameter even if you don't consume it yet.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        _clear_color: wgpu::Color,
        _prev: &RenderState,
        _current: &Game,
        _alpha: f32,
        _frame_events: &[GameEvent],
    ) {
    }
}
