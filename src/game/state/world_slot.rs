use crate::game::{GameEvent, TransitionEvent, WorldEvent};

use super::world::WorldState;

#[derive(Default)]
pub struct WorldSlot {
    active: Option<WorldState>,
}

impl WorldSlot {
    pub fn empty() -> Self {
        Self { active: None }
    }

    pub fn as_ref(&self) -> Option<&WorldState> {
        self.active.as_ref()
    }

    pub fn as_mut(&mut self) -> Option<&mut WorldState> {
        self.active.as_mut()
    }

    pub fn apply_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::World(WorldEvent::Create) => {
                self.active = Some(WorldState::empty());
            }
            GameEvent::Transition(TransitionEvent::ToMenu)
            | GameEvent::Transition(TransitionEvent::ToMenuFromGameOver) => {
                self.active = None;
            }
            _ => {}
        }
    }
}
