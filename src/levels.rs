//! Level data: scroll script + wave script, as const/static data consumed
//! deterministically by `game::waves::Scheduler`. No wgpu/winit types.
//!
//! Owners: the shapes below are fixed -- `game::waves` and `game::enemies`
//! already depend on them; do not change field names/types. Wave 3
//! `levels-level1` (this task) supplies the full level 1 teaching script.
//! Wave 4 `levels-level2-bosses` adds level 2 and boss pattern variants.
//! Each subsequent task only edits `level()`'s data, never these type
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

/// Level 1: the full teaching script (spec: "level 1 teaches
/// popcorn/diver, sparse turrets ... each ending in the boss"). Ramps
/// gently -- a couple of approachable Popcorn/Diver waves, then a
/// couple of widely-spaced, low-count Turret waves to introduce aimed
/// fire without overwhelming, then a single Boss wave once every
/// earlier wave has had room to clear. Triggers are spaced 300-400px
/// apart so waves don't overlap.
fn level_1() -> LevelScript {
    LevelScript {
        scroll_speed: 40.0,
        waves: vec![
            (
                300.0,
                Wave {
                    enemy: EnemyKind::Popcorn,
                    count: 5,
                    pattern: EntryPattern::Line,
                    spacing: 40.0,
                },
            ),
            (
                650.0,
                Wave {
                    enemy: EnemyKind::Diver,
                    count: 3,
                    pattern: EntryPattern::V,
                    spacing: 60.0,
                },
            ),
            (
                1000.0,
                Wave {
                    enemy: EnemyKind::Popcorn,
                    count: 6,
                    pattern: EntryPattern::Line,
                    spacing: 45.0,
                },
            ),
            (
                1350.0,
                Wave {
                    enemy: EnemyKind::Diver,
                    count: 4,
                    pattern: EntryPattern::V,
                    spacing: 55.0,
                },
            ),
            (
                // Sparse: only 2 turrets, spread far apart -- the
                // player's first taste of aimed fire, kept simple.
                1700.0,
                Wave {
                    enemy: EnemyKind::Turret,
                    count: 2,
                    pattern: EntryPattern::Column,
                    spacing: 220.0,
                },
            ),
            (
                2100.0,
                Wave {
                    enemy: EnemyKind::Popcorn,
                    count: 5,
                    pattern: EntryPattern::Line,
                    spacing: 40.0,
                },
            ),
            (
                // Second sparse turret wave: still low-count and
                // widely spaced, one more than the first.
                2450.0,
                Wave {
                    enemy: EnemyKind::Turret,
                    count: 3,
                    pattern: EntryPattern::Line,
                    spacing: 260.0,
                },
            ),
            (
                2800.0,
                Wave {
                    enemy: EnemyKind::Diver,
                    count: 5,
                    pattern: EntryPattern::V,
                    spacing: 50.0,
                },
            ),
            (
                // Boss: a single entity, so pattern/spacing are moot
                // (any `EntryPattern` with `count: 1` places it dead
                // center) -- pick Line for clarity. High trigger so
                // every earlier wave has cleared first.
                3200.0,
                Wave {
                    enemy: EnemyKind::Boss,
                    count: 1,
                    pattern: EntryPattern::Line,
                    spacing: 0.0,
                },
            ),
        ],
    }
}

/// Placeholder for Wave 4 `levels-level2-bosses`.
fn level_2() -> LevelScript {
    LevelScript {
        scroll_speed: 40.0,
        waves: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_1_teaches_popcorn_diver_and_sparse_turrets_then_ends_in_boss() {
        let script = level_1();
        let kinds: Vec<EnemyKind> = script.waves.iter().map(|(_, w)| w.enemy).collect();

        assert!(kinds.contains(&EnemyKind::Popcorn));
        assert!(kinds.contains(&EnemyKind::Diver));
        let turret_waves: Vec<_> = script
            .waves
            .iter()
            .filter(|(_, w)| w.enemy == EnemyKind::Turret)
            .collect();
        // "Sparse" turrets: low count, widely spaced, and there's more
        // than one such wave partway through the level.
        assert!(turret_waves.len() >= 2);
        for (_, w) in &turret_waves {
            assert!(w.count <= 3);
            assert!(w.spacing >= 200.0);
        }

        let (_, last_wave) = script.waves.last().expect("level 1 has waves");
        assert_eq!(last_wave.enemy, EnemyKind::Boss);
        assert_eq!(last_wave.count, 1);
    }

    #[test]
    fn level_1_triggers_are_strictly_increasing_and_well_spaced() {
        let script = level_1();
        for window in script.waves.windows(2) {
            let (a, _) = &window[0];
            let (b, _) = &window[1];
            assert!(b > a, "triggers must be in increasing order");
            assert!(
                b - a >= 200.0,
                "waves should be spaced far enough apart not to overlap"
            );
        }
    }
}
