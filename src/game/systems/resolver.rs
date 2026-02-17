use alloc::rc::Rc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{GameData, GameEvent, GameEventKind, GameEventSubscriber, WorldState};

pub trait DomainEventResolver: GameEventSubscriber {
    fn subscribed_kinds(&self) -> &'static [GameEventKind];
    fn resolve(
        &self,
        data: &Rc<GameData>,
        world: Option<&WorldState>,
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
