//! Two-layer parallax starfield plus a slow terrain band (colored quads).
//!
//! Owner: Wave 1 `render-pipeline` (one working starfield layer, scrolled
//! by `Game::scroll_y`, for Milestone 1). Extended by Wave 5
//! `parallax-terrain` (second parallax layer + terrain band).

use super::QuadInstance;
use crate::game::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// Number of dots in this starfield layer -- also sizes `MAX_QUADS` in
/// `mod.rs`.
pub(super) const STAR_COUNT: usize = 220;

/// Half-size (px) of one star quad -- small enough to read as a point of
/// light rather than a visible square.
const STAR_HALF_SIZE: f32 = 1.2;

/// How much of the raw `scroll_y` distance this layer actually moves by
/// -- the "parallax" in "parallax starfield": a distant layer drifts
/// past slower than the foreground it sits behind. Wave 5's second layer
/// (and terrain band) will each use their own factor to build real depth;
/// this is the one layer Milestone 1 asks for.
const PARALLAX_FACTOR: f32 = 0.35;

/// One star's fixed world position (`y` in `[0, PLAYFIELD_HEIGHT)`, wraps
/// as the layer scrolls past it) and a per-star brightness so the field
/// doesn't read as a flat grid of identical dots.
struct Star {
    x: f32,
    y: f32,
    brightness: f32,
}

/// Owns this layer's fixed star positions, scattered once at startup.
pub(super) struct Starfield {
    stars: Vec<Star>,
}

impl Starfield {
    /// Scatters `STAR_COUNT` stars uniformly at random over the
    /// playfield. Random rather than a grid: a grid reads as an
    /// obviously artificial pattern once it starts scrolling.
    pub(super) fn new() -> Self {
        let stars = (0..STAR_COUNT)
            .map(|_| Star {
                x: rand::random_range(0.0..PLAYFIELD_WIDTH),
                y: rand::random_range(0.0..PLAYFIELD_HEIGHT),
                brightness: rand::random_range(0.35..=1.0),
            })
            .collect();
        Self { stars }
    }

    /// Builds this frame's star quads at `scroll_y`, each wrapped back
    /// into view once it scrolls past the playfield edge -- an infinite
    /// field from a fixed-size point list.
    pub(super) fn instances(&self, scroll_y: f32) -> Vec<QuadInstance> {
        self.stars
            .iter()
            .map(|star| {
                let y = (star.y + scroll_y * PARALLAX_FACTOR).rem_euclid(PLAYFIELD_HEIGHT);
                let b = star.brightness;
                QuadInstance::new(
                    [star.x, y],
                    [STAR_HALF_SIZE, STAR_HALF_SIZE],
                    [b, b, b, 1.0],
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_produces_one_quad_per_star_within_the_playfield() {
        let field = Starfield::new();
        let instances = field.instances(0.0);

        assert_eq!(instances.len(), STAR_COUNT);
        for instance in &instances {
            assert!(instance.center[0] >= 0.0 && instance.center[0] < PLAYFIELD_WIDTH);
            assert!(instance.center[1] >= 0.0 && instance.center[1] < PLAYFIELD_HEIGHT);
        }
    }

    #[test]
    fn instances_move_as_scroll_y_advances() {
        let field = Starfield::new();
        let at_zero = field.instances(0.0);
        let at_large = field.instances(PLAYFIELD_HEIGHT * 10.0);

        // A large scroll distance must actually move at least one star --
        // the whole point of "scrolls based on Game::scroll_y".
        let moved = at_zero
            .iter()
            .zip(at_large.iter())
            .any(|(a, b)| (a.center[1] - b.center[1]).abs() > 0.01);
        assert!(moved, "starfield must move as scroll_y advances");
    }

    #[test]
    fn instances_wrap_back_into_the_playfield_no_matter_how_far_scrolled() {
        let field = Starfield::new();
        for instance in field.instances(PLAYFIELD_HEIGHT * 1000.0 + 17.0) {
            assert!(instance.center[1] >= 0.0 && instance.center[1] < PLAYFIELD_HEIGHT);
        }
    }
}
