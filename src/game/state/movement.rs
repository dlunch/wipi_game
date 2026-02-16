use crate::data::Direction;
use anyhow::{Result, anyhow, ensure};

use super::PlayerState;
use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{AppMovementEvent, GameEvent, GameState, TileApplyEvent, TransitionEvent};

#[derive(Default, Clone, Copy)]
pub struct MovementState {
    pub pressed_direction: Option<Direction>,
    pub move_cooldown: u32,
}

#[derive(Clone, Copy)]
pub struct MovementTickEvent {
    pub next_state: MovementState,
    pub facing: Option<(i32, i32)>,
    pub step: Option<(i32, i32)>,
}

impl MovementState {
    pub fn apply_tick(&mut self, player: &mut PlayerState, event: MovementTickEvent) -> bool {
        *self = event.next_state;

        if let Some((dx, dy)) = event.facing {
            set_facing(player, dx, dy);
        }

        if let Some((dx, dy)) = event.step {
            move_by(player, dx, dy);
            return true;
        }

        false
    }

    pub fn on_direction_pressed(&mut self, direction: Direction) {
        self.pressed_direction = Some(direction);
        self.move_cooldown = 0;
    }

    pub fn on_direction_released(&mut self, direction: Direction) {
        if self.pressed_direction == Some(direction) {
            self.pressed_direction = None;
        }
    }
}

fn set_facing(player: &mut PlayerState, dx: i32, dy: i32) {
    player.facing = match (dx, dy) {
        (0, -1) => Direction::Up,
        (0, 1) => Direction::Down,
        (-1, 0) => Direction::Left,
        (1, 0) => Direction::Right,
        _ => player.facing,
    };
}

fn move_by(player: &mut PlayerState, dx: i32, dy: i32) {
    if let Some(new_x) = player.x.checked_add_signed(dx as isize) {
        player.x = new_x;
    }
    if let Some(new_y) = player.y.checked_add_signed(dy as isize) {
        player.y = new_y;
    }
}

struct MovementApplier;
struct ReleaseMovementDirectionApplier;
struct PressMovementDirectionApplier;

static MOVEMENT_APPLIER: MovementApplier = MovementApplier;
static RELEASE_MOVEMENT_DIRECTION_APPLIER: ReleaseMovementDirectionApplier =
    ReleaseMovementDirectionApplier;
static PRESS_MOVEMENT_DIRECTION_APPLIER: PressMovementDirectionApplier =
    PressMovementDirectionApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![
        &MOVEMENT_APPLIER,
        &RELEASE_MOVEMENT_DIRECTION_APPLIER,
        &PRESS_MOVEMENT_DIRECTION_APPLIER,
    ]
}

impl DomainEventApplier for MovementApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Movement(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()> {
        let GameEvent::Movement(AppMovementEvent::Tick(movement_event, tile_event)) = event else {
            return Ok(());
        };
        let data = alloc::rc::Rc::clone(ctx.data);
        let s = ctx
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        let moved = s.movement.apply_tick(&mut s.player, *movement_event);
        if moved && let Some(tile_event) = tile_event.clone() {
            let _: TileApplyEvent = s.player.apply_tile_event(&data, tile_event);
        }
        Ok(())
    }
}

impl DomainEventApplier for ReleaseMovementDirectionApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(_))
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()> {
        let GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(direction)) = event
        else {
            return Ok(());
        };
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.movement.on_direction_released(*direction);
        Ok(())
    }
}

impl DomainEventApplier for PressMovementDirectionApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Explore(crate::game::AppExploreEvent::MoveDirection(_))
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()> {
        let GameEvent::Explore(crate::game::AppExploreEvent::MoveDirection(direction)) = event
        else {
            return Ok(());
        };
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.movement.on_direction_pressed(*direction);
        Ok(())
    }
}
