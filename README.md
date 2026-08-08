# Megablast

A vertical-scrolling shoot-'em-up with wave-based enemies and an
upgrade shop between levels, built in Rust with `wgpu`.

_Inspired by Xenon 2 Megablast._

![Screenshot](docs/screenshot.png)
_(screenshot pending — not yet captured)_

## Controls

- **Arrow keys or WASD** — move in 8 directions
- **Space** (hold) — fire
- **P or Escape** — pause

## Design notes

The simulation runs on a fixed 120 Hz timestep, fully decoupled from
rendering, so gameplay stays deterministic regardless of frame rate
and the renderer just interpolates between ticks. Enemy encounters are
driven by a per-level wave script — spawn triggers keyed to scroll
position rather than hand-placed timers — so a level's difficulty
curve lives as data, not code. Between levels, credits earned from
enemy drops are spent in a shop that upgrades weapons, drones, and
survivability, giving runs a persistent economy on top of the
moment-to-moment action.

## Building

```bash
cargo run --release
```
