//! Level data: scroll script + wave script, as const/static data consumed
//! deterministically by `game::waves::Scheduler`. No wgpu/winit types.
//!
//! Owners: Wave 1 `levels-data` task fixes the shapes below (do not change
//! field names/types -- `game::waves` and `game::enemies` already depend on
//! them) and writes a real, playable level 1. Wave 3 `levels-level1`
//! replaces level 1 with the full teaching script. Wave 4
//! `levels-level2-bosses` adds level 2 and boss pattern variants. Each
//! subsequent task only edits `LEVELS`'/`level()`'s data, never these type
//! shapes.

/// The five enemy types, shared by `levels` (to script waves) and
/// `game::enemies` (to implement behavior) -- defined here so `levels.rs`
/// doesn't have to wait on `game::enemies` to exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Popcorn,
    Diver,
    Turret,
    Weaver,
    Boss,
}

/// How a wave's enemies enter the playfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPattern {
    Line,
    Column,
    V,
}

/// One scripted group of enemies.
#[derive(Debug, Clone)]
pub struct Wave {
    pub enemy: EnemyKind,
    pub count: u32,
    pub pattern: EntryPattern,
    /// Pixels between entries along the pattern.
    pub spacing: f32,
}

/// A level: background scroll speed plus `(trigger_scroll_y, wave)`
/// entries consumed in order as `scroll_y` advances past each trigger.
#[derive(Debug, Clone)]
pub struct LevelScript {
    /// Base scroll speed, px/s (spec default 40.0).
    pub scroll_speed: f32,
    pub waves: Vec<(f32, Wave)>,
}

/// 1-based level lookup (`level(1)`, `level(2)`); panics outside that range
/// -- `Game` never advances `self.level` past the last one (see
/// `game::mod`'s Shop -> Playing / -> Victory transition).
pub fn level(n: usize) -> LevelScript {
    match n {
        1 => level_1(),
        2 => level_2(),
        _ => panic!("levels::level: no level {n}"),
    }
}

/// Placeholder until Wave 1 fills in a real teaching script: scroll only,
/// no waves yet, so Milestone 1's "ship on an empty scrolling starfield"
/// has real scroll data to render against.
fn level_1() -> LevelScript {
    LevelScript {
        scroll_speed: 40.0,
        waves: Vec::new(),
    }
}

/// Placeholder for Wave 4 `levels-level2-bosses`.
fn level_2() -> LevelScript {
    LevelScript {
        scroll_speed: 40.0,
        waves: Vec::new(),
    }
}
