//! Entity draw-list building: player, enemies, projectiles, pickups.
//!
//! Owner: Wave 1 `render-pipeline` (ship quad). Extended by Wave 2
//! `waves-scheduler` (enemies/shots/bullets), Wave 5 `juice-fx` (muzzle
//! flash quad, enemy-death shrinking quads).

use super::QuadInstance;
use crate::game::enemies;
use crate::game::player::PLAYER_RADIUS;
use crate::game::weapons;

/// Headroom this file's instance list is allowed to grow into without
/// `mod.rs`'s instance buffer needing a resize -- ship + enemies + player
/// shots as of Wave 2; enemy bullets/pickups and muzzle-flash/death quads
/// (Wave 5) land here later. Bumped from 256: level scripts are
/// data-driven with no hard cap on concurrent enemies, and a maxed-out
/// loadout (tier 4 + side + rear + 2 drones) can keep several volleys'
/// worth of shots alive on screen at once. Bump again if a future wave's
/// worst-case frame needs more than this reserves.
pub(super) const MAX_ENTITY_QUADS: usize = 512;

/// Ship hull color -- a cool cyan-white so it reads clearly against the
/// dim starfield background.
const SHIP_COLOR: [f32; 4] = [0.75, 0.92, 1.0, 1.0];

/// Enemy hull color -- warm red-orange, reads clearly as hostile against
/// the ship's cool cyan and the dim starfield.
const ENEMY_COLOR: [f32; 4] = [0.95, 0.35, 0.25, 1.0];

/// Player shot color -- bright yellow, distinct from both the ship and
/// enemies for quick target-vs-bullet reading at a glance.
const SHOT_COLOR: [f32; 4] = [1.0, 0.92, 0.3, 1.0];

/// Builds this frame's entity quads: the ship, drawn as a square the size
/// of its hitbox (`PLAYER_RADIUS`) at its (already-interpolated)
/// `player_x`/`player_y` position, followed by one quad per live enemy
/// (`game.enemies`, sized `enemies::RADIUS`) and one per live player shot
/// (`game.player_shots`, sized `weapons::SHOT_RADIUS`). `game` is the
/// live (non-interpolated) simulation state read here only -- no
/// simulation logic lives in this file; only the ship's own position
/// needs the prev/current interpolation `render::mod`'s caller already
/// did for `player_x`/`player_y`.
pub(super) fn build(player_x: f32, player_y: f32, game: &crate::game::Game) -> Vec<QuadInstance> {
    let mut instances = Vec::with_capacity(1 + game.enemies.len() + game.player_shots.len());
    instances.push(QuadInstance::new(
        [player_x, player_y],
        [PLAYER_RADIUS, PLAYER_RADIUS],
        SHIP_COLOR,
    ));
    for e in &game.enemies {
        instances.push(QuadInstance::new(
            [e.x, e.y],
            [enemies::RADIUS, enemies::RADIUS],
            ENEMY_COLOR,
        ));
    }
    for s in &game.player_shots {
        instances.push(QuadInstance::new(
            [s.x, s.y],
            [weapons::SHOT_RADIUS, weapons::SHOT_RADIUS],
            SHOT_COLOR,
        ));
    }
    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_draws_exactly_one_quad_for_the_ship_at_its_position() {
        let instances = build(123.0, 456.0, &crate::game::Game::new());

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].center, [123.0, 456.0]);
        assert_eq!(instances[0].half_size, [PLAYER_RADIUS, PLAYER_RADIUS]);
        assert_eq!(instances[0].color, SHIP_COLOR);
    }

    #[test]
    fn build_adds_one_quad_per_enemy_and_per_player_shot() {
        let mut game = crate::game::Game::new();
        game.enemies.push(enemies::spawn(
            crate::levels::EnemyKind::Popcorn,
            50.0,
            60.0,
            123.0,
            456.0,
        ));
        game.player_shots.push(weapons::Shot {
            x: 10.0,
            y: 20.0,
            vx: 0.0,
            vy: -weapons::PROJECTILE_SPEED,
            damage: weapons::CANNON_DAMAGE,
        });

        let instances = build(123.0, 456.0, &game);

        assert_eq!(instances.len(), 3); // ship + 1 enemy + 1 shot
        assert_eq!(instances[1].center, [50.0, 60.0]);
        assert_eq!(instances[1].half_size, [enemies::RADIUS, enemies::RADIUS]);
        assert_eq!(instances[1].color, ENEMY_COLOR);
        assert_eq!(instances[2].center, [10.0, 20.0]);
        assert_eq!(
            instances[2].half_size,
            [weapons::SHOT_RADIUS, weapons::SHOT_RADIUS]
        );
        assert_eq!(instances[2].color, SHOT_COLOR);
    }
}
