//! Enemy types, movement/attack patterns, and the credit pickups they
//! drop. No wgpu/winit types.
//!
//! Owner: Wave 2 `enemies-basic` implements Popcorn + Diver (this file's
//! shape is fixed -- `game/mod.rs` already calls `spawn` / `update` /
//! `apply_damage` / `advance_pickups`). Wave 3 `enemies-advanced` extends
//! the same `update()` for Turret/Weaver/Boss. Wave 4 `drops-cash`
//! implements `maybe_drop`'s real probability roll.

use crate::levels::EnemyKind;

pub const RADIUS: f32 = 14.0;
pub const BULLET_RADIUS: f32 = 3.0;
pub const PICKUP_FALL_SPEED: f32 = 100.0;
pub const PICKUP_RADIUS: f32 = 6.0;

/// Popcorn: gentle downward fall plus a horizontal sine sway.
const POPCORN_FALL_SPEED: f32 = 60.0; // px/s downward
const POPCORN_DRIFT_AMPLITUDE: f32 = 40.0; // px either side of spawn x
const POPCORN_DRIFT_FREQUENCY: f32 = 2.0; // rad/s (~3s sway period)

/// Diver: constant-speed dive toward its one-shot entry target.
const DIVER_SPEED: f32 = 220.0; // px/s

/// Turret: fixed emplacement, aims fresh at the player every shot.
const TURRET_FIRE_INTERVAL: f32 = 0.7; // s between aimed shots
const TURRET_BULLET_SPEED: f32 = 180.0; // px/s

/// Weaver: horizontal figure-eight around its spawn point via a
/// continuously-advancing angle (`phase`), dropping straight bullets.
const WEAVER_AMPLITUDE_X: f32 = 90.0; // px either side of spawn x
const WEAVER_AMPLITUDE_Y: f32 = 40.0; // px above/below spawn y (figure-eight lobe)
const WEAVER_FREQUENCY: f32 = 1.5; // rad/s
const WEAVER_FIRE_INTERVAL: f32 = 0.5; // s between drops
const WEAVER_BULLET_SPEED: f32 = 150.0; // px/s, straight down

/// Boss: stationary in the top third, cycling 3 attack patterns every
/// `BOSS_CYCLE_DURATION` seconds (one third of the cycle each).
const BOSS_CYCLE_DURATION: f32 = 6.0; // s, full pattern cycle
const BOSS_PATTERN_DURATION: f32 = 2.0; // s per pattern (cycle / 3)
const BOSS_BULLET_SPEED: f32 = 160.0; // px/s
const BOSS_WALL_BULLET_COUNT: usize = 10; // bullets across the wall-with-gap row
const BOSS_SPIRAL_COUNT: usize = 12; // bullets in the spiral pattern (spec: 12)
const BOSS_SPIRAL_ROTATION_RATE: f32 = 0.6; // rad/s, rotates the spiral cycle-to-cycle

/// Flat damage every enemy bullet carries (spec: "enemy bullet 10").
const ENEMY_BULLET_DAMAGE: u32 = 10;

#[derive(Debug, Clone, Copy)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub kind: EnemyKind,
    pub hp: i32,
    pub age: f32,
    /// Diver: the ship's position at spawn time, its one-shot dive target
    /// (spec: "no homing"). Weaver has no anchor slot of its own, so it
    /// borrows this pair too (otherwise dead weight for that kind) to
    /// cache its own spawn (x, y) -- the center its figure-eight orbits.
    pub entry_target_x: f32,
    pub entry_target_y: f32,
    /// Weaver/Boss: phase accumulator for figure-eight motion / the 6s
    /// attack-pattern cycle. Popcorn/Diver have no accumulator of their
    /// own, so they reuse this slot to cache a value fixed at spawn
    /// time (see `update`): Popcorn stores its spawn x (the sine-drift
    /// anchor), Diver stores its one-shot dive angle.
    pub phase: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub damage: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Pickup {
    pub x: f32,
    pub y: f32,
    pub value: u32,
}

/// Everything about an enemy that just died, for the caller to score and
/// maybe drop a pickup for -- avoids needing the removed `Enemy` value
/// after `apply_damage` already removed it from the vec.
#[derive(Debug, Clone, Copy)]
pub struct DeathInfo {
    pub x: f32,
    pub y: f32,
    pub kind: EnemyKind,
    pub credit_value: u32,
}

pub fn max_hp(kind: EnemyKind) -> i32 {
    match kind {
        EnemyKind::Popcorn => 10,
        EnemyKind::Diver => 20,
        EnemyKind::Weaver => 30,
        EnemyKind::Turret => 40,
        EnemyKind::Boss => 1200,
    }
}

pub fn score_value(kind: EnemyKind) -> u32 {
    match kind {
        EnemyKind::Popcorn => 50,
        EnemyKind::Diver => 100,
        EnemyKind::Weaver => 120,
        EnemyKind::Turret => 150,
        EnemyKind::Boss => 5000,
    }
}

/// Spawns one `kind` at (x, y). `player_x`/`player_y` fix Diver's
/// one-shot dive target at entry time.
pub fn spawn(kind: EnemyKind, x: f32, y: f32, player_x: f32, player_y: f32) -> Enemy {
    Enemy {
        x,
        y,
        kind,
        hp: max_hp(kind),
        age: 0.0,
        entry_target_x: player_x,
        entry_target_y: player_y,
        phase: 0.0,
    }
}

/// Advances every enemy's movement/firing by `dt`, appending to `bullets`
/// and pushing events as needed.
///
/// Popcorn sine-drifts down (no fire); Diver dives in a straight line at
/// constant speed toward `entry_target_*`, its direction fixed at spawn
/// (no homing on the player's current position).
///
/// Turret rides the background scroll (`scroll_dy`) and fires an aimed
/// shot at the player's *current* position every `TURRET_FIRE_INTERVAL`
/// (re-aimed fresh each shot, unlike Diver's fixed dive). Weaver orbits
/// its spawn point in a horizontal figure-eight via the continuously
/// advancing angle `phase`, dropping a straight-down bullet every
/// `WEAVER_FIRE_INTERVAL`. Boss holds in the top third of the playfield
/// and cycles 3 attack patterns every `BOSS_CYCLE_DURATION` seconds
/// (tracked by `phase` as an elapsed-time-in-cycle accumulator): aimed
/// shot, wall-with-a-gap, spiral of 12 -- pushing `BossPatternChanged`
/// each time the active pattern changes (see `apply_damage` for
/// `BossHit`, pushed there instead of `EnemyDamaged`).
pub fn update(
    enemies: &mut [Enemy],
    bullets: &mut Vec<Bullet>,
    scroll_dy: f32,
    player_x: f32,
    player_y: f32,
    dt: f32,
    events: &mut Vec<crate::events::GameEvent>,
) {
    for e in enemies.iter_mut() {
        match e.kind {
            EnemyKind::Popcorn => {
                // First tick since spawn: x/y are still the untouched
                // spawn point, so this is the one chance to remember
                // it -- `phase` doubles as the sine-drift anchor.
                if e.age <= 0.0 {
                    e.phase = e.x;
                }
                e.age += dt;
                e.y += POPCORN_FALL_SPEED * dt;
                e.x = e.phase + POPCORN_DRIFT_AMPLITUDE * (e.age * POPCORN_DRIFT_FREQUENCY).sin();
            }
            EnemyKind::Diver => {
                // Same one-chance trick: fix the dive angle toward the
                // (already-frozen) entry target on the first tick, then
                // just keep flying that heading -- no re-aiming.
                if e.age <= 0.0 {
                    e.phase = (e.entry_target_y - e.y).atan2(e.entry_target_x - e.x);
                }
                e.age += dt;
                e.x += DIVER_SPEED * dt * e.phase.cos();
                e.y += DIVER_SPEED * dt * e.phase.sin();
            }
            EnemyKind::Turret => {
                // Fixed emplacement -- it doesn't move under its own
                // power, just rides the background scroll.
                let age_before = e.age;
                e.age += dt;
                e.y += scroll_dy;
                if crosses_interval(age_before, e.age, TURRET_FIRE_INTERVAL) {
                    bullets.push(aimed_bullet(
                        e.x,
                        e.y,
                        player_x,
                        player_y,
                        TURRET_BULLET_SPEED,
                    ));
                }
            }
            EnemyKind::Weaver => {
                // First tick since spawn: x/y are still the untouched
                // spawn point, so this is the one chance to cache it as
                // the figure-eight's anchor (see the `entry_target_*`
                // doc comment on `Enemy` for why this pair is free to
                // borrow here).
                if e.age <= 0.0 {
                    e.entry_target_x = e.x;
                    e.entry_target_y = e.y;
                }
                let age_before = e.age;
                e.age += dt;
                e.phase += dt * WEAVER_FREQUENCY;
                e.x = e.entry_target_x + WEAVER_AMPLITUDE_X * e.phase.sin();
                e.y = e.entry_target_y + WEAVER_AMPLITUDE_Y * (2.0 * e.phase).sin();
                if crosses_interval(age_before, e.age, WEAVER_FIRE_INTERVAL) {
                    bullets.push(Bullet {
                        x: e.x,
                        y: e.y,
                        vx: 0.0,
                        vy: WEAVER_BULLET_SPEED,
                        damage: ENEMY_BULLET_DAMAGE,
                    });
                }
            }
            EnemyKind::Boss => {
                e.age += dt;
                e.y = super::PLAYFIELD_HEIGHT / 3.0;
                let pattern_before = boss_pattern_index(e.phase);
                e.phase += dt;
                if e.phase >= BOSS_CYCLE_DURATION {
                    e.phase -= BOSS_CYCLE_DURATION;
                }
                let pattern_after = boss_pattern_index(e.phase);
                if pattern_after != pattern_before {
                    events.push(crate::events::GameEvent::BossPatternChanged);
                    fire_boss_pattern(pattern_after, e, bullets, player_x, player_y);
                }
            }
        }
    }
}

/// True once `age` (having just advanced from `age_before` to
/// `age_after` by `dt`) has crossed a multiple of `interval` -- i.e. the
/// periodic-fire cadence "every `interval` seconds" without a dedicated
/// cooldown field.
fn crosses_interval(age_before: f32, age_after: f32, interval: f32) -> bool {
    (age_before / interval).floor() < (age_after / interval).floor()
}

/// A bullet from `(x, y)` aimed at `(target_x, target_y)` at `speed`,
/// re-aimed fresh each call (unlike Diver's fixed one-shot dive angle).
fn aimed_bullet(x: f32, y: f32, target_x: f32, target_y: f32, speed: f32) -> Bullet {
    let dx = target_x - x;
    let dy = target_y - y;
    let len = dx.hypot(dy).max(f32::EPSILON);
    Bullet {
        x,
        y,
        vx: speed * dx / len,
        vy: speed * dy / len,
        damage: ENEMY_BULLET_DAMAGE,
    }
}

/// Which third of the boss's `BOSS_CYCLE_DURATION`-second cycle `phase`
/// (already wrapped into `[0, BOSS_CYCLE_DURATION)`) falls in: 0 = aimed
/// shot, 1 = wall-with-gap, 2 = spiral of 12.
fn boss_pattern_index(phase: f32) -> usize {
    ((phase / BOSS_PATTERN_DURATION) as usize).min(2)
}

/// Fires the attack for `pattern` (see `boss_pattern_index`) from `e`.
fn fire_boss_pattern(
    pattern: usize,
    e: &Enemy,
    bullets: &mut Vec<Bullet>,
    player_x: f32,
    player_y: f32,
) {
    match pattern {
        0 => bullets.push(aimed_bullet(
            e.x,
            e.y,
            player_x,
            player_y,
            BOSS_BULLET_SPEED,
        )),
        1 => {
            // Wall with a gap: a horizontal row of bullets spanning the
            // playfield width, skipping one gap slot the player can fly
            // through.
            let gap = BOSS_WALL_BULLET_COUNT / 2;
            let slot_width = super::PLAYFIELD_WIDTH / BOSS_WALL_BULLET_COUNT as f32;
            for i in 0..BOSS_WALL_BULLET_COUNT {
                if i == gap {
                    continue;
                }
                bullets.push(Bullet {
                    x: (i as f32 + 0.5) * slot_width,
                    y: e.y,
                    vx: 0.0,
                    vy: BOSS_BULLET_SPEED,
                    damage: ENEMY_BULLET_DAMAGE,
                });
            }
        }
        _ => {
            // Spiral of 12, evenly spaced by angle; the rotation offset
            // grows with the boss's own age, so each time this pattern
            // comes back around (every full 6s cycle) it fires from a
            // different angle -- the "rotating spiral" look.
            let offset = e.age * BOSS_SPIRAL_ROTATION_RATE;
            for i in 0..BOSS_SPIRAL_COUNT {
                let angle = offset + i as f32 * std::f32::consts::TAU / BOSS_SPIRAL_COUNT as f32;
                bullets.push(Bullet {
                    x: e.x,
                    y: e.y,
                    vx: BOSS_BULLET_SPEED * angle.cos(),
                    vy: BOSS_BULLET_SPEED * angle.sin(),
                    damage: ENEMY_BULLET_DAMAGE,
                });
            }
        }
    }
}

/// Applies `dmg` to `enemies[idx]`. If it dies: removes it, pushes
/// `EnemyDied`, and returns `Some(DeathInfo)` for the caller to score and
/// pass to `maybe_drop`. Otherwise pushes `EnemyDamaged` -- or `BossHit`
/// instead, when `kind == EnemyKind::Boss` -- and returns `None`.
pub fn apply_damage(
    enemies: &mut Vec<Enemy>,
    idx: usize,
    dmg: i32,
    events: &mut Vec<crate::events::GameEvent>,
) -> Option<DeathInfo> {
    let e = enemies.get_mut(idx)?;
    e.hp -= dmg;
    if e.hp <= 0 {
        let e = enemies.remove(idx);
        let credit_value = score_value(e.kind) / 10;
        events.push(crate::events::GameEvent::EnemyDied {
            x: e.x,
            y: e.y,
            kind: e.kind,
            credit_value,
        });
        Some(DeathInfo {
            x: e.x,
            y: e.y,
            kind: e.kind,
            credit_value,
        })
    } else if e.kind == EnemyKind::Boss {
        events.push(crate::events::GameEvent::BossHit { x: e.x, y: e.y });
        None
    } else {
        events.push(crate::events::GameEvent::EnemyDamaged { x: e.x, y: e.y });
        None
    }
}

/// Drop table: 20% of Popcorn, 100% of everything else (spec).
pub fn maybe_drop(kind: EnemyKind, x: f32, y: f32, credit_value: u32) -> Option<Pickup> {
    let chance = match kind {
        EnemyKind::Popcorn => 0.2,
        _ => 1.0,
    };
    if rand::random::<f32>() < chance {
        Some(Pickup {
            x,
            y,
            value: credit_value,
        })
    } else {
        None
    }
}

/// Advances falling pickups straight down at `PICKUP_FALL_SPEED`.
pub fn advance_pickups(pickups: &mut [Pickup], dt: f32) {
    for p in pickups.iter_mut() {
        p.y += PICKUP_FALL_SPEED * dt;
    }
}

/// Advances every enemy bullet by its own velocity -- pure physics, same
/// shape as `weapons::advance_shots`. Bootstrap's original `tick_playing`
/// never called an equivalent for `enemy_bullets`, so bullets appeared
/// but never moved; caught by removing the temporary crate-wide
/// `dead_code` allow after wave 5 (bullet `vx`/`vy` were never read).
pub fn advance_bullets(bullets: &mut [Bullet], dt: f32) {
    for b in bullets.iter_mut() {
        b.x += b.vx * dt;
        b.y += b.vy * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::EnemyKind;

    #[test]
    fn popcorn_falls_and_sways_around_its_spawn_x() {
        let mut enemies = vec![spawn(EnemyKind::Popcorn, 100.0, 0.0, 0.0, 0.0)];
        let mut bullets = Vec::new();
        let mut events = Vec::new();
        for _ in 0..120 {
            update(
                &mut enemies,
                &mut bullets,
                0.0,
                0.0,
                0.0,
                1.0 / 60.0,
                &mut events,
            );
        }
        let e = enemies[0];
        assert!(e.y > 0.0, "popcorn should have drifted downward, y={}", e.y);
        assert!(
            (e.x - 100.0).abs() <= POPCORN_DRIFT_AMPLITUDE + 1e-3,
            "popcorn x={} should stay within amplitude of its spawn x=100",
            e.x
        );
        assert!(bullets.is_empty(), "popcorn must never fire");
    }

    #[test]
    fn diver_flies_a_straight_line_and_does_not_home_on_a_moving_player() {
        // Spawn at (0, 0), entry target (100, 100) (the "player's"
        // position at spawn time).
        let mut enemies = vec![spawn(EnemyKind::Diver, 0.0, 0.0, 100.0, 100.0)];
        let mut bullets = Vec::new();
        let mut events = Vec::new();
        // First tick fixes the dive angle toward (100, 100).
        update(
            &mut enemies,
            &mut bullets,
            0.0,
            0.0,
            0.0,
            1.0 / 60.0,
            &mut events,
        );
        // Player has since moved elsewhere; the diver must ignore that.
        update(
            &mut enemies,
            &mut bullets,
            0.0,
            -500.0,
            -500.0,
            1.0,
            &mut events,
        );
        let e = enemies[0];
        // Still heading toward (100, 100), i.e. x and y stay equal along
        // the 45-degree line from (0, 0).
        assert!(
            (e.x - e.y).abs() < 1e-3,
            "diver drifted off its fixed 45-degree line: x={}, y={}",
            e.x,
            e.y
        );
        assert!(
            e.x > 0.0 && e.y > 0.0,
            "diver should be moving toward its target"
        );
        assert!(bullets.is_empty(), "diver must never fire");
    }

    #[test]
    fn turret_fires_one_aimed_shot_every_0_7_seconds() {
        let mut enemies = vec![spawn(EnemyKind::Turret, 300.0, 100.0, 0.0, 0.0)];
        let mut bullets = Vec::new();
        let mut events = Vec::new();
        let dt = 1.0 / 120.0;
        let player_x = 500.0;
        let player_y = 700.0;

        // No scroll movement here, so the turret's (x, y) stay put and
        // the expected aim direction is easy to check against.
        let ticks_per_interval = (TURRET_FIRE_INTERVAL / dt).ceil() as usize + 1;
        for _ in 0..ticks_per_interval {
            update(
                &mut enemies,
                &mut bullets,
                0.0,
                player_x,
                player_y,
                dt,
                &mut events,
            );
        }
        assert_eq!(
            bullets.len(),
            1,
            "turret should have fired exactly once by just past {}s",
            TURRET_FIRE_INTERVAL
        );

        let e = enemies[0];
        let b = bullets[0];
        let expected_angle = (player_y - e.y).atan2(player_x - e.x);
        let actual_angle = b.vy.atan2(b.vx);
        assert!(
            (expected_angle - actual_angle).abs() < 1e-3,
            "turret's bullet should be aimed at the player: expected angle {}, got {}",
            expected_angle,
            actual_angle
        );
        let speed = b.vx.hypot(b.vy);
        assert!(
            (speed - TURRET_BULLET_SPEED).abs() < 1e-3,
            "turret bullet speed should be {} px/s, got {}",
            TURRET_BULLET_SPEED,
            speed
        );

        // A second interval should fire exactly one more shot.
        for _ in 0..ticks_per_interval {
            update(
                &mut enemies,
                &mut bullets,
                0.0,
                player_x,
                player_y,
                dt,
                &mut events,
            );
        }
        assert_eq!(
            bullets.len(),
            2,
            "turret should fire again after another 0.7s"
        );
    }

    #[test]
    fn turret_scrolls_down_with_the_background() {
        let mut enemies = vec![spawn(EnemyKind::Turret, 300.0, 0.0, 0.0, 0.0)];
        let mut bullets = Vec::new();
        let mut events = Vec::new();
        update(
            &mut enemies,
            &mut bullets,
            5.0,
            0.0,
            0.0,
            1.0 / 120.0,
            &mut events,
        );
        assert_eq!(enemies[0].y, 5.0, "turret should move by exactly scroll_dy");
    }

    #[test]
    fn weaver_orbits_its_spawn_point_in_a_figure_eight_and_drops_bullets() {
        let mut enemies = vec![spawn(EnemyKind::Weaver, 200.0, 50.0, 0.0, 0.0)];
        let mut bullets = Vec::new();
        let mut events = Vec::new();
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            // 10 seconds.
            update(&mut enemies, &mut bullets, 0.0, 0.0, 0.0, dt, &mut events);
            let e = enemies[0];
            assert!(
                (e.x - 200.0).abs() <= WEAVER_AMPLITUDE_X + 1e-2,
                "weaver x={} strayed beyond its horizontal amplitude around spawn x=200",
                e.x
            );
            assert!(
                (e.y - 50.0).abs() <= WEAVER_AMPLITUDE_Y + 1e-2,
                "weaver y={} strayed beyond its vertical amplitude around spawn y=50",
                e.y
            );
        }
        assert!(
            !bullets.is_empty(),
            "weaver should have dropped at least one bullet over 10s at 0.5/s"
        );
        assert!(
            bullets.iter().all(|b| b.vx == 0.0 && b.vy > 0.0),
            "weaver's bullets should drop straight down"
        );
    }

    #[test]
    fn boss_holds_top_third_and_cycles_patterns_every_two_seconds() {
        let mut enemies = vec![spawn(EnemyKind::Boss, 300.0, -100.0, 0.0, 0.0)];
        let mut bullets = Vec::new();
        let mut events = Vec::new();
        let dt = 1.0 / 60.0;
        let ticks_per_pattern = (BOSS_PATTERN_DURATION / dt).round() as usize;
        // Three full 6s cycles' worth of pattern boundaries (9), plus a
        // small buffer so the last boundary is reliably captured without
        // reaching a 10th.
        let total_ticks = ticks_per_pattern * 9 + 5;

        for _ in 0..total_ticks {
            update(
                &mut enemies,
                &mut bullets,
                0.0,
                500.0,
                700.0,
                dt,
                &mut events,
            );
        }

        assert_eq!(
            enemies[0].y,
            crate::game::PLAYFIELD_HEIGHT / 3.0,
            "boss should hold in the top third of the playfield"
        );

        let changes = events
            .iter()
            .filter(|e| matches!(e, crate::events::GameEvent::BossPatternChanged))
            .count();
        assert_eq!(
            changes, 9,
            "boss should announce a pattern change every 2s across 3 full 6s cycles"
        );
        assert!(
            !bullets.is_empty(),
            "each pattern change should have fired at least one bullet"
        );
    }

    #[test]
    fn maybe_drop_popcorn_drops_about_20_percent_of_the_time() {
        let trials = 10_000;
        let drops = (0..trials)
            .filter(|_| maybe_drop(EnemyKind::Popcorn, 0.0, 0.0, 5).is_some())
            .count();
        let rate = drops as f32 / trials as f32;
        assert!(
            (0.15..0.25).contains(&rate),
            "popcorn drop rate {rate} should be near 0.2 over {trials} trials"
        );
    }

    #[test]
    fn maybe_drop_non_popcorn_always_drops() {
        let trials = 10_000;
        for kind in [
            EnemyKind::Diver,
            EnemyKind::Weaver,
            EnemyKind::Turret,
            EnemyKind::Boss,
        ] {
            let drops = (0..trials)
                .filter(|_| maybe_drop(kind, 1.0, 2.0, 7).is_some())
                .count();
            assert_eq!(
                drops, trials,
                "{kind:?} should always drop (100%), got {drops}/{trials}"
            );
        }
    }

    #[test]
    fn maybe_drop_returns_the_death_position_and_credit_value() {
        let pickup = maybe_drop(EnemyKind::Diver, 42.0, 84.0, 10).expect("diver always drops");
        assert_eq!(pickup.x, 42.0);
        assert_eq!(pickup.y, 84.0);
        assert_eq!(pickup.value, 10);
    }

    #[test]
    fn apply_damage_pushes_boss_hit_not_enemy_damaged_for_a_boss() {
        let mut enemies = vec![spawn(EnemyKind::Boss, 300.0, 200.0, 0.0, 0.0)];
        let mut events = Vec::new();
        let death = apply_damage(&mut enemies, 0, 10, &mut events);
        assert!(death.is_none(), "10 damage should not kill a 1200 HP boss");
        assert_eq!(events.len(), 1);
        match events[0] {
            crate::events::GameEvent::BossHit { x, y } => {
                assert_eq!(x, 300.0);
                assert_eq!(y, 200.0);
            }
            other => panic!("expected BossHit for a boss, got {other:?}"),
        }

        // Non-boss kinds still get the plain EnemyDamaged event.
        let mut popcorn = vec![spawn(EnemyKind::Popcorn, 10.0, 10.0, 0.0, 0.0)];
        let mut events2 = Vec::new();
        apply_damage(&mut popcorn, 0, 1, &mut events2);
        assert!(matches!(
            events2[0],
            crate::events::GameEvent::EnemyDamaged { .. }
        ));
    }
}
