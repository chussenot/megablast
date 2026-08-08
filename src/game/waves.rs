//! Deterministic spawn scheduler: consumes a `levels::LevelScript` in
//! order, spawning each `Wave`'s enemies once `scroll_y` passes its
//! trigger. Same script + same seed => identical spawn sequence (spec:
//! assert this in a test).
//!
//! Owner: Wave 2 `waves-scheduler` task. `game/mod.rs` already owns a
//! `Scheduler` and calls `update` every tick -- keep this shape.

use crate::levels::{EntryPattern, LevelScript};

pub struct Scheduler {
    pub script: LevelScript,
    next_index: usize,
}

/// Fixed vertical offset above the playfield top every wave enters from
/// -- comfortably outside the visible field so nothing pops in already
/// on-screen; enemy movement (`enemies::update`) is what carries them
/// down into view as the game scrolls.
const SPAWN_Y: f32 = -60.0;

/// Horizontal center of the playfield -- every pattern below is built
/// centered on this so a wave reads as one intentional formation rather
/// than drifting off to one side.
const CENTER_X: f32 = super::PLAYFIELD_WIDTH / 2.0;

/// Deterministic spawn positions for `count` enemies laid out per
/// `pattern`, spaced by `spacing`. No randomness anywhere: same
/// `(pattern, count, spacing)` always produces the same positions in the
/// same order.
fn layout(pattern: EntryPattern, count: u32, spacing: f32) -> Vec<(f32, f32)> {
    let n = count as usize;
    (0..n)
        .map(|i| {
            // Centered index: 0 for the middle entry, symmetric either
            // side, whether `count` is odd or even.
            let centered = i as f32 - (n as f32 - 1.0) / 2.0;
            match pattern {
                // Evenly spaced horizontally at a fixed y above the top.
                EntryPattern::Line => (CENTER_X + centered * spacing, SPAWN_Y),
                // Single x, stacked vertically above the top so they
                // enter the playfield one after another.
                EntryPattern::Column => (CENTER_X, SPAWN_Y - i as f32 * spacing),
                // V-shape: the middle entry is closest to the playfield
                // (the point of the V), and entries further from center
                // sit both further out horizontally and further back
                // vertically -- the two arms converge on the center.
                EntryPattern::V => (
                    CENTER_X + centered * spacing,
                    SPAWN_Y - centered.abs() * spacing,
                ),
            }
        })
        .collect()
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
    pub fn update(
        &mut self,
        scroll_y: f32,
        enemies: &mut Vec<super::enemies::Enemy>,
        player_x: f32,
        player_y: f32,
    ) {
        while self.next_index < self.script.waves.len() {
            let (trigger_scroll_y, wave) = &self.script.waves[self.next_index];
            if *trigger_scroll_y > scroll_y {
                break;
            }
            for (x, y) in layout(wave.pattern, wave.count, wave.spacing) {
                enemies.push(super::enemies::spawn(wave.enemy, x, y, player_x, player_y));
            }
            self.next_index += 1;
        }
    }

    pub fn is_done(&self) -> bool {
        self.next_index >= self.script.waves.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::enemies::Enemy;
    use crate::levels::{EnemyKind, LevelScript, Wave};

    fn test_script() -> LevelScript {
        LevelScript {
            scroll_speed: 40.0,
            waves: vec![
                (
                    100.0,
                    Wave {
                        enemy: EnemyKind::Popcorn,
                        count: 3,
                        pattern: EntryPattern::Line,
                        spacing: 40.0,
                    },
                ),
                (
                    250.0,
                    Wave {
                        enemy: EnemyKind::Diver,
                        count: 2,
                        pattern: EntryPattern::V,
                        spacing: 50.0,
                    },
                ),
                (
                    400.0,
                    Wave {
                        enemy: EnemyKind::Turret,
                        count: 4,
                        pattern: EntryPattern::Column,
                        spacing: 30.0,
                    },
                ),
            ],
        }
    }

    /// Runs a scheduler for `test_script()` across `scroll_ys` in order,
    /// returning the resulting spawned enemies for comparison.
    fn run(scroll_ys: &[f32]) -> Vec<Enemy> {
        let mut scheduler = Scheduler::new(test_script());
        let mut enemies = Vec::new();
        for &scroll_y in scroll_ys {
            scheduler.update(scroll_y, &mut enemies, 300.0, 700.0);
        }
        enemies
    }

    #[test]
    fn update_is_deterministic_across_two_identical_runs() {
        let scroll_ys = [0.0, 50.0, 100.0, 100.0, 180.0, 250.0, 300.0, 400.0, 500.0];

        let first = run(&scroll_ys);
        let second = run(&scroll_ys);

        assert_eq!(first.len(), second.len());
        assert!(!first.is_empty());
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
        }
    }

    #[test]
    fn spawns_never_happen_before_their_trigger_scroll_y() {
        let mut scheduler = Scheduler::new(test_script());
        let mut enemies = Vec::new();

        // Below every trigger: nothing spawns yet.
        scheduler.update(50.0, &mut enemies, 300.0, 700.0);
        assert!(enemies.is_empty());

        // Reaches exactly the first trigger: spawns at/after, not before.
        scheduler.update(100.0, &mut enemies, 300.0, 700.0);
        assert_eq!(enemies.len(), 3);
        assert!(enemies.iter().all(|e| e.kind == EnemyKind::Popcorn));

        // Jump straight past the remaining two triggers at once -- both
        // fire, in script order.
        scheduler.update(500.0, &mut enemies, 300.0, 700.0);
        assert_eq!(enemies.len(), 3 + 2 + 4);
        assert!(scheduler.is_done());
    }
}
