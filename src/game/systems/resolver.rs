use alloc::{rc::Rc, vec::Vec};

use anyhow::Result;

use crate::game::{
    game_data::GameData,
    game_event::{GameEvent, GameEventKind, GameEventSubscriber},
    world::WorldState,
};

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
            .any(|subscribed| subscribed == &kind)
    }
}
