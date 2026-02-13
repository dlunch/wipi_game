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

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests in specific module
cargo test module_name::
```

Note: Tests run in simulation mode with std library available.
Manual gameplay testing: `cargo run` (in wipi repo with this as dependency).

## Project Structure

```
src/
├── main.rs              # Entry point, App trait impl (thin — delegates to game/app)
├── data.rs              # Re-exports from data module
├── data/
│   ├── types.rs         # Data structures (Item, Enemy, Map, Quest, Skill, etc.)
│   └── parser.rs        # Text file parsers for .dat resources
├── game.rs              # Re-exports from game module (state, systems, rendering, app)
└── game/
    ├── app.rs           # RpgGame struct, dispatch pipeline, effect application
    ├── app/
    │   ├── intent.rs    # AppAction, AppEffect, ExploreIntent enums
    │   ├── handler.rs   # Input handler free functions (per-state intent handling)
    │   └── update.rs    # Game loop free functions (loading, movement, combat tick)
    ├── state.rs         # GameState enum + re-exports from state sub-modules
    ├── state/
    │   ├── player.rs    # PlayerState struct + inventory/equipment/quest methods
    │   ├── dialog.rs    # DialogState, DialogIntent
    │   ├── shop.rs      # ShopState, ShopMode, ShopIntent
    │   ├── menu.rs      # MenuState, MenuAction, MenuIntent, PauseMenuIntent
    │   ├── inventory.rs # InventoryState, InventoryIntent
    │   └── tile_event.rs # TileEvent enum, check_tile_event
    ├── systems.rs       # Re-exports from systems sub-modules
    ├── systems/
    │   ├── combat.rs    # CombatState, CombatIntent/Event, stateless combat logic
    │   ├── movement.rs  # MovementState, tick/key press/release free functions
    │   ├── npc.rs       # NPC interaction free functions
    │   ├── player.rs    # PlayerIntent/Event, reduce free function
    │   └── quest.rs     # Quest progression free functions
    ├── rendering.rs     # Re-exports from rendering sub-modules
    ├── rendering/
    │   ├── renderer.rs  # Color constants, drawing primitives (text, rect, fill)
    │   ├── game.rs      # Main render dispatch, loading screen
    │   ├── dialog.rs    # Dialog box rendering
    │   ├── explore.rs   # Map/entity/HUD rendering
    │   ├── inventory.rs # Inventory & stats UI
    │   ├── menu.rs      # Main menu & pause menu
    │   ├── quest.rs     # Quest log UI
    │   └── shop.rs      # Shop UI
    ├── game_data.rs     # Resource loading, data queries
    └── save.rs          # Save/load system
resources/
├── data/                # Game data files (.dat)
└── images/              # Image assets
```

## Architecture

The codebase follows a **state + stateless systems + rendering** pattern:

- **State** (`state/`): Pure data structs (`PlayerState`, `DialogState`, etc.). All state struct names end with `State`.
- **Systems** (`systems/`): Stateless free functions that operate on state. Each system owns its runtime state (e.g. `CombatState`) but logic is free functions, not methods.
- **Rendering** (`rendering/`): Pure draw functions. No game logic — only reads state and draws to framebuffer.
- **App** (`app/`): Orchestration layer. `RpgGame` struct holds all state. Input → `AppAction` → `AppEffect` → apply via free functions taking individual fields.

Input flow: `key → intent_for_key() → AppEffect → apply_effect() → system free fn(state fields) → state mutation → render(state fields)`

## Code Style Guidelines

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

### Pattern Matching
- Use `if let ... && let ...` for chained conditions (Rust nightly feature)
- Prefer `matches!()` for simple boolean checks

```rust
// Good
if let Some(quest) = data.find_quest(&id)
    && quest.quest_type == QuestType::Kill
{
    // ...
}

// Good
if matches!(self.state, GameState::Explore) { ... }
```

### Memory & Allocation
- **NO standard library** - use `alloc::` for String, Vec, etc.
- Clone sparingly - prefer references where possible

### Struct Design
- Implement `Default` trait when struct has sensible defaults
- Use `#[derive(Debug, Clone)]` for data types
- Add `Copy` only for small enums/structs

### Comments
- **Avoid unnecessary comments** - code should be self-explanatory
- Korean comments allowed for game-specific domain terms
- Doc comments (`///`) only for public API if truly needed

### Module Organization
- Each system has its own file under `game/systems/`
- State structs live under `game/state/`, rendering under `game/rendering/`
- Module file (`game.rs`, `data.rs`, `state.rs`, etc.) only contains `mod` and `pub use`
- Prefer functions over methods when logic doesn't need `self`

### Functions
- Keep functions small and focused
- Use descriptive names over comments
- Prefer returning `Option<GameState>` for state changes

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
