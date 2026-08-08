//! HUD: HP bar (bottom-left), cash (bottom-right), score (top-left),
//! level (top-center); the shop screen.
//!
//! Plumbing (this doc comment's owner: bootstrap/integrator) -- the
//! glyphon text pipeline (`FontSystem`/`TextRenderer`/etc.) and the quad
//! pipeline both live in `render/mod.rs`, which you don't own; `build`
//! below is the seam: return the quads/text you want drawn, in logical
//! playfield coordinates, and `Renderer::render` handles turning them
//! into an actual draw call. Use `super::make_line_buffer(font_system,
//! text, font_size, line_height, wrap_width, align)` to shape a line of
//! text (mirrors arkanoid's identical helper), and `super::QuadInstance`
//! (accessible as a private sibling item since this module is a
//! descendant of `render`) for a quad -- e.g. an HP bar's background and
//! fill.
//!
//! Owner: Wave 3 `hud-lives` (HP/cash/score/level). Extended by Wave 4
//! `shop-wiring` (shop screen), Wave 5 `juice-fx` (HP-bar flash on
//! damage).

use glyphon::{Buffer as TextBuffer, Color as TextColor, FontSystem};

use crate::game::Game;

/// Headroom reserved in `mod.rs`'s shared instance buffer for this
/// module's own quads (e.g. an HP bar's background + fill) -- bump if a
/// future wave's worst-case frame needs more than it reserves.
pub(super) const MAX_HUD_QUADS: usize = 8;

/// One piece of HUD text: a shaped glyphon buffer plus where to draw it,
/// in the same logical playfield coordinates as everything else
/// (`Renderer::render` converts to physical pixels for you via
/// `to_physical`).
pub(super) struct HudText {
    pub buffer: TextBuffer,
    pub x: f32,
    pub y: f32,
    pub color: TextColor,
}

/// Everything this module wants drawn this frame.
pub(super) struct HudDraw {
    pub quads: Vec<super::QuadInstance>,
    pub texts: Vec<HudText>,
}

/// TODO(wave3 `hud-lives`): HP bar bottom-left (spec) -- e.g. two quads
/// (a dim background rect + a bright fill rect scaled to
/// `game.player.hp / game::player::MAX_HP`), cash bottom-right, score
/// top-left, level top-center, all as text via `super::make_line_buffer`.
/// `quads.len()` must stay `<= MAX_HUD_QUADS` above.
pub(super) fn build(font_system: &mut FontSystem, game: &Game) -> HudDraw {
    let _ = (font_system, game);
    HudDraw {
        quads: Vec::new(),
        texts: Vec::new(),
    }
}
