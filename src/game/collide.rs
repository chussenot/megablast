//! Pure collision math: circle/circle, circle/AABB, and a substepped
//! swept check so fast entities never tunnel through each other in one
//! tick. No game entity types imported here on purpose -- callers pass
//! plain positions/radii, keeping this trivially unit-testable from any
//! file.
//!
//! Owner: Wave 2 `collide` task. `game/mod.rs` already calls
//! `circle_circle` every tick -- keep these signatures.

pub fn circle_circle(x1: f32, y1: f32, r1: f32, x2: f32, y2: f32, r2: f32) -> bool {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let r = r1 + r2;
    dx * dx + dy * dy <= r * r
}

/// `(rx, ry)` is the rect's top-left corner, `(rw, rh)` its width/height.
pub fn circle_aabb(cx: f32, cy: f32, r: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let closest_x = cx.clamp(rx, rx + rw);
    let closest_y = cy.clamp(ry, ry + rh);
    let dx = cx - closest_x;
    let dy = cy - closest_y;
    dx * dx + dy * dy <= r * r
}

/// Swept circle/circle check across `substeps` linear-interpolation steps
/// between each shape's previous and current position -- the
/// no-tunneling guard for a projectile vs. an enemy at max relative
/// speed (spec: assert this in a test, arkanoid-style).
#[allow(clippy::too_many_arguments)]
pub fn circle_circle_swept(
    x1_prev: f32,
    y1_prev: f32,
    x1: f32,
    y1: f32,
    r1: f32,
    x2_prev: f32,
    y2_prev: f32,
    x2: f32,
    y2: f32,
    r2: f32,
    substeps: u32,
) -> bool {
    let steps = substeps.max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let sx1 = x1_prev + (x1 - x1_prev) * t;
        let sy1 = y1_prev + (y1 - y1_prev) * t;
        let sx2 = x2_prev + (x2 - x2_prev) * t;
        let sy2 = y2_prev + (y2 - y2_prev) * t;
        if circle_circle(sx1, sy1, r1, sx2, sy2, r2) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_circle_overlapping() {
        assert!(circle_circle(0.0, 0.0, 5.0, 3.0, 0.0, 5.0));
    }

    #[test]
    fn circle_circle_touching() {
        // distance between centers == sum of radii: still counts as a hit.
        assert!(circle_circle(0.0, 0.0, 3.0, 10.0, 0.0, 7.0));
    }

    #[test]
    fn circle_circle_separate() {
        assert!(!circle_circle(0.0, 0.0, 3.0, 20.0, 0.0, 3.0));
    }

    #[test]
    fn circle_aabb_corner_hit() {
        // Closest point on the rect is the (10, 10) corner; circle reaches it.
        assert!(circle_aabb(13.0, 13.0, 5.0, 0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn circle_aabb_corner_miss() {
        assert!(!circle_aabb(20.0, 20.0, 5.0, 0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn circle_aabb_edge_hit() {
        // Circle sits above the rect's top edge, away from any corner.
        assert!(circle_aabb(5.0, 13.0, 4.0, 0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn circle_aabb_center_inside_rect() {
        // Circle center inside the rect: closest point == center, always a hit
        // regardless of how small the radius is.
        assert!(circle_aabb(5.0, 5.0, 0.1, 0.0, 0.0, 10.0, 10.0));
    }

    #[test]
    fn swept_catches_tunneling_a_naive_endpoint_check_would_miss() {
        // A 4px-radius shot at 520px/s vs. a stationary 14px-radius enemy,
        // one 1/120s tick. The shot flies past the enemy's side: its path
        // dips inside the combined radius only at the midpoint of the tick,
        // while both its previous and current positions sit just outside
        // collision range. A naive "check current positions only" test (or
        // a swept check with too few substeps to sample the midpoint) must
        // miss it; enough substeps must catch it.
        let dt = 1.0 / 120.0;
        let speed = 520.0;
        let half_travel = speed * dt / 2.0; // shot's horizontal reach either side of center

        let r1 = 4.0; // shot radius
        let r2 = 14.0; // enemy radius
                       // Vertical miss distance: inside (sqrt(18^2 - half_travel^2), 18) so the
                       // path's closest approach (at its horizontal midpoint) collides while
                       // both endpoints stay just clear of the combined radius.
        let y_offset = 17.95;

        let x1_prev = -half_travel;
        let x1 = half_travel;
        let y1_prev = y_offset;
        let y1 = y_offset;

        // Enemy stationary at the origin.
        let x2_prev = 0.0;
        let y2_prev = 0.0;
        let x2 = 0.0;
        let y2 = 0.0;

        // Naive check: only the current (end-of-tick) positions.
        assert!(!circle_circle(x1, y1, r1, x2, y2, r2));
        // Sanity: the previous positions alone don't overlap either.
        assert!(!circle_circle(x1_prev, y1_prev, r1, x2_prev, y2_prev, r2));

        // Too few substeps (only samples the two endpoints) also misses it.
        assert!(!circle_circle_swept(
            x1_prev, y1_prev, x1, y1, r1, x2_prev, y2_prev, x2, y2, r2, 1
        ));

        // Enough substeps sample the midpoint of the tick and catch the hit.
        assert!(circle_circle_swept(
            x1_prev, y1_prev, x1, y1, r1, x2_prev, y2_prev, x2, y2, r2, 4
        ));
    }
}
