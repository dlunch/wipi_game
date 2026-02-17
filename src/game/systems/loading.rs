use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, GameEventKind, LoadingEvent};

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
        _ctx: &ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        if matches!(event, GameEvent::UpdateLoading) {
            out.push(GameEvent::Loading(LoadingEvent::Tick));
        }
        Ok(())
    }
}
