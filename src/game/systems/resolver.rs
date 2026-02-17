use alloc::rc::Rc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{
    GameData, GameEvent, GameEventKind, GameEventSubscriber, GameState, UiState, WorldState,
};

pub struct ResolveContext<'a> {
    pub state: &'a GameState,
    pub data: &'a Rc<GameData>,
    pub world: Option<&'a WorldState>,
    pub ui: &'a UiState,
}

impl<'a> ResolveContext<'a> {
    pub fn data(&self) -> &GameData {
        self.data
    }
}

pub trait DomainEventResolver: GameEventSubscriber {
    fn subscribed_kinds(&self) -> &'static [GameEventKind];
    fn resolve(
        &self,
        ctx: &ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()>;
}

impl<T: DomainEventResolver + ?Sized> GameEventSubscriber for T {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        self.subscribed_kinds()
            .iter()
            .copied()
            .any(|subscribed| subscribed == kind)
    }
}
