//! Weapon tree: main cannon tiers 1-4, side/rear shots, drones, and the
//! projectiles they fire. No wgpu/winit types.
//!
//! Owner: Wave 2 `weapons-t1` implements tier-1 cannon firing (this
//! file's shape is fixed -- `game/mod.rs` already calls `update` /
//! `advance_shots` / `downgrade`). Wave 3 `weapons-t2-4` extends the same
//! `update()` body for tiers 2-4, side shots, rear shot, and drone
//! auto-fire -- keep tier 1 working when you extend it.

pub const PROJECTILE_SPEED: f32 = 520.0;
pub const CANNON_DAMAGE: u32 = 10;
pub const SHOT_RADIUS: f32 = 4.0;
const FIRE_INTERVAL: f32 = 1.0 / 6.0; // 6 volleys/s, constant across tiers
const DRONE_FIRE_INTERVAL: f32 = 0.5; // 2/s per drone
const DRONE_DAMAGE: u32 = 10;

#[derive(Debug, Clone, Copy)]
pub struct Loadout {
    pub cannon_tier: u8, // 1..=4
    pub has_side: bool,
    pub has_rear: bool,
    pub drones: u8, // 0..=2
    pub fire_cooldown: f32,
    pub drone_cooldown: f32,
}

impl Default for Loadout {
    fn default() -> Self {
        Self {
            cannon_tier: 1,
            has_side: false,
            has_rear: false,
            drones: 0,
            fire_cooldown: 0.0,
            drone_cooldown: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Shot {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub damage: u32,
}

/// Fires from (origin_x, origin_y) while `fire_held`, appending `Shot`s
/// and pushing `GameEvent::ShotFired`.
///
/// TODO(wave2 `weapons-t1`): tier-1 path -- one forward shot every
/// `FIRE_INTERVAL` (use `loadout.fire_cooldown`, already ticked down
/// below), damage `CANNON_DAMAGE`, speed `PROJECTILE_SPEED`.
///
/// TODO(wave3 `weapons-t2-4`): tiers 2-4 (2/3/5 projectiles, spread
/// widening), side shots (+left+right), rear shot (+backward), and drone
/// auto-fire (`DRONE_FIRE_INTERVAL` per drone via `loadout.drone_cooldown`,
/// forward, `DRONE_DAMAGE`).
pub fn update(
    loadout: &mut Loadout,
    shots: &mut Vec<Shot>,
    origin_x: f32,
    origin_y: f32,
    fire_held: bool,
    dt: f32,
    events: &mut Vec<crate::events::GameEvent>,
) {
    loadout.fire_cooldown = (loadout.fire_cooldown - dt).max(0.0);
    loadout.drone_cooldown = (loadout.drone_cooldown - dt).max(0.0);
    let _ = (shots, origin_x, origin_y, fire_held, events);
}

/// Advances existing shots by `dt` -- pure physics, no tier logic needed.
pub fn advance_shots(shots: &mut [Shot], dt: f32) {
    for s in shots.iter_mut() {
        s.x += s.vx * dt;
        s.y += s.vy * dt;
    }
}

/// One-tier downgrade on life loss, floored at tier 1 (spec).
pub fn downgrade(loadout: &mut Loadout) {
    loadout.cannon_tier = loadout.cannon_tier.saturating_sub(1).max(1);
}
