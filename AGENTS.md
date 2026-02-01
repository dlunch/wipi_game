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
├── main.rs          # Entry point, RpgGame struct, App trait impl, input handling
├── data.rs          # Re-exports from data module
├── data/
│   ├── types.rs     # Data structures (Item, Enemy, Map, Quest, etc.)
│   └── parser.rs    # Text file parsers for .dat resources
├── game.rs          # Re-exports from game module
└── game/
    ├── combat.rs       # Combat system, enemy AI
    ├── dialog.rs       # Dialog rendering
    ├── explore.rs      # Map/entity rendering, HUD
    ├── game_data.rs    # Resource loading, data queries
    ├── inventory.rs    # Inventory UI
    ├── menu.rs         # Main menu
    ├── movement.rs     # Player movement controller
    ├── npc_system.rs   # NPC interaction, dialog processing
    ├── player.rs       # Player state
    ├── quest.rs        # Quest log UI
    ├── quest_system.rs # Quest progress tracking
    ├── renderer.rs     # Drawing primitives, colors
    ├── save.rs         # Save/load system
    ├── shop.rs         # Shop UI
    └── state.rs        # Game state enums
resources/
├── data/            # Game data files (.dat)
└── images/          # Image assets
```

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

### Naming Conventions
- **Structs/Enums**: PascalCase (`GameState`, `FieldEnemy`)
- **Functions/Methods**: snake_case (`update_combat`, `find_npc_at`)
- **Constants**: SCREAMING_SNAKE_CASE (`COLOR_RED`, `TILE_SIZE`)
- **Modules**: snake_case (`game_data`, `npc_system`)

### Error Handling
- No `panic!` in production code paths
- Use `Option<T>` for nullable values
- Use `Result<T, E>` only for operations that can fail at boundaries (resource loading)
- Prefer `unwrap_or_default()` over `unwrap()` for game data loading
- Use `let-else` pattern for early returns:

```rust
let Some(map) = self.current_map() else {
    return;
};
```

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
- `Color` type doesn't implement `Copy` - be careful with reuse

### Struct Design
- Implement `Default` trait when struct has sensible defaults
- Use `#[derive(Debug, Clone)]` for data types
- Add `Copy` only for small enums/structs

### Comments
- **Avoid unnecessary comments** - code should be self-explanatory
- Korean comments allowed for game-specific domain terms
- Doc comments (`///`) only for public API if truly needed

### Module Organization
- Each system has its own file under `game/`
- Module file (`game.rs`, `data.rs`) only contains `mod` and `pub use`
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
