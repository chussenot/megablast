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
    /// attack-pattern cycle.
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
/// TODO(wave2 `enemies-basic`): Popcorn (sine-drift down, no fire), Diver
/// (dive straight at `entry_target_*` once, no homing).
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
        e.age += dt;
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
