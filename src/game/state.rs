mod appliers;
mod combat;
mod movement;
mod player;
pub(crate) mod session;
pub(crate) mod ui;

pub use appliers::domain_appliers;
pub use combat::{CombatState, FieldEnemy, SkillEffect};
pub use movement::{MovementState, MovementTickEvent};
pub use player::{PlayerAction, PlayerEvent, PlayerState, TileApplyEvent, TileEvent};

use alloc::string::String;
use anyhow::Result;

use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{GameEvent, LoadingEvent, MenuState, TransitionEvent, has_save_data};

#[derive(Debug)]
pub enum GameState {
    Loading(usize),
    Menu,
    Explore,
    Inventory,
    Stats,
    Dialog,
    Shop,
    QuestLog,
    PauseMenu,
    GameOver,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameStateKind {
    Loading,
    Menu,
    Explore,
    Inventory,
    Stats,
    Dialog,
    Shop,
    QuestLog,
    PauseMenu,
    GameOver,
    Error,
}

impl GameState {
    fn kind(&self) -> GameStateKind {
        match self {
            GameState::Loading(_) => GameStateKind::Loading,
            GameState::Menu => GameStateKind::Menu,
            GameState::Explore => GameStateKind::Explore,
            GameState::Inventory => GameStateKind::Inventory,
            GameState::Stats => GameStateKind::Stats,
            GameState::Dialog => GameStateKind::Dialog,
            GameState::Shop => GameStateKind::Shop,
            GameState::QuestLog => GameStateKind::QuestLog,
            GameState::PauseMenu => GameStateKind::PauseMenu,
            GameState::GameOver => GameStateKind::GameOver,
            GameState::Error(_) => GameStateKind::Error,
        }
    }

    pub fn requires_session(&self) -> bool {
        matches!(
            self,
            GameState::Explore
                | GameState::Inventory
                | GameState::Stats
                | GameState::Dialog
                | GameState::Shop
                | GameState::QuestLog
                | GameState::PauseMenu
                | GameState::GameOver
        )
    }

    pub fn can_transition_to(&self, next: &GameState) -> bool {
        let current = self.kind();
        let target = next.kind();

        if current == target {
            return true;
        }

        match current {
            GameStateKind::Loading => {
                matches!(target, GameStateKind::Menu | GameStateKind::Error)
            }
            GameStateKind::Menu => {
                matches!(
                    target,
                    GameStateKind::Explore | GameStateKind::Dialog | GameStateKind::Error
                )
            }
            GameStateKind::Explore => matches!(
                target,
                GameStateKind::Menu
                    | GameStateKind::Inventory
                    | GameStateKind::Dialog
                    | GameStateKind::Shop
                    | GameStateKind::PauseMenu
                    | GameStateKind::GameOver
                    | GameStateKind::Error
            ),
            GameStateKind::Inventory => {
                matches!(target, GameStateKind::Explore | GameStateKind::Error)
            }
            GameStateKind::Stats => matches!(target, GameStateKind::Explore | GameStateKind::Error),
            GameStateKind::Dialog => {
                matches!(
                    target,
                    GameStateKind::Explore | GameStateKind::Shop | GameStateKind::Error
                )
            }
            GameStateKind::Shop => matches!(target, GameStateKind::Explore | GameStateKind::Error),
            GameStateKind::QuestLog => {
                matches!(target, GameStateKind::Explore | GameStateKind::Error)
            }
            GameStateKind::PauseMenu => matches!(
                target,
                GameStateKind::Explore
                    | GameStateKind::Inventory
                    | GameStateKind::Stats
                    | GameStateKind::QuestLog
                    | GameStateKind::Error
            ),
            GameStateKind::GameOver => matches!(target, GameStateKind::Menu | GameStateKind::Error),
            GameStateKind::Error => false,
        }
    }
}

struct GameStateApplier;

static GAME_STATE_APPLIER: GameStateApplier = GameStateApplier;

pub fn state_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&GAME_STATE_APPLIER]
}

impl DomainEventApplier for GameStateApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Loading(_) | GameEvent::Transition(_) | GameEvent::Exit(_)
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Loading(event) => match event {
                LoadingEvent::Advance(step) => ctx.transition_to(GameState::Loading(*step)),
                LoadingEvent::Loaded => {
                    ctx.transition_to(GameState::Menu);
                    ctx.ui.menu.set_menu(MenuState::new(has_save_data()));
                }
                LoadingEvent::Error(msg) => ctx.set_error(msg.clone()),
            },
            GameEvent::Transition(TransitionEvent::ToExplore) => {
                ctx.transition_to(GameState::Explore)
            }
            GameEvent::Transition(TransitionEvent::ToMenuFromGameOver) => {
                ctx.transition_to(GameState::Menu);
                ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
            }
            GameEvent::Exit(code) => {
                wipi::kernel::exit(*code);
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GameState;

    fn states() -> [GameState; 11] {
        [
            GameState::Loading(0),
            GameState::Menu,
            GameState::Explore,
            GameState::Inventory,
            GameState::Stats,
            GameState::Dialog,
            GameState::Shop,
            GameState::QuestLog,
            GameState::PauseMenu,
            GameState::GameOver,
            GameState::Error(alloc::string::String::from("error")),
        ]
    }

    fn expected_allowed(from: &GameState, to: &GameState) -> bool {
        use GameState as S;

        match from {
            S::Loading(_) => matches!(to, S::Loading(_) | S::Menu | S::Error(_)),
            S::Menu => matches!(to, S::Menu | S::Explore | S::Dialog | S::Error(_)),
            S::Explore => matches!(
                to,
                S::Explore
                    | S::Menu
                    | S::Inventory
                    | S::Dialog
                    | S::Shop
                    | S::PauseMenu
                    | S::GameOver
                    | S::Error(_)
            ),
            S::Inventory => matches!(to, S::Inventory | S::Explore | S::Error(_)),
            S::Stats => matches!(to, S::Stats | S::Explore | S::Error(_)),
            S::Dialog => matches!(to, S::Dialog | S::Explore | S::Shop | S::Error(_)),
            S::Shop => matches!(to, S::Shop | S::Explore | S::Error(_)),
            S::QuestLog => matches!(to, S::QuestLog | S::Explore | S::Error(_)),
            S::PauseMenu => matches!(
                to,
                S::PauseMenu | S::Explore | S::Inventory | S::Stats | S::QuestLog | S::Error(_)
            ),
            S::GameOver => matches!(to, S::GameOver | S::Menu | S::Error(_)),
            S::Error(_) => matches!(to, S::Error(_)),
        }
    }

    #[test]
    fn game_state_transitions_allow_expected_paths() {
        assert!(GameState::Loading(0).can_transition_to(&GameState::Menu));
        assert!(GameState::Menu.can_transition_to(&GameState::Explore));
        assert!(GameState::Menu.can_transition_to(&GameState::Dialog));
        assert!(GameState::Explore.can_transition_to(&GameState::PauseMenu));
        assert!(GameState::PauseMenu.can_transition_to(&GameState::QuestLog));
        assert!(GameState::Dialog.can_transition_to(&GameState::Shop));
        assert!(GameState::GameOver.can_transition_to(&GameState::Menu));
    }

    #[test]
    fn game_state_transitions_reject_invalid_paths() {
        assert!(!GameState::Menu.can_transition_to(&GameState::Shop));
        assert!(!GameState::Inventory.can_transition_to(&GameState::Dialog));
        assert!(!GameState::Stats.can_transition_to(&GameState::PauseMenu));
        assert!(!GameState::GameOver.can_transition_to(&GameState::Explore));
    }

    #[test]
    fn game_state_transition_matrix_matches_rules_table() {
        let all = states();

        for from in &all {
            for to in &all {
                assert_eq!(
                    from.can_transition_to(to),
                    expected_allowed(from, to),
                    "transition mismatch: {:?} -> {:?}",
                    from,
                    to
                );
            }
        }
    }
}
