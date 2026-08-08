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

/// Level 2: the escalation script (spec: "level 2 escalates (weavers,
/// dense turrets) ... each ending in the boss"). Introduces Weaver
/// (absent from level 1) and makes Turret genuinely dense: higher
/// counts per wave (4-7, vs level 1's 2-3) laid out with tight entry
/// spacing (70-100px, vs level 1's 220-260px "sparse" spread), and the
/// waves themselves trigger closer together (210-230px gaps, vs level
/// 1's 350-400px) -- still respecting the same >= 200px no-overlap
/// floor level 1 used, so nothing stacks on top of the previous wave.
/// Ends in a Boss wave with generous clearing room after the last
/// regular wave.
fn level_2() -> LevelScript {
    LevelScript {
        scroll_speed: 40.0,
        waves: vec![
            (
                200.0,
                Wave {
                    enemy: EnemyKind::Weaver,
                    count: 4,
                    pattern: EntryPattern::V,
                    spacing: 70.0,
                },
            ),
            (
                // First dense turret wave: already more turrets, packed
                // tighter, than either of level 1's sparse ones.
                420.0,
                Wave {
                    enemy: EnemyKind::Turret,
                    count: 4,
                    pattern: EntryPattern::Column,
                    spacing: 100.0,
                },
            ),
            (
                650.0,
                Wave {
                    enemy: EnemyKind::Weaver,
                    count: 5,
                    pattern: EntryPattern::Line,
                    spacing: 60.0,
                },
            ),
            (
                870.0,
                Wave {
                    enemy: EnemyKind::Turret,
                    count: 5,
                    pattern: EntryPattern::Line,
                    spacing: 90.0,
                },
            ),
            (
                1090.0,
                Wave {
                    enemy: EnemyKind::Weaver,
                    count: 6,
                    pattern: EntryPattern::V,
                    spacing: 55.0,
                },
            ),
            (
                1300.0,
                Wave {
                    enemy: EnemyKind::Turret,
                    count: 6,
                    pattern: EntryPattern::Column,
                    spacing: 75.0,
                },
            ),
            (
                1520.0,
                Wave {
                    enemy: EnemyKind::Weaver,
                    count: 5,
                    pattern: EntryPattern::Line,
                    spacing: 60.0,
                },
            ),
            (
                // Densest turret wave of the level: 7 turrets, tightest
                // entry spacing, right before the boss.
                1730.0,
                Wave {
                    enemy: EnemyKind::Turret,
                    count: 7,
                    pattern: EntryPattern::Line,
                    spacing: 70.0,
                },
            ),
            (
                // Boss: same "single entity, pattern/spacing moot" note
                // as level 1's boss wave. Large gap after the last
                // regular wave so it has room to clear first.
                2200.0,
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

/// Per-level boss pattern-color variant (spec: "each ending in the boss
/// with pattern colors varied"): a small value keyed by the same 1-based
/// level number as [`level`], for `game::enemies`'s Boss pattern-picking
/// logic to read so its three cycling attack patterns (aimed spread,
/// wall-with-gap, spiral) can render with a different color/palette per
/// level instead of an identical hardcoded one.
///
/// This is a free function rather than a new field on [`LevelScript`]:
/// `LevelScript` is already built with plain struct-literal syntax
/// outside this file (`game::waves`'s tests construct one directly), so
/// adding a field would require also editing that file to supply it --
/// out of scope for this task (`src/levels.rs` only). A function keyed
/// on the level number gives `game::enemies` the same per-level signal
/// without that edit.
///
/// Not wired up yet -- `game::enemies`'s Boss doesn't currently take a
/// level number anywhere along its spawn/update path. That's a follow-up
/// for `enemies.rs`: call `levels::boss_variant(level)` at Boss spawn
/// time, stash the result on the Boss enemy, and use it to pick the
/// bullet color/palette for its attack patterns.
pub fn boss_variant(level: usize) -> u32 {
    match level {
        1 => 0,
        2 => 1,
        _ => 0,
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

    #[test]
    fn level_2_escalates_with_weavers_and_dense_turrets_then_ends_in_boss() {
        let script = level_2();
        let kinds: Vec<EnemyKind> = script.waves.iter().map(|(_, w)| w.enemy).collect();

        assert!(
            kinds.contains(&EnemyKind::Weaver),
            "level 2 must introduce weavers"
        );

        let turret_waves: Vec<_> = script
            .waves
            .iter()
            .filter(|(_, w)| w.enemy == EnemyKind::Turret)
            .collect();
        let weaver_waves: Vec<_> = script
            .waves
            .iter()
            .filter(|(_, w)| w.enemy == EnemyKind::Weaver)
            .collect();
        assert!(turret_waves.len() >= 2);
        assert!(weaver_waves.len() >= 2);

        // "Dense" turrets: every level 2 turret wave beats level 1's max
        // count (3) and every level 2 turret wave's max count beats
        // level 1's, i.e. the escalation is real, not just "some wave
        // somewhere".
        for (_, w) in &turret_waves {
            assert!(w.count > 3, "level 2 turret waves must out-count level 1");
        }
        let max_turret_count = turret_waves.iter().map(|(_, w)| w.count).max().unwrap();
        assert!(max_turret_count > 3);

        let (_, last_wave) = script.waves.last().expect("level 2 has waves");
        assert_eq!(last_wave.enemy, EnemyKind::Boss);
        assert_eq!(last_wave.count, 1);
    }

    #[test]
    fn level_2_triggers_are_strictly_increasing_well_spaced_and_tighter_than_level_1() {
        let script = level_2();
        for window in script.waves.windows(2) {
            let (a, _) = &window[0];
            let (b, _) = &window[1];
            assert!(b > a, "triggers must be in increasing order");
            assert!(
                b - a >= 200.0,
                "waves should be spaced far enough apart not to overlap"
            );
        }

        // "Closer trigger spacing than level 1's sparse turrets": the
        // smallest gap anywhere in level 2 is tighter than the smallest
        // gap anywhere in level 1, while both stay at/above the 200px
        // no-overlap floor.
        let min_gap = |waves: &[(f32, Wave)]| -> f32 {
            waves
                .windows(2)
                .map(|w| w[1].0 - w[0].0)
                .fold(f32::INFINITY, f32::min)
        };
        assert!(min_gap(&level_2().waves) < min_gap(&level_1().waves));
    }

    #[test]
    fn boss_variant_differs_between_level_1_and_level_2() {
        // "Pattern colors varied": the two levels must not resolve to
        // the same variant value.
        assert_ne!(boss_variant(1), boss_variant(2));
    }
}
