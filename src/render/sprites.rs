//! Entity draw-list building: player, enemies, projectiles, pickups.
//!
//! Owner: Wave 1 `render-pipeline` (ship quad). Extended by Wave 2
//! `waves-scheduler` (enemies/shots/bullets), Wave 5 `juice-fx` (muzzle
//! flash quad, enemy-death shrinking quads).

use super::QuadInstance;
use crate::events::GameEvent;
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

/// Muzzle flash quad color on `GameEvent::ShotFired` -- near-white so it
/// reads as a bright flash rather than another yellow shot.
const MUZZLE_FLASH_COLOR: [f32; 4] = [1.0, 1.0, 0.85, 1.0];
/// Bigger than a shot's own radius (`weapons::SHOT_RADIUS`) so the flash
/// is visibly distinct from the projectile it's drawn alongside.
const MUZZLE_FLASH_HALF_SIZE: f32 = 6.0;

/// Death-burst fragment color -- warm ember orange, distinct from the
/// ship/enemy/shot colors above.
const DEATH_PARTICLE_COLOR: [f32; 4] = [1.0, 0.6, 0.2, 1.0];
/// Spec: "enemy death = 4 shrinking quads".
const DEATH_PARTICLE_COUNT: usize = 4;
/// How far (px) a fragment can land from the enemy's death position --
/// spreads the 4 quads into a burst instead of a stacked single square.
const DEATH_PARTICLE_SPREAD: f32 = 8.0;
/// Per-particle starting half-size range (px), randomized so the burst
/// doesn't read as 4 identical squares.
const DEATH_PARTICLE_MIN_HALF_SIZE: f32 = 2.0;
const DEATH_PARTICLE_MAX_HALF_SIZE: f32 = 5.0;
/// Seconds a fragment takes to shrink from its starting size to nothing
/// -- short enough to read as a burst flash, not a lingering effect
/// (spec: "Juice, all cheap").
pub(super) const DEATH_PARTICLE_LIFETIME: f32 = 0.25;

/// One "shrinking quad" fragment of an enemy-death burst. `render()`
/// spawns `DEATH_PARTICLE_COUNT` of these per `GameEvent::EnemyDied` and
/// decays them every call via `decay_death_particles` -- a real
/// multi-frame shrink (each fragment's drawn size is `base_half_size *
/// life / DEATH_PARTICLE_LIFETIME`) rather than a single-frame
/// approximation, since `Renderer` (which owns this list across frames)
/// is where that state lives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DeathParticle {
    x: f32,
    y: f32,
    base_half_size: f32,
    life: f32,
}

/// Spawns one death burst (`DEATH_PARTICLE_COUNT` fragments) scattered
/// around `(x, y)`, each with a randomized size and full starting life.
pub(super) fn spawn_death_particles(x: f32, y: f32) -> Vec<DeathParticle> {
    (0..DEATH_PARTICLE_COUNT)
        .map(|_| DeathParticle {
            x: x + rand::random_range(-DEATH_PARTICLE_SPREAD..=DEATH_PARTICLE_SPREAD),
            y: y + rand::random_range(-DEATH_PARTICLE_SPREAD..=DEATH_PARTICLE_SPREAD),
            base_half_size: rand::random_range(
                DEATH_PARTICLE_MIN_HALF_SIZE..=DEATH_PARTICLE_MAX_HALF_SIZE,
            ),
            life: DEATH_PARTICLE_LIFETIME,
        })
        .collect()
}

/// Ages every fragment in `particles` by `dt` seconds and drops the ones
/// that have fully shrunk away -- called once per `Renderer::render`.
pub(super) fn decay_death_particles(particles: &mut Vec<DeathParticle>, dt: f32) {
    for p in particles.iter_mut() {
        p.life -= dt;
    }
    particles.retain(|p| p.life > 0.0);
}

/// Builds this frame's entity quads: the ship, drawn as a square the size
/// of its hitbox (`PLAYER_RADIUS`) at its (already-interpolated)
/// `player_x`/`player_y` position, followed by one quad per live enemy
/// (`game.enemies`, sized `enemies::RADIUS`), one per live player shot
/// (`game.player_shots`, sized `weapons::SHOT_RADIUS`), one muzzle-flash
/// quad per `GameEvent::ShotFired` in `frame_events` (this frame only --
/// `frame_events` only ever holds one frame's worth by the time it gets
/// here, see `main.rs`'s clear-after-render), and one quad per live entry
/// in `death_particles` (`Renderer`'s multi-frame enemy-death bursts).
/// `game` is the live (non-interpolated) simulation state read here only
/// -- no simulation logic lives in this file; only the ship's own
/// position needs the prev/current interpolation `render::mod`'s caller
/// already did for `player_x`/`player_y`.
pub(super) fn build(
    player_x: f32,
    player_y: f32,
    game: &crate::game::Game,
    frame_events: &[GameEvent],
    death_particles: &[DeathParticle],
) -> Vec<QuadInstance> {
    let mut instances = Vec::with_capacity(
        1 + game.enemies.len()
            + game.player_shots.len()
            + frame_events.len()
            + death_particles.len(),
    );
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
    for event in frame_events {
        if let GameEvent::ShotFired { x, y } = event {
            instances.push(QuadInstance::new(
                [*x, *y],
                [MUZZLE_FLASH_HALF_SIZE, MUZZLE_FLASH_HALF_SIZE],
                MUZZLE_FLASH_COLOR,
            ));
        }
    }
    for p in death_particles {
        let shrink = (p.life / DEATH_PARTICLE_LIFETIME).clamp(0.0, 1.0);
        let half = p.base_half_size * shrink;
        instances.push(QuadInstance::new(
            [p.x, p.y],
            [half, half],
            DEATH_PARTICLE_COLOR,
        ));
    }
    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_draws_exactly_one_quad_for_the_ship_at_its_position() {
        let instances = build(123.0, 456.0, &crate::game::Game::new(), &[], &[]);

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

        let instances = build(123.0, 456.0, &game, &[], &[]);

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

    #[test]
    fn build_adds_one_muzzle_flash_quad_per_shot_fired_event() {
        let game = crate::game::Game::new();
        let events = [
            GameEvent::ShotFired { x: 5.0, y: 6.0 },
            GameEvent::ShotFired { x: 7.0, y: 8.0 },
        ];

        let instances = build(0.0, 0.0, &game, &events, &[]);

        assert_eq!(instances.len(), 3); // ship + 2 muzzle flashes
        assert_eq!(instances[1].center, [5.0, 6.0]);
        assert_eq!(instances[1].color, MUZZLE_FLASH_COLOR);
        assert_eq!(instances[2].center, [7.0, 8.0]);
        assert_eq!(instances[2].color, MUZZLE_FLASH_COLOR);
    }

    #[test]
    fn build_ignores_non_shot_fired_events_for_muzzle_flash() {
        let game = crate::game::Game::new();
        let events = [GameEvent::PlayerHit { x: 1.0, y: 2.0 }];

        let instances = build(0.0, 0.0, &game, &events, &[]);

        assert_eq!(instances.len(), 1); // ship only, no flash
    }

    #[test]
    fn spawn_death_particles_returns_four_fragments_near_the_death_point() {
        let particles = spawn_death_particles(100.0, 200.0);

        assert_eq!(particles.len(), DEATH_PARTICLE_COUNT);
        for p in &particles {
            assert!((p.x - 100.0).abs() <= DEATH_PARTICLE_SPREAD);
            assert!((p.y - 200.0).abs() <= DEATH_PARTICLE_SPREAD);
            assert_eq!(p.life, DEATH_PARTICLE_LIFETIME);
            assert!(p.base_half_size >= DEATH_PARTICLE_MIN_HALF_SIZE);
            assert!(p.base_half_size <= DEATH_PARTICLE_MAX_HALF_SIZE);
        }
    }

    #[test]
    fn decay_death_particles_shrinks_life_and_drops_expired_fragments() {
        let mut particles = vec![
            DeathParticle {
                x: 0.0,
                y: 0.0,
                base_half_size: 4.0,
                life: 0.05,
            },
            DeathParticle {
                x: 0.0,
                y: 0.0,
                base_half_size: 4.0,
                life: 0.2,
            },
        ];

        decay_death_particles(&mut particles, 0.1);

        assert_eq!(particles.len(), 1); // the 0.05-life fragment expired
        assert!((particles[0].life - 0.1).abs() < 1e-6);
    }

    #[test]
    fn build_draws_a_death_particle_quad_shrunk_by_remaining_life() {
        let game = crate::game::Game::new();
        let particles = [DeathParticle {
            x: 10.0,
            y: 20.0,
            base_half_size: 4.0,
            life: DEATH_PARTICLE_LIFETIME / 2.0,
        }];

        let instances = build(0.0, 0.0, &game, &[], &particles);

        assert_eq!(instances.len(), 2); // ship + 1 death particle
        assert_eq!(instances[1].center, [10.0, 20.0]);
        assert_eq!(instances[1].color, DEATH_PARTICLE_COLOR);
        assert!((instances[1].half_size[0] - 2.0).abs() < 1e-4);
    }
}
