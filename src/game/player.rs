//! Ship movement, hitbox, HP, invulnerability. No wgpu/winit types.
//!
//! Owner: Wave 1 `player` task. Shape (fields/methods) is fixed --
//! `game/mod.rs`'s collision handling already calls `hit()` and reads
//! `x`/`y`/`lives` -- implement the bodies below.

pub const PLAYER_RADIUS: f32 = 12.0;
pub const MAX_HP: f32 = 100.0;
pub const SPEED: f32 = 260.0; // px/s, diagonals normalized (spec)
const INVULN_ON_HIT: f32 = 1.5;
const INVULN_ON_RESPAWN: f32 = 2.0;
const STARTING_LIVES: u32 = 3;

#[derive(Debug, Clone, Copy)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub invuln_timer: f32,
    pub lives: u32,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            x: super::PLAYFIELD_WIDTH / 2.0,
            y: super::PLAYFIELD_HEIGHT - 100.0,
            hp: MAX_HP,
            invuln_timer: 0.0,
            lives: STARTING_LIVES,
        }
    }

    /// 8-directional movement at `SPEED` px/s (diagonals normalized so
    /// they're not faster than cardinal moves), clamped to the playfield,
    /// plus invulnerability-timer countdown (already correct below).
    pub fn update(&mut self, input: &super::Input, dt: f32) {
        let mut dx: f32 = 0.0;
        let mut dy: f32 = 0.0;
        if input.left {
            dx -= 1.0;
        }
        if input.right {
            dx += 1.0;
        }
        if input.up {
            dy -= 1.0;
        }
        if input.down {
            dy += 1.0;
        }
        if dx != 0.0 || dy != 0.0 {
            let len = (dx * dx + dy * dy).sqrt();
            self.x += dx / len * SPEED * dt;
            self.y += dy / len * SPEED * dt;
        }
        self.x = self
            .x
            .clamp(PLAYER_RADIUS, super::PLAYFIELD_WIDTH - PLAYER_RADIUS);
        self.y = self
            .y
            .clamp(PLAYER_RADIUS, super::PLAYFIELD_HEIGHT - PLAYER_RADIUS);

        if self.invuln_timer > 0.0 {
            self.invuln_timer = (self.invuln_timer - dt).max(0.0);
        }
    }

    /// Applies `dmg` unless currently invulnerable (gate below is already
    /// correct). Returns `true` if this call spent a life (HP hit 0) --
    /// on a life loss, resets HP/position to spawn and starts the 2s
    /// respawn invulnerability itself (lives/position are this struct's
    /// own fields per the module boundary); the caller (`game/mod.rs`)
    /// reacts by downgrading the weapon tier and checking Game Over.
    pub fn hit(&mut self, dmg: f32, events: &mut Vec<crate::events::GameEvent>) -> bool {
        if self.invuln_timer > 0.0 {
            return false;
        }
        self.invuln_timer = INVULN_ON_HIT;
        self.hp -= dmg;
        events.push(crate::events::GameEvent::PlayerHit {
            x: self.x,
            y: self.y,
        });
        if self.hp <= 0.0 {
            self.lives = self.lives.saturating_sub(1);
            let (death_x, death_y) = (self.x, self.y);
            let spawn = Self::new();
            self.x = spawn.x;
            self.y = spawn.y;
            self.hp = spawn.hp;
            self.invuln_timer = INVULN_ON_RESPAWN;
            events.push(crate::events::GameEvent::PlayerDied {
                x: death_x,
                y: death_y,
            });
            events.push(crate::events::GameEvent::PlayerRespawned {
                x: self.x,
                y: self.y,
            });
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEvent;
    use crate::game::Input;

    #[test]
    fn diagonal_movement_does_not_exceed_speed() {
        let mut p = Player::new();
        let start = (p.x, p.y);
        let input = Input {
            up: true,
            right: true,
            ..Default::default()
        };
        p.update(&input, 1.0);
        let dist = ((p.x - start.0).powi(2) + (p.y - start.1).powi(2)).sqrt();
        assert!(
            dist <= SPEED + 1e-4,
            "diagonal move covered {dist}px, expected <= {SPEED}px"
        );
    }

    #[test]
    fn position_clamps_at_playfield_edges() {
        let mut p = Player::new();
        p.x = 0.0;
        p.y = 0.0;
        let input = Input {
            left: true,
            up: true,
            ..Default::default()
        };
        p.update(&input, 10.0);
        assert_eq!(p.x, PLAYER_RADIUS);
        assert_eq!(p.y, PLAYER_RADIUS);

        p.x = super::super::PLAYFIELD_WIDTH;
        p.y = super::super::PLAYFIELD_HEIGHT;
        let input = Input {
            right: true,
            down: true,
            ..Default::default()
        };
        p.update(&input, 10.0);
        assert_eq!(p.x, super::super::PLAYFIELD_WIDTH - PLAYER_RADIUS);
        assert_eq!(p.y, super::super::PLAYFIELD_HEIGHT - PLAYER_RADIUS);
    }

    #[test]
    fn hit_is_noop_while_invulnerable() {
        let mut p = Player::new();
        p.invuln_timer = 1.0;
        let hp_before = p.hp;
        let mut events = Vec::new();
        let died = p.hit(50.0, &mut events);
        assert!(!died);
        assert_eq!(p.hp, hp_before);
        assert!(events.is_empty());
    }

    #[test]
    fn hit_to_zero_hp_costs_one_life_and_respawns() {
        let mut p = Player::new();
        p.lives = 3;
        p.hp = 10.0;
        p.x = 5.0;
        p.y = 5.0;
        let mut events = Vec::new();
        let died = p.hit(50.0, &mut events);
        let spawn = Player::new();

        assert!(died);
        assert_eq!(p.lives, 2);
        assert_eq!(p.hp, spawn.hp);
        assert_eq!(p.x, spawn.x);
        assert_eq!(p.y, spawn.y);
        assert_eq!(p.invuln_timer, INVULN_ON_RESPAWN);
        assert_eq!(
            events,
            vec![
                GameEvent::PlayerHit { x: 5.0, y: 5.0 },
                GameEvent::PlayerDied { x: 5.0, y: 5.0 },
                GameEvent::PlayerRespawned {
                    x: spawn.x,
                    y: spawn.y,
                },
            ]
        );
    }
}
