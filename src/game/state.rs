mod combat;
mod movement;
mod player;

pub use combat::{CombatAction, CombatEvent, CombatState, FieldEnemy, PlayerEffect};
pub use movement::{MovementState, MovementTickEvent};
pub use player::{PlayerAction, PlayerEvent, PlayerState, TileApplyEvent, TileEvent};

use alloc::string::String;

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

#[cfg(test)]
mod tests {
    use super::GameState;

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
}
