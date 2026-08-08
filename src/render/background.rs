//! Two-layer parallax starfield plus a slow terrain band (colored quads).
//!
//! Owner: Wave 1 `render-pipeline` (one working starfield layer, scrolled
//! by `Game::scroll_y`, for Milestone 1). Extended by Wave 5
//! `parallax-terrain` (second parallax layer + terrain band): a more
//! distant, dimmer/sparser starfield behind the original layer, and a
//! handful of large colored quads scrolling faster than either -- three
//! factors, three apparent depths, from the same fixed-size point lists.

use super::QuadInstance;
use crate::game::{PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};

/// Number of dots in the near starfield layer.
pub(super) const STAR_COUNT: usize = 220;

/// Half-size (px) of one near-layer star quad -- small enough to read as
/// a point of light rather than a visible square.
const STAR_HALF_SIZE: f32 = 1.2;

/// How much of the raw `scroll_y` distance the near layer actually moves
/// by -- the "parallax" in "parallax starfield": a distant layer drifts
/// past slower than the foreground it sits behind.
const PARALLAX_FACTOR: f32 = 0.35;

/// Number of dots in the far starfield layer -- fewer than `STAR_COUNT`
/// so the layer behind reads as sparser as well as slower, not just a
/// duplicate of the near field.
const FAR_STAR_COUNT: usize = 120;

/// Half-size (px) of one far-layer star quad -- slightly smaller than
/// the near layer's, reinforcing the sense of distance.
const FAR_STAR_HALF_SIZE: f32 = 0.9;

/// The far layer's parallax factor: slower than `PARALLAX_FACTOR`, so it
/// drifts past even more slowly than the near layer -- the more distant
/// half of the two-layer starfield.
const FAR_PARALLAX_FACTOR: f32 = 0.15;

/// Number of quads making up the terrain band's ground/horizon
/// silhouette -- "a few", large enough that they overlap into a jagged
/// skyline rather than reading as separate rectangles.
const TERRAIN_QUAD_COUNT: usize = 8;

/// The terrain band's parallax factor: faster than either starfield
/// layer, so it reads as the closest of the three depths -- ground
/// scrolling past underneath a more distant sky.
const TERRAIN_PARALLAX_FACTOR: f32 = 0.6;

/// A handful of muted ground tones terrain quads are drawn in, picked
/// per-quad (by index) so the band doesn't read as one flat color.
const TERRAIN_COLORS: [[f32; 4]; 3] = [
    [0.16, 0.15, 0.20, 1.0],
    [0.20, 0.14, 0.16, 1.0],
    [0.13, 0.17, 0.19, 1.0],
];

/// One star's fixed world position (`y` in `[0, PLAYFIELD_HEIGHT)`, wraps
/// as the layer scrolls past it) and a per-star brightness so the field
/// doesn't read as a flat grid of identical dots.
struct Star {
    x: f32,
    y: f32,
    brightness: f32,
}

/// Scatters `count` stars uniformly at random over the playfield, with
/// brightness drawn from `brightness_range` -- random rather than a
/// grid, since a grid reads as an obviously artificial pattern once it
/// starts scrolling. Shared by both starfield layers; only the count and
/// brightness range differ between them.
fn scatter_stars(count: usize, brightness_range: std::ops::RangeInclusive<f32>) -> Vec<Star> {
    (0..count)
        .map(|_| Star {
            x: rand::random_range(0.0..PLAYFIELD_WIDTH),
            y: rand::random_range(0.0..PLAYFIELD_HEIGHT),
            brightness: rand::random_range(brightness_range.clone()),
        })
        .collect()
}

/// One terrain quad's fixed world position, size and color. `y` wraps
/// the same way a star's does as the band scrolls past it.
struct TerrainQuad {
    x: f32,
    y: f32,
    half_size: [f32; 2],
    color: [f32; 4],
}

/// Scatters `TERRAIN_QUAD_COUNT` large rectangles across the playfield
/// width (evenly spaced, so they tile into a continuous band with no
/// gaps) with jittered size and a per-quad ground tone -- large enough,
/// and close enough together, to overlap into a jagged silhouette rather
/// than reading as isolated boxes.
fn scatter_terrain() -> Vec<TerrainQuad> {
    (0..TERRAIN_QUAD_COUNT)
        .map(|i| {
            let x = (i as f32 + 0.5) / TERRAIN_QUAD_COUNT as f32 * PLAYFIELD_WIDTH;
            TerrainQuad {
                x,
                y: rand::random_range(0.0..PLAYFIELD_HEIGHT),
                half_size: [
                    rand::random_range(50.0..=110.0),
                    rand::random_range(20.0..=45.0),
                ],
                color: TERRAIN_COLORS[i % TERRAIN_COLORS.len()],
            }
        })
        .collect()
}

/// Total quads this module contributes per frame -- both starfield
/// layers plus the terrain band. `render/mod.rs`'s `MAX_QUADS` must size
/// the instance buffer off this constant (not just `STAR_COUNT`) now
/// that this module draws more than one layer.
pub(super) const BACKGROUND_QUAD_COUNT: usize = STAR_COUNT + FAR_STAR_COUNT + TERRAIN_QUAD_COUNT;

/// Owns this module's fixed layer layouts, scattered once at startup:
/// the near and far starfields, and the terrain band.
pub(super) struct Starfield {
    stars: Vec<Star>,
    far_stars: Vec<Star>,
    terrain: Vec<TerrainQuad>,
}

impl Starfield {
    /// Scatters both starfield layers and the terrain band.
    pub(super) fn new() -> Self {
        Self {
            stars: scatter_stars(STAR_COUNT, 0.35..=1.0),
            far_stars: scatter_stars(FAR_STAR_COUNT, 0.15..=0.5),
            terrain: scatter_terrain(),
        }
    }

    /// Builds this frame's quads at `scroll_y`: far starfield, then near
    /// starfield, then the terrain band, each wrapped back into view once
    /// it scrolls past the playfield edge -- an infinite field/band from
    /// fixed-size point lists. Draw order matches depth (far to near) so
    /// the closer layers paint over the more distant ones where they
    /// overlap.
    pub(super) fn instances(&self, scroll_y: f32) -> Vec<QuadInstance> {
        let far = self.far_stars.iter().map(|star| {
            let y = (star.y + scroll_y * FAR_PARALLAX_FACTOR).rem_euclid(PLAYFIELD_HEIGHT);
            let b = star.brightness;
            QuadInstance::new(
                [star.x, y],
                [FAR_STAR_HALF_SIZE, FAR_STAR_HALF_SIZE],
                [b, b, b, 1.0],
            )
        });
        let near = self.stars.iter().map(|star| {
            let y = (star.y + scroll_y * PARALLAX_FACTOR).rem_euclid(PLAYFIELD_HEIGHT);
            let b = star.brightness;
            QuadInstance::new(
                [star.x, y],
                [STAR_HALF_SIZE, STAR_HALF_SIZE],
                [b, b, b, 1.0],
            )
        });
        let terrain = self.terrain.iter().map(|quad| {
            let y = (quad.y + scroll_y * TERRAIN_PARALLAX_FACTOR).rem_euclid(PLAYFIELD_HEIGHT);
            QuadInstance::new([quad.x, y], quad.half_size, quad.color)
        });
        far.chain(near).chain(terrain).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_produces_one_quad_per_star_and_terrain_piece_within_the_playfield() {
        let field = Starfield::new();
        let instances = field.instances(0.0);

        assert_eq!(instances.len(), BACKGROUND_QUAD_COUNT);
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
        assert!(moved, "background layers must move as scroll_y advances");
    }

    #[test]
    fn instances_wrap_back_into_the_playfield_no_matter_how_far_scrolled() {
        let field = Starfield::new();
        for instance in field.instances(PLAYFIELD_HEIGHT * 1000.0 + 17.0) {
            assert!(instance.center[1] >= 0.0 && instance.center[1] < PLAYFIELD_HEIGHT);
        }
    }

    #[test]
    fn far_layer_scrolls_slower_than_near_layer_which_scrolls_slower_than_terrain() {
        let field = Starfield::new();
        let scroll_delta = 500.0;
        let at_zero = field.instances(0.0);
        let at_delta = field.instances(scroll_delta);

        // Every entry in a given layer moves by the same amount modulo
        // PLAYFIELD_HEIGHT (same scroll_y, same factor), regardless of
        // that entry's own starting position, so the first quad in each
        // layer's range is enough to read off that layer's actual shift.
        let shift = |i: usize| -> f32 {
            (at_delta[i].center[1] - at_zero[i].center[1]).rem_euclid(PLAYFIELD_HEIGHT)
        };

        let far_shift = shift(0);
        let near_shift = shift(FAR_STAR_COUNT);
        let terrain_shift = shift(FAR_STAR_COUNT + STAR_COUNT);

        assert!(
            far_shift < near_shift,
            "far starfield ({far_shift}) should scroll slower than near starfield ({near_shift})"
        );
        assert!(
            near_shift < terrain_shift,
            "near starfield ({near_shift}) should scroll slower than terrain ({terrain_shift})"
        );
    }
}
