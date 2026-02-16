use crate::data::Direction;
use anyhow::{Result, ensure};

use super::CharacterState;
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
    pub fn apply_tick(&mut self, player: &mut CharacterState, event: MovementTickEvent) -> bool {
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

    pub fn apply_event(
        &mut self,
        data: &crate::game::GameData,
        state: &GameState,
        player: &mut CharacterState,
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::Movement(AppMovementEvent::Tick(movement_event, tile_event)) => {
                let moved = self.apply_tick(player, *movement_event);
                if moved && let Some(tile_event) = tile_event.clone() {
                    let _: TileApplyEvent = player.apply_tile_event(data, tile_event);
                }
            }
            GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(direction)) => {
                ensure!(
                    matches!(state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                self.on_direction_released(*direction);
            }
            GameEvent::Explore(crate::game::AppExploreEvent::MoveDirection(direction)) => {
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

fn set_facing(player: &mut CharacterState, dx: i32, dy: i32) {
    player.facing = match (dx, dy) {
        (0, -1) => Direction::Up,
        (0, 1) => Direction::Down,
        (-1, 0) => Direction::Left,
        (1, 0) => Direction::Right,
        _ => player.facing,
    };
}

fn move_by(player: &mut CharacterState, dx: i32, dy: i32) {
    if let Some(new_x) = player.x.checked_add_signed(dx as isize) {
        player.x = new_x;
    }
    if let Some(new_y) = player.y.checked_add_signed(dy as isize) {
        player.y = new_y;
    }
}
