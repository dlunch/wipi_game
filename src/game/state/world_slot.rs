use super::world::WorldState;
use crate::game::{GameEvent, TransitionEvent, WorldEvent};

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

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn apply_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::World(WorldEvent::CreateWorld) => {
                if self.active.is_none() {
                    self.active = Some(WorldState::empty());
                }
            }
            GameEvent::Transition(TransitionEvent::ToMenu) => {
                self.active = None;
            }
            _ => {}
        }
    }
}
