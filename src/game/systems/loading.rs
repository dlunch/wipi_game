use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use alloc::rc::Rc;

use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{GameData, GameEvent, GameEventKind, GameState, LoadingEvent, WorldState};

struct LoadingResolver;

static LOADING_RESOLVER: LoadingResolver = LoadingResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&LOADING_RESOLVER]
}

impl DomainEventResolver for LoadingResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::UpdateLoading]
    }

    fn resolve(
        &self,
        _state: &GameState,
        _data: &Rc<GameData>,
        _world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        if matches!(event, GameEvent::UpdateLoading) {
            out.push(GameEvent::Loading(LoadingEvent::Tick));
        }
        Ok(())
    }
}
