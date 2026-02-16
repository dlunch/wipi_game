use anyhow::{Result, anyhow, ensure};

use crate::game::GameEvent;
use crate::game::selection::{step_down, step_up};
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameState, InputKey, MenuAction};

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
struct MenuInputResolver;
struct PauseMenuInputResolver;

static MENU_ACTION_CASCADE_RESOLVER: MenuActionCascadeResolver = MenuActionCascadeResolver;
static MENU_INPUT_RESOLVER: MenuInputResolver = MenuInputResolver;
static PAUSE_MENU_INPUT_RESOLVER: PauseMenuInputResolver = PauseMenuInputResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![
        &MENU_INPUT_RESOLVER,
        &PAUSE_MENU_INPUT_RESOLVER,
        &MENU_ACTION_CASCADE_RESOLVER,
    ]
}

impl DomainEventResolver for MenuInputResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::MenuInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::MenuInput(key) = event else {
            return Err(anyhow!("Invalid event: expected MenuInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::Menu),
            "Invalid state: expected Menu"
        );

        let selected = ctx.ui.menu.selected;
        let items = &ctx.ui.menu.state.items;
        let event = match key {
            InputKey::Up => {
                let next = step_up(selected);
                if next != selected {
                    MenuEvent::SetSelected(next)
                } else {
                    MenuEvent::None
                }
            }
            InputKey::Down => {
                let next = step_down(selected, items.len());
                if next != selected {
                    MenuEvent::SetSelected(next)
                } else {
                    MenuEvent::None
                }
            }
            InputKey::Ok => {
                if let Some((_, action)) = items.get(selected).copied() {
                    MenuEvent::Action(action)
                } else {
                    MenuEvent::None
                }
            }
            _ => MenuEvent::None,
        };

        match event {
            MenuEvent::None => Ok(alloc::vec::Vec::new()),
            event => Ok(alloc::vec![GameEvent::Menu(event)]),
        }
    }
}

impl DomainEventResolver for PauseMenuInputResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::PauseMenuInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
    ) -> Result<alloc::vec::Vec<GameEvent>> {
        let GameEvent::PauseMenuInput(key) = event else {
            return Err(anyhow!("Invalid event: expected PauseMenuInput"));
        };
        ensure!(
            matches!(ctx.state, GameState::PauseMenu),
            "Invalid state: expected PauseMenu"
        );

        let selected = ctx.ui.pause_menu.selected;
        let item_count = ctx.ui.pause_menu.state.items.len();
        let event = match key {
            InputKey::Up => {
                let next = step_up(selected);
                if next != selected {
                    PauseMenuEvent::SetSelected(next)
                } else {
                    PauseMenuEvent::None
                }
            }
            InputKey::Down => {
                let next = step_down(selected, item_count);
                if next != selected {
                    PauseMenuEvent::SetSelected(next)
                } else {
                    PauseMenuEvent::None
                }
            }
            InputKey::Ok => match selected {
                0 => PauseMenuEvent::OpenInventory,
                1 => PauseMenuEvent::OpenStats,
                2 => PauseMenuEvent::OpenQuestLog,
                3 => PauseMenuEvent::SaveAndReturnExplore,
                _ => PauseMenuEvent::None,
            },
            InputKey::Back | InputKey::Key0 => PauseMenuEvent::BackToExplore,
            _ => PauseMenuEvent::None,
        };

        match event {
            PauseMenuEvent::None => Ok(alloc::vec::Vec::new()),
            event => Ok(alloc::vec![GameEvent::PauseMenu(event)]),
        }
    }
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
            return Err(anyhow!("Invalid event: expected Menu(Action)"));
        };
        let event = match action {
            MenuAction::NewGame => GameEvent::StartNewGame,
            MenuAction::Continue => GameEvent::ContinueGame,
            MenuAction::Exit => GameEvent::Exit(0),
        };
        Ok(alloc::vec![event])
    }
}
