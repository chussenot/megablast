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
    /// TODO(wave1 `player`): implement the movement itself.
    pub fn update(&mut self, _input: &super::Input, dt: f32) {
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
    /// TODO(wave1 `player`): fill in the life-loss/respawn branch using
    /// `INVULN_ON_RESPAWN` and `Self::new()`'s spawn position.
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
        false
    }
}
