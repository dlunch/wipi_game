use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::systems::resolver::DomainEventResolver;
use crate::game::{ExploreEvent, GameData, GameEvent, GameEventKind, GameState, WorldState};

struct ExploreResolver;

static EXPLORE_RESOLVER: ExploreResolver = ExploreResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&EXPLORE_RESOLVER]
}

impl DomainEventResolver for ExploreResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::Explore]
    }

    fn resolve(
        &self,
        _state: &GameState,
        _data: &Rc<GameData>,
        _world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        if let GameEvent::Explore(ExploreEvent::UseAction(action)) = event {
            out.push(GameEvent::CombatPlayerAction(*action));
        }
        Ok(())
    }
}
