use anyhow::Result;

use crate::game::GameEvent;
use crate::game::MenuAction;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};

#[derive(Debug, Clone, Copy)]
pub enum MenuEvent {
    None,
    SetSelected(usize),
    Action(MenuAction),
}

#[derive(Clone, Copy)]
pub enum PauseMenuEvent {
    None,
    SetSelected(usize),
    OpenInventory,
    OpenStats,
    OpenQuestLog,
    SaveAndReturnExplore,
    BackToExplore,
}

struct MenuActionCascadeResolver;

static MENU_ACTION_CASCADE_RESOLVER: MenuActionCascadeResolver = MenuActionCascadeResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&MENU_ACTION_CASCADE_RESOLVER]
}

impl DomainEventResolver for MenuActionCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Menu(MenuEvent::Action(_)))
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::Menu(MenuEvent::Action(action)) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        let event = match action {
            MenuAction::NewGame => GameEvent::StartNewGame,
            MenuAction::Continue => GameEvent::ContinueGame,
            MenuAction::Exit => GameEvent::Exit(0),
        };
        Ok(alloc::vec![event])
    }
}
