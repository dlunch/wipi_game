use alloc::rc::Rc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{GameData, GameEvent, GameState, SessionState, UiState};

pub struct ResolveContext<'a> {
    pub state: &'a GameState,
    pub data: &'a mut Rc<GameData>,
    pub session: Option<&'a SessionState>,
    pub ui: &'a UiState,
}

impl<'a> ResolveContext<'a> {
    pub fn data(&self) -> &GameData {
        self.data
    }
}

pub trait DomainEventResolver {
    fn handles(&self, event: &GameEvent) -> bool;
    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>>;
}
