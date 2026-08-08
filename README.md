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

## How this was built

Implemented from a written spec by 20 coordinating agents across 5
sequential waves, using git worktrees, `pact` (file leases +
messaging), and `bd` (issue tracking). Retrospective, including where
the coordination held and where it didn't:
[Megablast Coordination Audit](https://claude.ai/code/artifact/818d328c-9e06-48f6-aae6-841e823d31a0).
