//! Weapon tree: main cannon tiers 1-4, side/rear shots, drones, and the
//! projectiles they fire. No wgpu/winit types.
//!
//! Wave 2 `weapons-t1` implemented tier-1 cannon firing (this file's shape
//! is fixed -- `game/mod.rs` already calls `update` / `advance_shots` /
//! `downgrade`). Wave 3 `weapons-t2-4` extended the same `update()` body
//! with tiers 2-4, side shots, rear shot, and drone auto-fire.

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
/// Tier 1: one forward shot every `FIRE_INTERVAL` (gated by
/// `loadout.fire_cooldown`, ticked down below), damage `CANNON_DAMAGE`,
/// speed `PROJECTILE_SPEED`. Tiers 2-4 add extra fanned-out projectiles to
/// the same volley (see `extra_cannon_angles`); side/rear shots add one
/// more projectile each, same cooldown gate. Drones auto-fire forward on
/// their own `DRONE_FIRE_INTERVAL` cadence, independent of `fire_held`.
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

    while fire_held && loadout.fire_cooldown <= 0.0 {
        shots.push(Shot {
            x: origin_x,
            y: origin_y,
            vx: 0.0,
            vy: -PROJECTILE_SPEED,
            damage: CANNON_DAMAGE,
        });
        events.push(crate::events::GameEvent::ShotFired {
            x: origin_x,
            y: origin_y,
        });
        loadout.fire_cooldown += FIRE_INTERVAL;

        // Tiers 2-4: extra projectiles fan out from the tier-1 forward shot
        // above, so tier N's total per volley is 1 + extras = 2/3/5. Angles
        // are degrees off dead-ahead (positive = right, matching the side
        // shot convention below); widening per tier is a fixed table since
        // there are only three of them (tier 2 narrowest at +/-10 deg [an
        // odd extra count leans right rather than split unevenly], tier 4
        // widest at +/-30 deg).
        for &angle_deg in extra_cannon_angles(loadout.cannon_tier) {
            let rad = angle_deg.to_radians();
            shots.push(Shot {
                x: origin_x,
                y: origin_y,
                vx: PROJECTILE_SPEED * rad.sin(),
                vy: -PROJECTILE_SPEED * rad.cos(),
                damage: CANNON_DAMAGE,
            });
            events.push(crate::events::GameEvent::ShotFired {
                x: origin_x,
                y: origin_y,
            });
        }

        if loadout.has_side {
            for vx in [-PROJECTILE_SPEED, PROJECTILE_SPEED] {
                shots.push(Shot {
                    x: origin_x,
                    y: origin_y,
                    vx,
                    vy: 0.0,
                    damage: CANNON_DAMAGE,
                });
                events.push(crate::events::GameEvent::ShotFired {
                    x: origin_x,
                    y: origin_y,
                });
            }
        }

        if loadout.has_rear {
            shots.push(Shot {
                x: origin_x,
                y: origin_y,
                vx: 0.0,
                vy: PROJECTILE_SPEED,
                damage: CANNON_DAMAGE,
            });
            events.push(crate::events::GameEvent::ShotFired {
                x: origin_x,
                y: origin_y,
            });
        }
    }

    // Drone auto-fire: independent of fire_held and of the cannon's own
    // cooldown above -- each owned drone fires forward on its own
    // DRONE_FIRE_INTERVAL cadence. A `while` (not `if`) mirrors the cannon
    // loop so a large `dt` still catches up correctly.
    while loadout.drones > 0 && loadout.drone_cooldown <= 0.0 {
        for _ in 0..loadout.drones {
            shots.push(Shot {
                x: origin_x,
                y: origin_y,
                vx: 0.0,
                vy: -PROJECTILE_SPEED,
                damage: DRONE_DAMAGE,
            });
            events.push(crate::events::GameEvent::ShotFired {
                x: origin_x,
                y: origin_y,
            });
        }
        loadout.drone_cooldown += DRONE_FIRE_INTERVAL;
    }
}

/// Extra cannon-tier projectile angles (degrees off dead-ahead, + = right),
/// fired alongside the always-on forward shot. See the comment at the call
/// site in `update` for the widening rationale.
fn extra_cannon_angles(tier: u8) -> &'static [f32] {
    match tier {
        2 => &[10.0],
        3 => &[-18.0, 18.0],
        4 => &[-30.0, -15.0, 15.0, 30.0],
        _ => &[],
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEvent;

    #[test]
    fn tier1_fires_one_shot_per_interval_while_held_and_accumulates_cooldown() {
        let mut loadout = Loadout::default();
        let mut shots = Vec::new();
        let mut events = Vec::new();

        // A single dt smaller than FIRE_INTERVAL fires exactly one shot.
        update(
            &mut loadout,
            &mut shots,
            100.0,
            200.0,
            true,
            0.01,
            &mut events,
        );
        assert_eq!(shots.len(), 1);
        assert_eq!(events.len(), 1);
        let s = shots[0];
        assert_eq!((s.x, s.y), (100.0, 200.0));
        assert_eq!((s.vx, s.vy), (0.0, -PROJECTILE_SPEED));
        assert_eq!(s.damage, CANNON_DAMAGE);
        assert!(matches!(events[0], GameEvent::ShotFired { x, y } if x == 100.0 && y == 200.0));

        // Not held: no new shot, cooldown just ticks down.
        update(
            &mut loadout,
            &mut shots,
            100.0,
            200.0,
            false,
            0.01,
            &mut events,
        );
        assert_eq!(shots.len(), 1);

        // Cooldown is set via `+=`, not `=`: firing while cooldown is
        // already at exactly zero leaves it at FIRE_INTERVAL, not some
        // smaller/negative-clamped remainder silently dropped.
        loadout.fire_cooldown = 0.0;
        update(&mut loadout, &mut shots, 0.0, 0.0, true, 0.5, &mut events);
        assert_eq!(shots.len(), 2);
        assert!((loadout.fire_cooldown - FIRE_INTERVAL).abs() < 1e-6);
    }

    fn speed_sq(s: &Shot) -> f32 {
        s.vx * s.vx + s.vy * s.vy
    }

    #[test]
    fn extra_cannon_angles_widen_with_tier_and_match_expected_counts() {
        assert_eq!(extra_cannon_angles(1), &[] as &[f32]);
        assert_eq!(extra_cannon_angles(2).len(), 1);
        assert_eq!(extra_cannon_angles(3).len(), 2);
        assert_eq!(extra_cannon_angles(4).len(), 4);

        let max_abs = |angles: &[f32]| angles.iter().fold(0.0_f32, |m, a| m.max(a.abs()));
        assert!(max_abs(extra_cannon_angles(2)) < max_abs(extra_cannon_angles(3)));
        assert!(max_abs(extra_cannon_angles(3)) < max_abs(extra_cannon_angles(4)));
    }

    #[test]
    fn volley_composition_across_every_tier_side_and_rear_combination() {
        for tier in 1u8..=4 {
            let expected_cannon: usize = match tier {
                1 => 1,
                2 => 2,
                3 => 3,
                4 => 5,
                _ => unreachable!(),
            };
            for has_side in [false, true] {
                for has_rear in [false, true] {
                    let mut loadout = Loadout {
                        cannon_tier: tier,
                        has_side,
                        has_rear,
                        drones: 0,
                        fire_cooldown: 0.0,
                        drone_cooldown: 0.0,
                    };
                    let mut shots = Vec::new();
                    let mut events = Vec::new();

                    update(&mut loadout, &mut shots, 1.0, 2.0, true, 0.01, &mut events);

                    let expected_total = expected_cannon
                        + if has_side { 2 } else { 0 }
                        + if has_rear { 1 } else { 0 };
                    assert_eq!(
                        shots.len(),
                        expected_total,
                        "tier={tier} side={has_side} rear={has_rear}"
                    );
                    assert_eq!(events.len(), expected_total);

                    // Every projectile in the volley: same damage, same
                    // speed magnitude, fired from the ship's origin.
                    for s in &shots {
                        assert_eq!(s.damage, CANNON_DAMAGE);
                        assert_eq!((s.x, s.y), (1.0, 2.0));
                        assert!((speed_sq(s) - PROJECTILE_SPEED * PROJECTILE_SPEED).abs() < 1e-1);
                    }

                    // The tier-1 forward shot is always present, unchanged.
                    assert!(shots
                        .iter()
                        .any(|s| s.vx == 0.0 && s.vy == -PROJECTILE_SPEED));

                    if has_side {
                        assert!(shots
                            .iter()
                            .any(|s| s.vx == -PROJECTILE_SPEED && s.vy == 0.0));
                        assert!(shots
                            .iter()
                            .any(|s| s.vx == PROJECTILE_SPEED && s.vy == 0.0));
                    }
                    if has_rear {
                        assert!(shots
                            .iter()
                            .any(|s| s.vx == 0.0 && s.vy == PROJECTILE_SPEED));
                    }
                }
            }
        }
    }

    #[test]
    fn drone_autofire_ignores_fire_held_and_fires_one_shot_per_drone_per_cadence() {
        let mut loadout = Loadout {
            cannon_tier: 1,
            has_side: false,
            has_rear: false,
            drones: 2,
            fire_cooldown: 0.0,
            drone_cooldown: 0.0,
        };
        let mut shots = Vec::new();
        let mut events = Vec::new();

        // fire_held = false: the cannon doesn't fire, but drones still do.
        update(&mut loadout, &mut shots, 5.0, 6.0, false, 0.01, &mut events);
        assert_eq!(shots.len(), 2); // one shot per drone
        assert_eq!(events.len(), 2);
        for s in &shots {
            assert_eq!(s.damage, DRONE_DAMAGE);
            assert_eq!((s.vx, s.vy), (0.0, -PROJECTILE_SPEED));
            assert_eq!((s.x, s.y), (5.0, 6.0));
        }
        // Cooldown decrement clamps at 0.0 before the fire check runs (same
        // pattern as the cannon's own cooldown), so firing from an
        // already-zero cooldown leaves it at exactly `DRONE_FIRE_INTERVAL`.
        assert!((loadout.drone_cooldown - DRONE_FIRE_INTERVAL).abs() < 1e-6);

        // Cooldown not yet elapsed: no new volley.
        update(&mut loadout, &mut shots, 5.0, 6.0, false, 0.01, &mut events);
        assert_eq!(shots.len(), 2);

        // Cooldown elapsed: next call fires another volley of `drones` shots.
        loadout.drone_cooldown = 0.0;
        update(&mut loadout, &mut shots, 5.0, 6.0, false, 0.01, &mut events);
        assert_eq!(shots.len(), 4);
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn zero_drones_never_autofire() {
        let mut loadout = Loadout::default(); // drones = 0
        let mut shots = Vec::new();
        let mut events = Vec::new();
        update(&mut loadout, &mut shots, 0.0, 0.0, false, 10.0, &mut events);
        assert!(shots.is_empty());
        assert!(events.is_empty());
    }
}
