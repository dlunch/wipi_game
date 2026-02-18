use alloc::format;
use alloc::string::String;

use anyhow::Result;

use crate::game::{GameEvent, GameEventKind, GameEventSubscriber, LoadingEvent, TransitionEvent};

#[derive(Debug)]
pub enum GameState {
    Loading(usize),
    Menu,
    Explore,
    Dead,
    Inventory,
    Stats,
    Dialog,
    Shop,
    QuestLog,
    PauseMenu,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameStateKind {
    Loading,
    Menu,
    Explore,
    Dead,
    Inventory,
    Stats,
    Dialog,
    Shop,
    QuestLog,
    PauseMenu,
    Error,
}

impl GameState {
    fn kind(&self) -> GameStateKind {
        match self {
            GameState::Loading(_) => GameStateKind::Loading,
            GameState::Menu => GameStateKind::Menu,
            GameState::Explore => GameStateKind::Explore,
            GameState::Dead => GameStateKind::Dead,
            GameState::Inventory => GameStateKind::Inventory,
            GameState::Stats => GameStateKind::Stats,
            GameState::Dialog => GameStateKind::Dialog,
            GameState::Shop => GameStateKind::Shop,
            GameState::QuestLog => GameStateKind::QuestLog,
            GameState::PauseMenu => GameStateKind::PauseMenu,
            GameState::Error(_) => GameStateKind::Error,
        }
    }

    pub fn requires_world(&self) -> bool {
        matches!(
            self,
            GameState::Explore
                | GameState::Dead
                | GameState::Inventory
                | GameState::Stats
                | GameState::Dialog
                | GameState::Shop
                | GameState::QuestLog
                | GameState::PauseMenu
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
                    | GameStateKind::Dead
                    | GameStateKind::Inventory
                    | GameStateKind::Dialog
                    | GameStateKind::Shop
                    | GameStateKind::PauseMenu
                    | GameStateKind::Error
            ),
            GameStateKind::Dead => {
                matches!(
                    target,
                    GameStateKind::Explore | GameStateKind::Menu | GameStateKind::Error
                )
            }
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
            GameStateKind::Error => false,
        }
    }
}

impl GameEventSubscriber for GameState {
    fn subscribes(&self, kind: GameEventKind) -> bool {
        matches!(
            kind,
            GameEventKind::Loading
                | GameEventKind::Transition
                | GameEventKind::ApplyDialogTransition
                | GameEventKind::OpenDialogState
                | GameEventKind::OpenShopState
        )
    }
}

impl GameState {
    pub fn apply_event(&mut self, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::Loading(event) => match event {
                LoadingEvent::Tick => {}
                LoadingEvent::Advance(step) => self.transition_to(GameState::Loading(*step)),
                LoadingEvent::Loaded => self.transition_to(GameState::Menu),
                LoadingEvent::Error(msg) => *self = GameState::Error(msg.clone()),
            },
            GameEvent::Transition(TransitionEvent::ToExplore)
            | GameEvent::ApplyDialogTransition(crate::game::DialogTransition::CloseToExplore) => {
                self.transition_to(GameState::Explore)
            }
            GameEvent::Transition(TransitionEvent::ToDead) => {
                self.transition_to(GameState::Dead);
            }
            GameEvent::Transition(TransitionEvent::ToMenu) => {
                self.transition_to(GameState::Menu);
            }
            GameEvent::Transition(TransitionEvent::ToPauseMenu) => {
                self.transition_to(GameState::PauseMenu);
            }
            GameEvent::Transition(TransitionEvent::ToInventory) => {
                self.transition_to(GameState::Inventory);
            }
            GameEvent::Transition(TransitionEvent::ToStats) => {
                self.transition_to(GameState::Stats);
            }
            GameEvent::Transition(TransitionEvent::ToQuestLog) => {
                self.transition_to(GameState::QuestLog);
            }
            GameEvent::ApplyDialogTransition(crate::game::DialogTransition::SetLine(_))
            | GameEvent::OpenDialogState(_) => self.transition_to(GameState::Dialog),
            GameEvent::OpenShopState(_) => self.transition_to(GameState::Shop),
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn transition_to(&mut self, next: GameState) {
        if self.can_transition_to(&next) {
            *self = next;
            return;
        }

        *self = GameState::Error(format!(
            "Invalid state transition: {:?} -> {:?}",
            self, next
        ));
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::GameState;

    fn states() -> [GameState; 11] {
        [
            GameState::Loading(0),
            GameState::Menu,
            GameState::Explore,
            GameState::Dead,
            GameState::Inventory,
            GameState::Stats,
            GameState::Dialog,
            GameState::Shop,
            GameState::QuestLog,
            GameState::PauseMenu,
            GameState::Error(String::from("error")),
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
                    | S::Dead
                    | S::Inventory
                    | S::Dialog
                    | S::Shop
                    | S::PauseMenu
                    | S::Error(_)
            ),
            S::Dead => matches!(to, S::Dead | S::Explore | S::Menu | S::Error(_)),
            S::Inventory => matches!(to, S::Inventory | S::Explore | S::Error(_)),
            S::Stats => matches!(to, S::Stats | S::Explore | S::Error(_)),
            S::Dialog => matches!(to, S::Dialog | S::Explore | S::Shop | S::Error(_)),
            S::Shop => matches!(to, S::Shop | S::Explore | S::Error(_)),
            S::QuestLog => matches!(to, S::QuestLog | S::Explore | S::Error(_)),
            S::PauseMenu => matches!(
                to,
                S::PauseMenu | S::Explore | S::Inventory | S::Stats | S::QuestLog | S::Error(_)
            ),
            S::Error(_) => matches!(to, S::Error(_)),
        }
    }

    #[test]
    fn game_state_transitions_allow_expected_paths() {
        assert!(GameState::Loading(0).can_transition_to(&GameState::Menu));
        assert!(GameState::Menu.can_transition_to(&GameState::Explore));
        assert!(GameState::Menu.can_transition_to(&GameState::Dialog));
        assert!(GameState::Explore.can_transition_to(&GameState::Dead));
        assert!(GameState::Dead.can_transition_to(&GameState::Explore));
        assert!(GameState::Explore.can_transition_to(&GameState::PauseMenu));
        assert!(GameState::PauseMenu.can_transition_to(&GameState::QuestLog));
        assert!(GameState::Dialog.can_transition_to(&GameState::Shop));
    }

    #[test]
    fn game_state_transitions_reject_invalid_paths() {
        assert!(!GameState::Menu.can_transition_to(&GameState::Shop));
        assert!(!GameState::Inventory.can_transition_to(&GameState::Dialog));
        assert!(!GameState::Dead.can_transition_to(&GameState::Dialog));
        assert!(!GameState::Stats.can_transition_to(&GameState::PauseMenu));
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
