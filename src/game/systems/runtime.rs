use alloc::rc::Rc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{
    GameData, GameEvent, GameEventKind, GameEventSubscriber, GameState, SessionState, UiState,
};

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

pub trait DomainEventResolver: GameEventSubscriber {
    fn subscribed_kinds(&self) -> &'static [GameEventKind];
    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
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
