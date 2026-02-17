use crate::data::Direction;
use anyhow::{Result, ensure};

use crate::game::{
    ExploreEvent, GameEvent, GameEventKind, GameEventSubscriber, GameState, MovementEvent,
    TransitionEvent,
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
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
    pub fn apply_tick(&mut self, event: MovementTickEvent) -> bool {
        *self = event.next_state;
        event.step.is_some()
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

    pub fn apply_event(&mut self, state: &GameState, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Movement(MovementEvent::Tick(movement_event, tile_event)) => {
                let _ = tile_event;
                self.apply_tick(*movement_event);
            }
            GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(direction)) => {
                ensure!(
                    matches!(state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                self.on_direction_released(*direction);
            }
            GameEvent::Explore(ExploreEvent::MoveDirection(direction)) => {
                ensure!(
                    matches!(state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                self.on_direction_pressed(*direction);
            }
            _ => {}
        }
        Ok(())
    }
}

impl GameEventSubscriber for MovementState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(
            kind,
            GameEventKind::Movement | GameEventKind::Transition | GameEventKind::Explore
        )
    }
}
