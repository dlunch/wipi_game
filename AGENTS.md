# AGENTS.md - Coding Agent Guidelines

## Project Overview

WIPI Game is a retro RPG demo for the WIPI SDK (Korean feature phone platform emulator).
- **Language**: Rust (nightly)
- **Environment**: `no_std` with `alloc` crate - NO standard library
- **Target**: ARM thumbv4t (feature phone) + simulation mode
- **Screen**: 240x320 pixels, 16x16 tile size

## Build Commands

```bash
# Development build (simulation mode - default)
cargo build

# Lint check - MUST pass before commit
cargo clippy -- -D warnings

# Format check - MUST pass before commit
cargo fmt --check

# Apply formatting
cargo fmt

# Release build for emulator (requires ../wipi repo cloned)
./release.sh
```

## Project Structure

```
src/
├── main.rs              # App entry/WIPI glue
├── engine.rs            # Runtime orchestration (input->event queue->apply->render)
├── data/                # Data model + parser
└── game/
    ├── game_event.rs    # Core runtime event types
    ├── state/           # Persistent domain state + apply logic
    ├── systems/         # Stateless resolver logic (derive events)
    ├── ui/              # UI state + input resolve/apply
    ├── rendering/       # Render state + draw modules + sprite atlas
    ├── game_data.rs     # Loaded game data access
    └── save.rs          # Save/load
resources/
├── data/                # Game data files (.dat)
└── images/              # Image assets
```

## Architecture

The codebase follows an **event queue + resolve systems + apply + rendering** pattern:

- **State** (`state/`): Owns data and only applies events (mutation sink).
- **Systems** (`systems/`): Read-only over current state/data, derive additional `GameEvent`s.
- **UI** (`ui/`): `Input -> UiEvent -> GameEvent` conversion, no direct world mutation.
- **Engine** (`engine.rs`): Only orchestrates queue order and subscribers.
- **Rendering** (`rendering/`): Consumes state/render-fx to produce frames.
- **App** (`main.rs`): Platform hook glue only.

Input flow: `keydown/keyup -> pending input queue -> UiEvent resolve/apply -> GameEvent dispatch -> resolve/apply loop -> render patch or rebuild`

Update flow (timer tick): `process pending inputs -> render fx tick (includes animation tick) -> UpdateLoading/UpdateMovement/UpdateCombat events -> resolve/apply -> render patch or rebuild`.

Mandatory architecture rules:
- Resolve must not mutate state directly.
- Only Apply mutates state.
- Systems must not orchestrate by calling other systems/states directly.
- Cross-system ordering must be controlled only in the `engine.rs` event queue.
- Keep a strict boundary between UI events and Game events.
- Prefer field/delta events over whole-state snapshot replacement when possible.
- Propagate errors upward with `anyhow::Result`; the engine handles state transition to `GameState::Error`.
- Prefer the simplest architecture that satisfies requirements; avoid speculative abstractions.

## Code Style Guidelines

### Simplicity First
- Prefer the simplest code that works and keeps behavior explicit.
- Do not introduce extra layers/types/helpers if a direct implementation is clear enough.
- Avoid one-off abstractions (single-call-site wrappers, unnecessary indirection, over-generic traits).
- Remove obsolete branches/fallback structures once the new path is stable.
- When two implementations are equivalent, choose the one with fewer moving parts.

### Imports
- Order: `alloc` crate → external crates (`wipi`) → `super::` → `crate::`
- Group related imports, separate groups with blank lines
- Use `cargo fmt` to auto-sort within groups

```rust
use alloc::string::String;
use alloc::vec::Vec;

use wipi::{app::App, event::KeyCode, framebuffer::Framebuffer};

use super::{Player, GameState};
use crate::data::{Item, Map};
```

### Error Handling
- No `panic!` or `unwrap()` in production code paths
- Use `anyhow::Result` with `?` for error propagation (`anyhow = { version = "1", default-features = false }`)
- **Game data parsers**: Strict — malformed data must return errors (wrong fields, bad numbers, unknown types)
- **Save/load**: IO errors propagate via `anyhow::Result`, field deserialization uses `unwrap_or` for graceful recovery
- **Top-level handlers** (`on_paint`, `on_keydown`): Catch errors and transition to `GameState::Error`
- `WIPICError` has no `Display` — convert with `.map_err(|e| anyhow::anyhow!("{:?}", e))`
- Use `Option<T>` for nullable values
- Use `let-else` pattern for early returns:

```rust
let Some(map) = self.current_map() else {
    return;
};
```

### Testing
- Test functions should return `Result<()>` and use `?` instead of `unwrap()`
- Error case tests use `assert!(result.is_err())` directly

### Memory & Allocation
- **NO standard library** - use `alloc::` for String, Vec, etc.
- Clone sparingly - prefer references where possible
- Avoid `#[derive(Clone)]` unless clone semantics are truly required by call sites.
- For `Copy` types, `Clone` may be derived only as required by Rust trait rules.

### Comments
- **Avoid unnecessary comments** - code should be self-explanatory
- Korean comments allowed for game-specific domain terms
- Doc comments (`///`) only for public API if truly needed

### Module Organization
- Each system has its own file under `game/systems/`
- Persistent world state lives under `game/state/`, UI state lives under `game/ui/`, rendering under `game/rendering/`
- `main.rs` should stay thin; orchestration belongs in `engine.rs`
- Module files (`game.rs`, `data.rs`, `state.rs`, etc.) should primarily contain `mod` and `pub use`
- Prefer methods for state/session mutation and functions for stateless system resolution
- Sprite assets should be atlas-based (`images/atlas.png` + `images/atlas.meta`), with rectangle fallback when atlas/meta is missing.

## Key Constraints

1. **no_std**: Cannot use `std::`, only `core::` and `alloc::`
2. **No threads**: Single-threaded environment
3. **No filesystem**: Resources accessed via `wipi::resource::Resource`
4. **Limited memory**: Embedded target, be mindful of allocations
5. **No floating point hardware**: `f32` operations are software-emulated

## Commit Guidelines

Before every commit:
1. `cargo fmt` - format code
2. `cargo clippy -- -D warnings` - must pass with no warnings
3. `cargo build` - must compile

Commit messages: imperative mood, concise ("Add player HP bar", "Fix NPC collision")

## WIPI SDK Notes

- Entry point: `#[wipi_main]` macro on `main()` function
- App trait: implement `on_paint`, `on_keydown`, `on_keyup`
- Rendering: `Framebuffer::screen_framebuffer()` + `repaint()`
- Input: `KeyCode` enum (Ok, Back, Up, Down, Left, Right, Key0-Key9)
- Resources: `Resource::new("path")?.read()` returns `&[u8]`
