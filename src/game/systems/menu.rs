use crate::game::MenuAction;
use crate::game::selection::{step_down, step_up};
use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{
    ApplyContext, DomainEventApplier, DomainEventResolver, ResolveContext,
};
use crate::game::{GameState, MenuState, RuntimeEvent, has_save_data};

#[derive(Debug, Clone, Copy)]
pub enum MenuIntent {
    MoveUp,
    MoveDown,
    Select,
}

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

#[derive(Debug, Clone, Copy)]
pub enum PauseMenuIntent {
    MoveUp,
    MoveDown,
    Select,
    Back,
}

pub fn resolve(selected: usize, items: &[(&str, MenuAction)], intent: MenuIntent) -> MenuEvent {
    match intent {
        MenuIntent::MoveUp => {
            let next = step_up(selected);
            if next != selected {
                return MenuEvent::SetSelected(next);
            }
        }
        MenuIntent::MoveDown => {
            let next = step_down(selected, items.len());
            if next != selected {
                return MenuEvent::SetSelected(next);
            }
        }
        MenuIntent::Select => {
            if let Some((_, action)) = items.get(selected).copied() {
                return MenuEvent::Action(action);
            }
        }
    }

    MenuEvent::None
}

pub fn resolve_many(
    selected: usize,
    items: &[(&str, MenuAction)],
    intent: MenuIntent,
) -> alloc::vec::Vec<MenuEvent> {
    match resolve(selected, items, intent) {
        MenuEvent::None => alloc::vec::Vec::new(),
        event => alloc::vec![event],
    }
}

pub fn resolve_pause(
    selected: usize,
    item_count: usize,
    intent: PauseMenuIntent,
) -> PauseMenuEvent {
    match intent {
        PauseMenuIntent::MoveUp => {
            let next = step_up(selected);
            if next != selected {
                return PauseMenuEvent::SetSelected(next);
            }
        }
        PauseMenuIntent::MoveDown => {
            let next = step_down(selected, item_count);
            if next != selected {
                return PauseMenuEvent::SetSelected(next);
            }
        }
        PauseMenuIntent::Select => match selected {
            0 => return PauseMenuEvent::OpenInventory,
            1 => return PauseMenuEvent::OpenStats,
            2 => return PauseMenuEvent::OpenQuestLog,
            3 => return PauseMenuEvent::SaveAndReturnExplore,
            _ => {}
        },
        PauseMenuIntent::Back => return PauseMenuEvent::BackToExplore,
    }

    PauseMenuEvent::None
}

pub fn resolve_pause_many(
    selected: usize,
    item_count: usize,
    intent: PauseMenuIntent,
) -> alloc::vec::Vec<PauseMenuEvent> {
    match resolve_pause(selected, item_count, intent) {
        PauseMenuEvent::None => alloc::vec::Vec::new(),
        event => alloc::vec![event],
    }
}

struct MenuInputResolver;
struct PauseMenuInputResolver;
struct MenuActionCascadeResolver;
struct MenuApplier;
struct PauseMenuApplier;
struct OpenPauseMenuApplier;
struct OpenMenuFromExploreApplier;

static MENU_INPUT_RESOLVER: MenuInputResolver = MenuInputResolver;
static PAUSE_MENU_INPUT_RESOLVER: PauseMenuInputResolver = PauseMenuInputResolver;
static MENU_ACTION_CASCADE_RESOLVER: MenuActionCascadeResolver = MenuActionCascadeResolver;
static MENU_APPLIER: MenuApplier = MenuApplier;
static PAUSE_MENU_APPLIER: PauseMenuApplier = PauseMenuApplier;
static OPEN_PAUSE_MENU_APPLIER: OpenPauseMenuApplier = OpenPauseMenuApplier;
static OPEN_MENU_FROM_EXPLORE_APPLIER: OpenMenuFromExploreApplier = OpenMenuFromExploreApplier;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![
        &MENU_INPUT_RESOLVER,
        &PAUSE_MENU_INPUT_RESOLVER,
        &MENU_ACTION_CASCADE_RESOLVER,
    ]
}

pub fn appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![
        &MENU_APPLIER,
        &PAUSE_MENU_APPLIER,
        &OPEN_PAUSE_MENU_APPLIER,
        &OPEN_MENU_FROM_EXPLORE_APPLIER,
    ]
}

impl DomainEventResolver for MenuInputResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::MenuInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::MenuInput(intent) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        ensure!(
            matches!(ctx.state, GameState::Menu),
            "Invalid state: expected Menu"
        );

        Ok(
            resolve_many(ctx.ui.menu.selected, &ctx.ui.menu.state.items, *intent)
                .into_iter()
                .map(RuntimeEvent::Menu)
                .collect(),
        )
    }
}

impl DomainEventResolver for PauseMenuInputResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::PauseMenuInput(_))
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::PauseMenuInput(intent) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        ensure!(
            matches!(ctx.state, GameState::PauseMenu),
            "Invalid state: expected PauseMenu"
        );
        ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        Ok(resolve_pause_many(
            ctx.ui.pause_menu.selected,
            ctx.ui.pause_menu.state.items.len(),
            *intent,
        )
        .into_iter()
        .map(RuntimeEvent::PauseMenu)
        .collect())
    }
}

impl DomainEventResolver for MenuActionCascadeResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Menu(MenuEvent::Action(_)))
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::Menu(MenuEvent::Action(action)) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        let event = match action {
            MenuAction::NewGame => RuntimeEvent::StartNewGame,
            MenuAction::Continue => RuntimeEvent::ContinueGame,
            MenuAction::Exit => RuntimeEvent::Exit(0),
        };
        Ok(alloc::vec![event])
    }
}

impl DomainEventApplier for MenuApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Menu(_))
    }

    fn apply(&self, engine: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Menu(event) = event else {
            return Ok(());
        };
        match event {
            MenuEvent::None => {}
            MenuEvent::SetSelected(selected) => engine.ui_mut().menu.set_selected(*selected),
            MenuEvent::Action(_) => {}
        }
        Ok(())
    }
}

impl DomainEventApplier for PauseMenuApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::PauseMenu(_))
    }

    fn apply(&self, engine: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::PauseMenu(event) = event else {
            return Ok(());
        };
        match event {
            PauseMenuEvent::None => {}
            PauseMenuEvent::SetSelected(selected) => {
                engine.ui_mut().pause_menu.set_selected(*selected)
            }
            PauseMenuEvent::OpenInventory => {
                engine.ui_mut().inventory.reset();
                engine.transition_to(GameState::Inventory);
            }
            PauseMenuEvent::OpenStats => engine.transition_to(GameState::Stats),
            PauseMenuEvent::OpenQuestLog => engine.transition_to(GameState::QuestLog),
            PauseMenuEvent::SaveAndReturnExplore => {
                {
                    let s = engine
                        .session()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let _ = crate::game::save_game(&s.player);
                }
                engine.ui_mut().shop.reset();
                engine.transition_to(GameState::Explore);
            }
            PauseMenuEvent::BackToExplore => engine.transition_to(GameState::Explore),
        }
        Ok(())
    }
}

impl DomainEventApplier for OpenPauseMenuApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::OpenPauseMenu)
    }

    fn apply(&self, engine: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        engine.ui_mut().pause_menu.reset();
        engine.transition_to(GameState::PauseMenu);
        Ok(())
    }
}

impl DomainEventApplier for OpenMenuFromExploreApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::OpenMenuFromExplore)
    }

    fn apply(&self, engine: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        {
            let s = engine
                .session()
                .ok_or_else(|| anyhow!("No active session"))?;
            let _ = crate::game::save_game(&s.player);
        }
        engine
            .ui_mut()
            .menu
            .set_menu(MenuState::new(has_save_data()));
        engine.transition_to(GameState::Menu);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuEvent, MenuIntent, PauseMenuEvent, PauseMenuIntent, resolve, resolve_pause};
    use crate::game::{MenuAction, MenuState};

    #[test]
    fn menu_reduce_returns_selection_and_action_events() {
        let items = MenuState::new(true).items;
        let mut selected = 0;

        let event = resolve(selected, &items, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::SetSelected(1)));
        selected = 1;

        let event = resolve(selected, &items, MenuIntent::MoveDown);
        assert!(matches!(event, MenuEvent::SetSelected(2)));
        selected = 2;

        let event = resolve(selected, &items, MenuIntent::Select);
        assert!(matches!(event, MenuEvent::Action(MenuAction::Exit)));
    }

    #[test]
    fn pause_reduce_returns_expected_events() {
        let mut selected = 0;
        let item_count = 4;

        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenInventory));

        selected = 1;
        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenStats));

        selected = 2;
        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::OpenQuestLog));

        selected = 3;
        let event = resolve_pause(selected, item_count, PauseMenuIntent::Select);
        assert!(matches!(event, PauseMenuEvent::SaveAndReturnExplore));
    }
}
