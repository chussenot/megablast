//! Pure collision math: circle/circle, circle/AABB, and a substepped
//! swept check so fast entities never tunnel through each other in one
//! tick. No game entity types imported here on purpose -- callers pass
//! plain positions/radii, keeping this trivially unit-testable from any
//! file.
//!
//! Owner: Wave 2 `collide` task. `game/mod.rs` already calls
//! `circle_circle` every tick -- keep these signatures.

pub fn circle_circle(_x1: f32, _y1: f32, _r1: f32, _x2: f32, _y2: f32, _r2: f32) -> bool {
    false // TODO(wave2 `collide`)
}

pub fn circle_aabb(_cx: f32, _cy: f32, _r: f32, _rx: f32, _ry: f32, _rw: f32, _rh: f32) -> bool {
    false // TODO(wave2 `collide`)
}

/// Swept circle/circle check across `substeps` linear-interpolation steps
/// between each shape's previous and current position -- the
/// no-tunneling guard for a projectile vs. an enemy at max relative
/// speed (spec: assert this in a test, arkanoid-style).
#[allow(clippy::too_many_arguments)]
pub fn circle_circle_swept(
    _x1_prev: f32,
    _y1_prev: f32,
    _x1: f32,
    _y1: f32,
    _r1: f32,
    _x2_prev: f32,
    _y2_prev: f32,
    _x2: f32,
    _y2: f32,
    _r2: f32,
    _substeps: u32,
) -> bool {
    false // TODO(wave2 `collide`)
}
