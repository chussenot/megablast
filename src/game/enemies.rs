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

#[derive(Debug, Clone, Copy)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub kind: EnemyKind,
    pub hp: i32,
    pub age: f32,
    /// Diver: the ship's position at spawn time, its one-shot dive target
    /// (spec: "no homing").
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
/// TODO(wave3 `enemies-advanced`): Turret (scrolls with `scroll_dy`,
/// aimed shots at 0.7/s, bullet speed 180), Weaver (horizontal
/// figure-eight via `phase`, straight bullets at 0.5/s), Boss (holds in
/// the top third, 3 attack patterns cycling every 6s via `phase` --
/// aimed spread / wall-with-gap / spiral of 12 -- pushing
/// `BossPatternChanged` on each switch and `BossHit` instead of
/// `EnemyDamaged` when it takes damage; see `apply_damage`).
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
                // TODO(wave3 `enemies-advanced`): scroll with
                // `scroll_dy`, aimed shots at 0.7/s, bullet speed 180.
                e.age += dt;
            }
            EnemyKind::Weaver => {
                // TODO(wave3 `enemies-advanced`): horizontal
                // figure-eight via `phase`, straight bullets at 0.5/s.
                e.age += dt;
            }
            EnemyKind::Boss => {
                // TODO(wave3 `enemies-advanced`): hold in the top
                // third, 3 attack patterns cycling every 6s via `phase`
                // (aimed spread / wall-with-gap / spiral of 12), push
                // `BossPatternChanged` on each switch.
                e.age += dt;
            }
        }
    }
    let _ = (bullets, scroll_dy, player_x, player_y, events);
}

/// Applies `dmg` to `enemies[idx]`. If it dies: removes it, pushes
/// `EnemyDied`, and returns `Some(DeathInfo)` for the caller to score and
/// pass to `maybe_drop`. Otherwise pushes `EnemyDamaged` and returns
/// `None`. TODO(wave3): push `BossHit` instead of `EnemyDamaged` when
/// `kind == EnemyKind::Boss`.
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
    } else {
        events.push(crate::events::GameEvent::EnemyDamaged { x: e.x, y: e.y });
        None
    }
}

/// Drop table: 20% of Popcorn, 100% of everything else (spec). TODO
/// (wave4 `drops-cash`): roll the dice with `rand`. This stub never
/// drops, so earlier waves' collision wiring has something to call.
pub fn maybe_drop(kind: EnemyKind, x: f32, y: f32, credit_value: u32) -> Option<Pickup> {
    let _ = (kind, x, y, credit_value);
    None
}

/// Advances falling pickups straight down at `PICKUP_FALL_SPEED`.
pub fn advance_pickups(pickups: &mut [Pickup], dt: f32) {
    for p in pickups.iter_mut() {
        p.y += PICKUP_FALL_SPEED * dt;
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
}
