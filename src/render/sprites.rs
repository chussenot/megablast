//! Entity draw-list building: player, enemies, projectiles, pickups.
//!
//! Owner: Wave 1 `render-pipeline` (ship quad). Extended by Wave 2
//! `waves-scheduler` (enemies/shots/bullets), Wave 5 `juice-fx` (muzzle
//! flash quad, enemy-death shrinking quads).

use super::QuadInstance;
use crate::game::player::PLAYER_RADIUS;

/// Headroom this file's instance list is allowed to grow into without
/// `mod.rs`'s instance buffer needing a resize -- just the ship for
/// Milestone 1; enemies/shots/enemy bullets/pickups (Wave 2) and
/// muzzle-flash/death quads (Wave 5) land here later. Bump this if a
/// future wave's worst-case frame needs more than it reserves.
pub(super) const MAX_ENTITY_QUADS: usize = 256;

/// Ship hull color -- a cool cyan-white so it reads clearly against the
/// dim starfield background.
const SHIP_COLOR: [f32; 4] = [0.75, 0.92, 1.0, 1.0];

/// Builds this frame's entity quads. Milestone 1: just the ship, drawn
/// as a square the size of its hitbox (`PLAYER_RADIUS`) at its
/// (already-interpolated) `player_x`/`player_y` position.
pub(super) fn build(player_x: f32, player_y: f32) -> Vec<QuadInstance> {
    vec![QuadInstance::new(
        [player_x, player_y],
        [PLAYER_RADIUS, PLAYER_RADIUS],
        SHIP_COLOR,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_draws_exactly_one_quad_for_the_ship_at_its_position() {
        let instances = build(123.0, 456.0);

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].center, [123.0, 456.0]);
        assert_eq!(instances[0].half_size, [PLAYER_RADIUS, PLAYER_RADIUS]);
        assert_eq!(instances[0].color, SHIP_COLOR);
    }
}
