//! Events emitted by the simulation each tick, describing what happened so
//! other layers (render juice now, audio later) can react without the
//! simulation depending on them. Same drain contract as arkanoid: `Game`
//! owns `pub events: Vec<GameEvent>`, `tick()` pushes onto it, the frame
//! loop in `main.rs` accumulates it into `frame_events` for the renderer
//! then clears it before the next frame.
//!
//! Owner: Wave 1 `events` task. This enum covers every audible/visual
//! moment named across all five milestones so downstream files never need
//! to add a variant later -- only construct/match the ones already here.
//! `src/events.rs` is a single-owner file (no other wave task may edit
//! it), so the shape below is meant to be final: every variant already
//! constructed elsewhere (`EnemyDamaged`, `EnemyDied`, `PlayerHit`,
//! `CreditCollected`, `LevelCleared`, `GameOver`, `Victory` -- see
//! `game/enemies.rs`, `game/player.rs`, `game/mod.rs`) keeps exactly its
//! current field set; variants no file constructs yet (`ShotFired`,
//! `BossHit`, `BossPatternChanged`, `PlayerDied`, `PlayerRespawned`,
//! `ShopItemBought`, `ShopItemSold`) were free to gain the fields their
//! future callers will already have on hand.
//!
//! Convention: where a moment has a natural on-screen location, the
//! payload carries `x`/`y` so a render-juice consumer never has to reach
//! back into `Game` state for it.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    // -- Weapons & projectiles (Wave 2/3 `weapons-*`) --
    /// Any weapon (main cannon, side, rear, drone) fired one shot from
    /// (x, y) -- drives muzzle-flash juice (Wave 5).
    ShotFired { x: f32, y: f32 },

    // -- Enemies & boss (Wave 1 `enemies-basic`, Wave 3 `enemies-advanced`) --
    /// An enemy took damage but did not die.
    EnemyDamaged { x: f32, y: f32 },
    /// An enemy died -- drives the shrinking-quads death juice (Wave 5)
    /// and is the trigger for `credit_value`'s pickup to spawn (Wave 4).
    /// `kind` lets audio/visuals distinguish all five enemy types (and
    /// the boss) on the same event.
    EnemyDied {
        x: f32,
        y: f32,
        kind: crate::levels::EnemyKind,
        credit_value: u32,
    },
    /// The boss took a hit this tick (pushed instead of `EnemyDamaged`
    /// for `EnemyKind::Boss`) -- drives screen shake (Wave 5).
    BossHit { x: f32, y: f32 },
    /// The boss switched attack patterns (aimed spread / wall-with-gap /
    /// spiral, 6s cycle) -- cue for a telegraph sting/flash.
    BossPatternChanged,

    // -- Player ship (Wave 1 `player`) --
    /// The player's ship took damage (contact or bullet).
    PlayerHit { x: f32, y: f32 },
    /// A life was actually spent (HP hit 0) -- `x`/`y` is where the ship
    /// died, for an explosion effect before it respawns.
    PlayerDied { x: f32, y: f32 },
    /// The ship respawned after a life was lost, at `x`/`y` (spawn point).
    PlayerRespawned { x: f32, y: f32 },

    // -- Economy (Wave 4 `drops-cash`, `shop`) --
    /// A falling credit pickup was collected; value already added to cash.
    CreditCollected { value: u32 },
    /// A shop purchase succeeded; `item` picks the confirmation sound/text.
    ShopItemBought { item: crate::game::shop::Item },
    /// A shop sale succeeded; `item` picks the confirmation sound/text.
    ShopItemSold { item: crate::game::shop::Item },

    // -- Level & game flow (bootstrap `game/mod.rs`) --
    /// The current level's boss was defeated -- transitions Playing -> Shop
    /// (or -> Victory after the last level).
    LevelCleared,
    /// Last life spent -- transitions Playing -> GameOver.
    GameOver,
    /// Final level's shop was left -- the run is won.
    Victory,
}
