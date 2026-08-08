# SPEC — `megablast-rs`

A vertical-scrolling shoot-'em-up in the spirit of Xenon 2 Megablast:
forced upward scroll, wave-based enemies, an upgrade shop between levels.
Rust, native windowed UI, wgpu rendering. This file is the contract:
implement in milestone order, ask before deviating.

**Legal line: "in the spirit of" means original everything.** No Bitmap
Brothers sprites, names, level layouts, or the Megablast track. All art
is programmer-art quads/shapes; all names are original. "Xenon 2" appears
nowhere in code, assets, or docs except a one-line README credit
("inspired by").

## Technology decisions (made — do not relitigate)

Same stack, versions, and disciplines as `arkanoid` (use that repo as
the reference implementation for structure and wiring):
- `wgpu` 30.x, `winit` 0.30, `glyphon` for text, `bytemuck`, `rand`.
- Fixed timestep 120 Hz (`FIXED_DT`, `MAX_TICKS_PER_FRAME` guard),
  render interpolation, vsync. Copy the proven main-loop shape.
- Logical resolution 600×800 (portrait playfield), letterboxed.
- **Audio: none in v1**; every audible moment must emit a `GameEvent`
  so a sound layer can subscribe later.
- No engine, no ECS crate. Dependency budget: the five above; anything
  else needs a justifying comment.
- Rust 2021, clippy `-D warnings` clean, fmt clean, no `unsafe` outside
  wgpu setup.

## Architecture — deliberately wider than arkanoid

Arkanoid's fleet telemetry showed two hot files (game.rs 1.5k lines,
render.rs 1.3k lines) serializing 8 agents. This spec splits modules so
parallel tasks touch disjoint files. Respect these boundaries:

- `src/main.rs` — window/event loop wiring only (clone arkanoid's).
- `src/game/mod.rs` — `struct Game`, `fn tick`, state machine, glue only.
- `src/game/player.rs` — ship movement, hitbox, HP, invulnerability.
- `src/game/weapons.rs` — weapon tree, firing, projectiles, damage.
- `src/game/enemies.rs` — enemy types, movement patterns, per-type HP.
- `src/game/waves.rs` — spawn scripting (see Waves), scroll position.
- `src/game/shop.rs` — economy and shop state (pure logic, no UI).
- `src/game/collide.rs` — all collision math (circle/circle, circle/AABB).
- `src/events.rs` — `GameEvent` enum (extend arkanoid's pattern).
- `src/levels.rs` — level data: scroll script + wave script as const data.
- `src/render/mod.rs` — pipeline setup (port arkanoid's instanced quads).
- `src/render/sprites.rs` — entity draw-list building.
- `src/render/background.rs` — scrolling starfield/terrain layers.
- `src/render/hud.rs` — HP bar, cash, score, level, shop screen.

**Rule: no wgpu/winit/IO types anywhere under `src/game/`** — the whole
simulation must run headless for tests, exactly like arkanoid.

## Gameplay spec

**Scroll**: the world scrolls down past the ship (ship flies "up") at a
per-segment speed defined in level data (base 40 px/s). The ship moves
freely in the visible playfield; it cannot push or reverse the scroll
(v1 simplification vs the original).

**Player ship**: 8-directional movement, 260 px/s (diagonals
normalized). HP bar: 100; enemy contact 25, enemy bullet 10. On hit:
1.5 s invulnerability + blink. HP 0 → lose a life, respawn center-bottom
with 2 s invulnerability, weapons downgraded one tier. 3 lives,
0 → Game Over.

**Weapons** (the upgrade tree is the heart of the game):
- Main cannon tiers 1–4: 1/2/3/5 projectiles per volley, spread widens,
  fire rate 6/s constant. Damage per projectile: 10.
- Side shots (owned or not): adds one projectile left+right per volley.
- Rear shot (owned or not): one projectile backward per volley.
- Drone (0–2): small pods flanking the ship, each auto-fires forward
  2/s for 10 damage; drones absorb one enemy bullet each, then are lost.
- All firing is hold-to-fire (Space). Projectile speed 520 px/s.

**Enemies** (five types, all original shapes/names):
- Popcorn (HP 10, 50 pts): sine-drift down, no fire.
- Diver (HP 20, 100 pts): enters top, dives at the ship's position at
  entry time (no homing).
- Turret (HP 40, 150 pts): scrolls with the background, fires aimed
  shots at 0.7/s, bullet speed 180 px/s.
- Weaver (HP 30, 120 pts): horizontal figure-eight, drops straight
  bullets 0.5/s.
- Boss (per level, HP 1200, 5000 pts): stays top-third, three attack
  patterns cycling every 6 s (aimed spread, wall-with-gap, spiral of
  12 bullets). Boss defeat → level cleared.

**Cash & drops**: enemies drop credits (value = score/10) as pickups
falling at 100 px/s; 20% of popcorn, 100% of others. Credits persist
across lives, spent in the shop.

**The shop** (between levels, keyboard menu — the signature feature):
items with fixed prices — Cannon upgrade 300/600/1000 (tier-dependent),
Side shots 400, Rear shot 250, Drone 500 each, Repair 25% HP 150, Extra
life 900. Selling: any owned item back at half price. Leaving the shop
starts the next level. Pure logic in `game/shop.rs` with the UI reading
its state — buying/selling must be unit-testable without a window.

**Waves**: `levels.rs` defines each level as a list of
`(trigger_scroll_y, wave)` entries; a wave names an enemy type, count,
entry pattern (line/column/V), spacing, and per-wave overrides. The
spawn scheduler in `waves.rs` consumes this script deterministically:
same level data + same seed ⇒ identical playthrough (assert in a test).
Two levels in v1: level 1 teaches (popcorn/diver, sparse turrets),
level 2 escalates (weavers, dense turrets), each ending in the boss with
pattern colors varied.

**HUD**: HP bar bottom-left, cash bottom-right, score top-left, level
top-center. States: `Menu → Playing ⇄ Paused → Shop → Playing …
→ GameOver/Victory` — explicit enum, no ad-hoc booleans.

## Rendering quality bar

Instanced quads, sRGB, one pipeline; whole frame ≤ 3 draw calls
(background, entities, text). Two-layer parallax starfield plus a slow
terrain band (colored quads). Juice, all cheap: muzzle flash quad
(1 tick), enemy death = 4 shrinking quads, screen shake 4 px/100 ms on
boss hits, HP bar flashes on damage. 60+ fps on integrated graphics;
simplify effects before simulation, never the reverse.

## Tests (headless, `src/game/` only — no GPU in CI)

Deterministic replay: fixed seed + scripted input ⇒ identical final
score/state hash across two runs. Wave scheduler fires exact spawns at
exact scroll positions. Collision: circle/AABB edge cases + no tunneling
at max relative speed (substep assertion, arkanoid-style). Weapon-tree
volley composition per tier/ownership combination. Shop: every
buy/sell/insufficient-funds path, repair capping at 100, downgrade-on-
death tier floor. Damage/HP arithmetic including invulnerability windows.
CI: fmt + clippy + test on ubuntu-latest (reuse arkanoid's workflow).

## Milestones (one commit per milestone minimum; if multiple agents
work a milestone, each commits its own leased paths — no integration
mega-commits)

1. Skeleton: window + loop + module tree compiling, ship moving on an
   empty scrolling starfield. Playable feel check.
2. Weapons tier 1 + popcorn/diver waves + collisions + deaths + score.
3. Full enemy roster + level script for level 1 + HP/lives/game over.
4. Cash, drops, shop, weapon tree complete, level 2, boss both levels.
5. Polish (parallax, juice, HUD final), README (controls, design notes,
   one screenshot), CI green.

Definition of done: `cargo run --release` from a fresh clone reaches the
menu; a keyboard player can buy a drone in the shop and beat both
bosses; deterministic-replay test passes; clippy clean.
