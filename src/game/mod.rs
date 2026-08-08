//! `Game`: top-level simulation state, the fixed-tick `tick()` entry
//! point, and the explicit state machine
//! (`Menu -> Playing <-> Paused -> Shop -> Playing... -> GameOver/Victory`).
//! No wgpu/winit/IO types anywhere under `src/game/` -- headless-testable.
//!
//! Wiring/state-machine owner: bootstrap. This file's `mod` layout,
//! struct shapes, and `tick()` dispatch are fixed for the whole build --
//! no wave task edits this file. Every wave implements the real body of
//! the functions it calls, inside the files it owns.

pub mod enemies;
pub mod player;
pub mod shop;
pub mod weapons;

mod collide;
mod waves;

use crate::events::GameEvent;
use crate::levels;

/// Logical portrait playfield (spec: 600x800, letterboxed -- never scaled
/// in simulation space).
pub const PLAYFIELD_WIDTH: f32 = 600.0;
pub const PLAYFIELD_HEIGHT: f32 = 800.0;

const CONTACT_DAMAGE: f32 = 25.0;
const BULLET_DAMAGE: f32 = 10.0;
const TOTAL_LEVELS: usize = 2;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
    pub pause: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    Paused,
    Shop,
    GameOver,
    Victory,
}

pub struct Game {
    /// Drained every tick by the frame loop in `main.rs` -- see
    /// `events.rs` for the emission/drain contract.
    pub events: Vec<GameEvent>,
    pub state: GameState,
    pub player: player::Player,
    pub loadout: weapons::Loadout,
    pub enemies: Vec<enemies::Enemy>,
    pub player_shots: Vec<weapons::Shot>,
    pub enemy_bullets: Vec<enemies::Bullet>,
    pub pickups: Vec<enemies::Pickup>,
    pub shop: shop::Shop,
    pub score: u32,
    pub level: usize,
    pub scroll_y: f32,
    scheduler: waves::Scheduler,
    pause_was_held: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            state: GameState::Menu,
            player: player::Player::new(),
            loadout: weapons::Loadout::default(),
            enemies: Vec::new(),
            player_shots: Vec::new(),
            enemy_bullets: Vec::new(),
            pickups: Vec::new(),
            shop: shop::Shop::default(),
            score: 0,
            level: 1,
            scroll_y: 0.0,
            scheduler: waves::Scheduler::new(levels::level(1)),
            pause_was_held: false,
        }
    }

    /// Advance the simulation by one fixed timestep. Dispatches on
    /// `state` first -- the explicit state machine the spec asks for.
    pub fn tick(&mut self, input: &Input, dt: f32) {
        match self.state {
            GameState::Menu => self.tick_menu(input),
            GameState::Playing => self.tick_playing(input, dt),
            GameState::Paused => {}
            GameState::Shop => self.tick_shop(input),
            GameState::GameOver | GameState::Victory => {}
        }
        self.debounce_pause(input);
    }

    /// Edge-detects Pause so a held key doesn't toggle every tick (same
    /// pattern as arkanoid's `pause_was_held`).
    fn debounce_pause(&mut self, input: &Input) {
        if input.pause && !self.pause_was_held {
            self.state = match self.state {
                GameState::Playing => GameState::Paused,
                GameState::Paused => GameState::Playing,
                other => other,
            };
        }
        self.pause_was_held = input.pause;
    }

    fn tick_menu(&mut self, input: &Input) {
        if input.fire {
            self.state = GameState::Playing;
        }
    }

    fn tick_playing(&mut self, input: &Input, dt: f32) {
        self.scroll_y += self.scheduler.script.scroll_speed * dt;

        self.player.update(input, dt);
        weapons::update(
            &mut self.loadout,
            &mut self.player_shots,
            self.player.x,
            self.player.y,
            input.fire,
            dt,
            &mut self.events,
        );
        weapons::advance_shots(&mut self.player_shots, dt);
        self.player_shots
            .retain(|s| s.y > -32.0 && s.y < PLAYFIELD_HEIGHT + 32.0);

        enemies::update(
            &mut self.enemies,
            &mut self.enemy_bullets,
            self.scroll_y,
            self.player.x,
            self.player.y,
            dt,
            &mut self.events,
        );
        enemies::advance_pickups(&mut self.pickups, dt);
        self.pickups.retain(|p| p.y < PLAYFIELD_HEIGHT + 32.0);

        self.scheduler.update(
            self.scroll_y,
            &mut self.enemies,
            self.player.x,
            self.player.y,
        );

        self.resolve_collisions();
        self.collect_pickups();
        self.check_level_cleared();
    }

    /// Real menu navigation/buy-sell input handling is Wave 4
    /// `shop-wiring`'s job -- extend this function (this file stays
    /// bootstrap-owned otherwise; see docs/megablast.md Milestone 4).
    /// Leaving the shop starts the next level, or ends the game in
    /// Victory after the last one.
    fn tick_shop(&mut self, input: &Input) {
        if !input.fire {
            return;
        }
        if self.level < TOTAL_LEVELS {
            self.level += 1;
            self.scheduler = waves::Scheduler::new(levels::level(self.level));
            self.state = GameState::Playing;
        } else {
            self.state = GameState::Victory;
            self.events.push(GameEvent::Victory);
        }
    }

    fn resolve_collisions(&mut self) {
        // Player shots vs enemies.
        for shot_idx in (0..self.player_shots.len()).rev() {
            let shot = self.player_shots[shot_idx];
            let mut hit_idx = None;
            for (i, e) in self.enemies.iter().enumerate() {
                if collide::circle_circle(
                    shot.x,
                    shot.y,
                    weapons::SHOT_RADIUS,
                    e.x,
                    e.y,
                    enemies::RADIUS,
                ) {
                    hit_idx = Some(i);
                    break;
                }
            }
            let Some(i) = hit_idx else { continue };
            self.player_shots.remove(shot_idx);
            if let Some(death) =
                enemies::apply_damage(&mut self.enemies, i, shot.damage as i32, &mut self.events)
            {
                self.score += enemies::score_value(death.kind);
                if let Some(p) =
                    enemies::maybe_drop(death.kind, death.x, death.y, death.credit_value)
                {
                    self.pickups.push(p);
                }
            }
        }

        // Enemy bullets vs player (drones absorb one bullet each first).
        for bi in (0..self.enemy_bullets.len()).rev() {
            let b = self.enemy_bullets[bi];
            if collide::circle_circle(
                b.x,
                b.y,
                enemies::BULLET_RADIUS,
                self.player.x,
                self.player.y,
                player::PLAYER_RADIUS,
            ) {
                self.enemy_bullets.remove(bi);
                if self.loadout.drones > 0 {
                    self.loadout.drones -= 1;
                } else {
                    Self::apply_player_hit(
                        &mut self.player,
                        &mut self.loadout,
                        &mut self.state,
                        BULLET_DAMAGE,
                        &mut self.events,
                    );
                }
            }
        }

        // Enemy contact vs player.
        let mut contact = false;
        for e in self.enemies.iter() {
            if collide::circle_circle(
                e.x,
                e.y,
                enemies::RADIUS,
                self.player.x,
                self.player.y,
                player::PLAYER_RADIUS,
            ) {
                contact = true;
                break;
            }
        }
        if contact {
            Self::apply_player_hit(
                &mut self.player,
                &mut self.loadout,
                &mut self.state,
                CONTACT_DAMAGE,
                &mut self.events,
            );
        }
    }

    /// Damages the player and reacts to a life being spent (spec:
    /// downgrade one weapon tier, Game Over at 0 lives). A free function
    /// taking split borrows so callers don't need `&mut self` twice.
    fn apply_player_hit(
        player: &mut player::Player,
        loadout: &mut weapons::Loadout,
        state: &mut GameState,
        dmg: f32,
        events: &mut Vec<GameEvent>,
    ) {
        let life_lost = player.hit(dmg, events);
        if life_lost {
            weapons::downgrade(loadout);
            if player.lives == 0 {
                *state = GameState::GameOver;
                events.push(GameEvent::GameOver);
            }
        }
    }

    fn collect_pickups(&mut self) {
        let mut collected = 0u32;
        let mut i = 0;
        while i < self.pickups.len() {
            let p = self.pickups[i];
            if collide::circle_circle(
                p.x,
                p.y,
                enemies::PICKUP_RADIUS,
                self.player.x,
                self.player.y,
                player::PLAYER_RADIUS,
            ) {
                collected += p.value;
                self.pickups.remove(i);
            } else {
                i += 1;
            }
        }
        if collected > 0 {
            self.shop.cash += collected;
            self.events
                .push(GameEvent::CreditCollected { value: collected });
        }
    }

    fn check_level_cleared(&mut self) {
        let boss_was_spawned = self
            .scheduler
            .script
            .waves
            .iter()
            .any(|(_, w)| w.enemy == levels::EnemyKind::Boss);
        let boss_alive = self
            .enemies
            .iter()
            .any(|e| e.kind == levels::EnemyKind::Boss);
        if boss_was_spawned && !boss_alive && self.scheduler.is_done() {
            self.state = GameState::Shop;
            self.events.push(GameEvent::LevelCleared);
        }
    }
}
