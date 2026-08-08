//! Events emitted by the simulation each tick, describing what happened so
//! other layers (render juice now, audio later) can react without the
//! simulation depending on them. Same drain contract as arkanoid: `Game`
//! owns `pub events: Vec<GameEvent>`, `tick()` pushes onto it, the frame
//! loop in `main.rs` accumulates it into `frame_events` for the renderer
//! then clears it before the next frame.
//!
//! Owner: Wave 1 `events` task. This draft covers every audible/visual
//! moment named across all five milestones so downstream files never need
//! to add a variant later -- only construct/match the ones already here.
//! Refine names/fields/docs freely; do not remove a variant another wave's
//! task description already depends on without checking `docs/megablast.md`.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    /// Any weapon (main cannon, side, rear, drone) fired one shot from
    /// (x, y) -- drives muzzle-flash juice (Wave 5).
    ShotFired {
        x: f32,
        y: f32,
    },
    /// An enemy took damage but did not die.
    EnemyDamaged {
        x: f32,
        y: f32,
    },
    /// An enemy died -- drives the shrinking-quads death juice (Wave 5)
    /// and is the trigger for `credit_value`'s pickup to spawn (Wave 4).
    EnemyDied {
        x: f32,
        y: f32,
        kind: crate::levels::EnemyKind,
        credit_value: u32,
    },
    /// The boss took damage this tick -- drives screen shake (Wave 5).
    BossHit {
        x: f32,
        y: f32,
    },
    /// The boss switched attack patterns (6s cycle).
    BossPatternChanged,
    /// The player's ship took damage (contact or bullet).
    PlayerHit {
        x: f32,
        y: f32,
    },
    /// A life was actually spent (HP hit 0).
    PlayerDied,
    /// The ship respawned after a life was lost.
    PlayerRespawned,
    /// A falling credit pickup was collected; value already added to cash.
    CreditCollected {
        value: u32,
    },
    /// The current level's boss was defeated -- transitions Playing -> Shop
    /// (or -> Victory after the last level).
    LevelCleared,
    ShopItemBought,
    ShopItemSold,
    GameOver,
    Victory,
}
