//! Deterministic spawn scheduler: consumes a `levels::LevelScript` in
//! order, spawning each `Wave`'s enemies once `scroll_y` passes its
//! trigger. Same script + same seed => identical spawn sequence (spec:
//! assert this in a test).
//!
//! Owner: Wave 2 `waves-scheduler` task. `game/mod.rs` already owns a
//! `Scheduler` and calls `update` every tick -- keep this shape.

use crate::levels::LevelScript;

pub struct Scheduler {
    pub script: LevelScript,
    next_index: usize,
}

impl Scheduler {
    pub fn new(script: LevelScript) -> Self {
        Self {
            script,
            next_index: 0,
        }
    }

    /// Spawns every wave whose trigger `scroll_y` has now been reached,
    /// in script order, appending to `enemies` via `enemies::spawn`.
    ///
    /// TODO(wave2 `waves-scheduler`): iterate
    /// `self.script.waves[self.next_index..]`, spawning each wave (per
    /// its `count`/`pattern`/`spacing`) whose trigger `<= scroll_y`,
    /// advancing `next_index` past it.
    pub fn update(
        &mut self,
        scroll_y: f32,
        enemies: &mut Vec<super::enemies::Enemy>,
        player_x: f32,
        player_y: f32,
    ) {
        let _ = (scroll_y, enemies, player_x, player_y);
    }

    pub fn is_done(&self) -> bool {
        self.next_index >= self.script.waves.len()
    }
}
