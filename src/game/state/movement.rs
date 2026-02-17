use crate::data::Direction;
use anyhow::Result;

use crate::game::{
    ExploreEvent, GameEvent, GameEventKind, GameEventSubscriber, MovementEvent, TransitionEvent,
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct MovementState {
    pub pressed_direction: Option<Direction>,
    pub move_cooldown: u32,
    pressed_mask: u8,
    press_order: [u8; 4],
    press_len: u8,
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
        let bit = direction_bit(direction);
        self.pressed_mask |= bit;
        self.remove_from_press_order(direction);
        if self.press_len < 4 {
            self.press_order[self.press_len as usize] = direction_to_u8(direction);
            self.press_len += 1;
        }
        self.refresh_pressed_direction();
        self.move_cooldown = 0;
    }

    pub fn on_direction_released(&mut self, direction: Direction) {
        let bit = direction_bit(direction);
        self.pressed_mask &= !bit;
        self.remove_from_press_order(direction);
        self.refresh_pressed_direction();
    }

    pub fn apply_event(&mut self, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Movement(MovementEvent::Tick(movement_event, tile_event)) => {
                let _ = tile_event;
                self.apply_tick(*movement_event);
            }
            GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(direction)) => {
                self.on_direction_released(*direction);
            }
            GameEvent::Explore(ExploreEvent::MoveDirection(direction)) => {
                self.on_direction_pressed(*direction);
            }
            _ => {}
        }
        Ok(())
    }

    fn remove_from_press_order(&mut self, direction: Direction) {
        let code = direction_to_u8(direction);
        let mut write = 0usize;
        let len = self.press_len as usize;
        for read in 0..len {
            let current = self.press_order[read];
            if current != code {
                self.press_order[write] = current;
                write += 1;
            }
        }
        for idx in write..4 {
            self.press_order[idx] = u8::MAX;
        }
        self.press_len = write as u8;
    }

    fn refresh_pressed_direction(&mut self) {
        let mut next = None;
        let mut idx = self.press_len as usize;
        while idx > 0 {
            idx -= 1;
            let code = self.press_order[idx];
            if let Some(direction) = direction_from_u8(code) {
                let bit = direction_bit(direction);
                if (self.pressed_mask & bit) != 0 {
                    next = Some(direction);
                    break;
                }
            }
        }
        self.pressed_direction = next;
    }
}

fn direction_to_u8(direction: Direction) -> u8 {
    match direction {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

fn direction_from_u8(code: u8) -> Option<Direction> {
    match code {
        0 => Some(Direction::Up),
        1 => Some(Direction::Down),
        2 => Some(Direction::Left),
        3 => Some(Direction::Right),
        _ => None,
    }
}

fn direction_bit(direction: Direction) -> u8 {
    1u8 << direction_to_u8(direction)
}

impl GameEventSubscriber for MovementState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(
            kind,
            GameEventKind::Movement | GameEventKind::Transition | GameEventKind::Explore
        )
    }
}
