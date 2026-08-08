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
    /// Which of the shop menu's 7 slots is highlighted (0..7, wrapping)
    /// -- slots 0..6 are Cannon/SideShots/RearShot/Drone/Repair/
    /// ExtraLife, slot 6 is a "Leave" sentinel with no price --
    /// `pub` because `render/hud.rs`'s shop screen (Wave 4
    /// `shop-wiring`) needs to know what to highlight and mirrors this
    /// exact 7-slot shape. Only `tick_shop` writes it.
    pub shop_cursor: usize,
    scheduler: waves::Scheduler,
    pause_was_held: bool,
    /// Shop menu's own edge-detection state: the input seen on the
    /// previous `tick_shop` call, so held keys don't repeat every tick
    /// and a key already held from the fight that just ended (fire to
    /// kill the boss, arrows to dodge) can't read as a fresh press on
    /// the shop's first tick. `None` means "just entered the shop,
    /// seed only" -- reset there by `tick_shop` on every leave.
    shop_prev_input: Option<Input>,
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
            shop_cursor: 0,
            scheduler: waves::Scheduler::new(levels::level(1)),
            pause_was_held: false,
            shop_prev_input: None,
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
        let scroll_dy = self.scheduler.script.scroll_speed * dt;
        self.scroll_y += scroll_dy;

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
            scroll_dy,
            self.player.x,
            self.player.y,
            dt,
            &mut self.events,
        );
        enemies::advance_bullets(&mut self.enemy_bullets, dt);
        self.enemy_bullets.retain(|b| {
            b.y > -32.0
                && b.y < PLAYFIELD_HEIGHT + 32.0
                && b.x > -32.0
                && b.x < PLAYFIELD_WIDTH + 32.0
        });
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

    /// Shop menu (Wave 4 `shop-wiring`): `shop_cursor` cycles over
    /// `SHOP_SLOTS` slots -- slots `0..SHOP_ITEMS.len()` are the six
    /// purchasable items (`SHOP_ITEMS`, same order as the spec); the
    /// last slot is a "Leave" sentinel with no price. `render/hud.rs`
    /// mirrors this exact slot count when it draws the shop screen, so
    /// the two files must stay in sync if this ever changes.
    ///
    /// Left/right cycle the cursor; fire buys the highlighted item, or
    /// (on the Leave sentinel) leaves the shop -- starting the next
    /// level, or ending the game in Victory after the last one; up or
    /// down sell the highlighted item. Deliberately never reads
    /// `input.pause`: `debounce_pause` unconditionally re-checks it
    /// right after this returns every tick, so touching it here would
    /// double-fire one physical press into two meanings in the same
    /// tick.
    fn tick_shop(&mut self, input: &Input) {
        const SHOP_ITEMS: [shop::Item; 6] = [
            shop::Item::Cannon,
            shop::Item::SideShots,
            shop::Item::RearShot,
            shop::Item::Drone,
            shop::Item::Repair,
            shop::Item::ExtraLife,
        ];
        const LEAVE_SLOT: usize = SHOP_ITEMS.len();
        const SHOP_SLOTS: usize = SHOP_ITEMS.len() + 1;

        // Seed edge-detection on the shop's first tick (see
        // `shop_prev_input`'s doc comment) instead of acting on it.
        let Some(prev) = self.shop_prev_input.replace(*input) else {
            return;
        };

        if input.left && !prev.left {
            self.shop_cursor = (self.shop_cursor + SHOP_SLOTS - 1) % SHOP_SLOTS;
        }
        if input.right && !prev.right {
            self.shop_cursor = (self.shop_cursor + 1) % SHOP_SLOTS;
        }

        if input.fire && !prev.fire {
            if self.shop_cursor == LEAVE_SLOT {
                self.shop_prev_input = None;
                if self.level < TOTAL_LEVELS {
                    self.level += 1;
                    self.scheduler = waves::Scheduler::new(levels::level(self.level));
                    self.state = GameState::Playing;
                } else {
                    self.state = GameState::Victory;
                    self.events.push(GameEvent::Victory);
                }
            } else {
                let item = SHOP_ITEMS[self.shop_cursor];
                let bought = shop::buy(
                    &mut self.shop,
                    &mut self.loadout,
                    &mut self.player.lives,
                    &mut self.player.hp,
                    player::MAX_HP,
                    item,
                )
                .is_ok();
                if bought {
                    self.events.push(GameEvent::ShopItemBought { item });
                }
            }
            return;
        }

        let sell_now = input.up || input.down;
        let sell_prev = prev.up || prev.down;
        if sell_now && !sell_prev && self.shop_cursor != LEAVE_SLOT {
            let item = SHOP_ITEMS[self.shop_cursor];
            if shop::sell(&mut self.shop, &mut self.loadout, item).is_ok() {
                self.events.push(GameEvent::ShopItemSold { item });
            }
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
                        b.damage as f32,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::EnemyKind;

    /// The subset of `Game` state this replay test can promise is
    /// bit-for-bit identical across two runs of the same scripted input.
    ///
    /// Deliberately excludes `pickups` and `shop.cash`. `enemies::maybe_drop`
    /// (`src/game/enemies.rs`, not owned by this task -- see mb-epic.23,
    /// already filed for the real fix of threading a seeded RNG through)
    /// rolls the global, unseeded `rand::random()` on every enemy death to
    /// decide whether it drops a pickup. That roll is the *only* place
    /// randomness enters `Game::tick`, so it -- and, downstream of it,
    /// `shop.cash`, which is fed exclusively by collecting those pickups --
    /// are the only state not reproducible across two otherwise-identical
    /// runs. `score` is unaffected: it comes from the fixed `EnemyDied`
    /// event `enemies::apply_damage` always pushes on a kill, computed and
    /// pushed before `maybe_drop` is ever called for that death.
    #[derive(Debug, PartialEq)]
    struct Snapshot {
        state: GameState,
        player_x: f32,
        player_y: f32,
        player_hp: f32,
        player_lives: u32,
        score: u32,
        level: usize,
        scroll_y: f32,
        enemy_positions: Vec<(EnemyKind, f32, f32)>,
    }

    fn snapshot(game: &Game) -> Snapshot {
        Snapshot {
            state: game.state,
            player_x: game.player.x,
            player_y: game.player.y,
            player_hp: game.player.hp,
            player_lives: game.player.lives,
            score: game.score,
            level: game.level,
            scroll_y: game.scroll_y,
            enemy_positions: game.enemies.iter().map(|e| (e.kind, e.x, e.y)).collect(),
        }
    }

    /// Purely a function of the tick index -- no wall-clock time, no
    /// randomness -- so replaying it from a fresh `Game::new()` reproduces
    /// the exact same sequence of `Input`s every time. Fire is held the
    /// whole run (the cannon keeps firing); the ship bobs up/down on a
    /// fixed cadence but never strafes off the vertical line level 1's
    /// first (centered) Popcorn wave spawns on, so the volley keeps
    /// intersecting it as it falls and drifts back through center.
    fn scripted_input(tick: usize) -> Input {
        Input {
            up: tick % 40 < 20,
            down: tick % 40 >= 20,
            left: false,
            right: false,
            fire: true,
            pause: false,
        }
    }

    fn run_script(ticks: usize) -> Game {
        let mut game = Game::new();
        let dt = 1.0 / 120.0;
        for tick in 0..ticks {
            game.tick(&scripted_input(tick), dt);
        }
        game
    }

    #[test]
    fn deterministic_replay_same_input_script_produces_identical_state_twice() {
        // Long enough to leave the menu, scroll past level 1's first
        // wave's trigger (a centered Popcorn line at scroll_y 300), and
        // have the continuously-firing cannon kill some of it.
        let ticks = 3000;

        let a = run_script(ticks);
        let b = run_script(ticks);

        // Sanity: the script actually drove the game somewhere -- not two
        // blank starts trivially equal to each other.
        assert_eq!(a.state, GameState::Playing);
        assert!(a.scroll_y > 0.0);
        assert!(
            a.score > 0,
            "expected the scripted volley to kill at least one enemy by tick {ticks}"
        );

        assert_eq!(snapshot(&a), snapshot(&b));
    }
}
