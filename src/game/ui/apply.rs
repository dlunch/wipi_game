use alloc::vec::Vec;

use super::state::{DialogTransition, ExploreCommand, InputKey, MenuAction, UiEvent, UiState};
use crate::data::DialogAction;
use crate::game::selection::{step_down, step_up};
use crate::game::{GameEvent, TransitionEvent, WorldState};

pub trait UiEventApplier {
    fn apply_ui_event(
        &mut self,
        session: Option<&WorldState>,
        event: UiEvent,
        out: &mut Vec<GameEvent>,
    );
}

#[derive(Debug, Clone, Copy)]
enum MenuEvent {
    None,
    SetSelected(usize),
    Action(MenuAction),
}

#[derive(Clone, Copy)]
enum PauseMenuAction {
    None,
    OpenInventory,
    OpenStats,
    OpenQuestLog,
    SaveAndReturnExplore,
    BackToExplore,
}

impl UiEventApplier for UiState {
    fn apply_ui_event(
        &mut self,
        session: Option<&WorldState>,
        event: UiEvent,
        out: &mut Vec<GameEvent>,
    ) {
        match event {
            UiEvent::OverlayCloseRequested => {
                out.push(GameEvent::Transition(TransitionEvent::ToExplore))
            }
            UiEvent::GameOverConfirmRequested => {
                out.push(GameEvent::Transition(TransitionEvent::ToMenuFromGameOver))
            }
            UiEvent::ErrorConfirmRequested => out.push(GameEvent::Exit(1)),
            UiEvent::MovementKeyReleased(direction) => out.push(GameEvent::Transition(
                TransitionEvent::ReleaseMovementDirection(direction),
            )),
            UiEvent::MenuInput(key) => apply_menu_input(self, key, out),
            UiEvent::PauseMenuInput(key) => apply_pause_menu_input(self, key, out),
            UiEvent::ExploreInput(key) => apply_explore_input(key, out),
            UiEvent::InventoryInput(key) => apply_inventory_input(self, session, key, out),
            UiEvent::DialogInput(key) => apply_dialog_input(self, key, out),
            UiEvent::ShopBuySelected(selected) => {
                apply_shop_buy_selected(self, session, selected, out)
            }
            UiEvent::ShopSellSelected(selected) => apply_shop_sell_selected(selected, out),
            UiEvent::ShopClose => out.push(GameEvent::Transition(TransitionEvent::ToExplore)),
        };
    }
}

fn apply_explore_input(key: InputKey, out: &mut Vec<GameEvent>) {
    let event = match key {
        InputKey::Up => Some(ExploreCommand::Move(crate::data::Direction::Up)),
        InputKey::Down => Some(ExploreCommand::Move(crate::data::Direction::Down)),
        InputKey::Left => Some(ExploreCommand::Move(crate::data::Direction::Left)),
        InputKey::Right => Some(ExploreCommand::Move(crate::data::Direction::Right)),
        InputKey::Ok => Some(ExploreCommand::Confirm),
        InputKey::Key1 => Some(ExploreCommand::UseSlot(0)),
        InputKey::Key2 => Some(ExploreCommand::UseSlot(1)),
        InputKey::Key3 => Some(ExploreCommand::UseSlot(2)),
        InputKey::Key0 => Some(ExploreCommand::OpenPauseMenu),
        InputKey::Back => Some(ExploreCommand::OpenMenu),
        _ => None,
    };

    if let Some(command) = event {
        out.push(GameEvent::ExploreCommand(command));
    }
}

fn apply_inventory_input(
    ui: &mut UiState,
    session: Option<&WorldState>,
    key: InputKey,
    out: &mut Vec<GameEvent>,
) {
    let Some(s) = session else {
        return;
    };

    let selected = ui.inventory.selected;
    match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                ui.inventory.set_selected(next);
            }
        }
        InputKey::Down => {
            let next = step_down(selected, s.leader.inventory.len());
            if next != selected {
                ui.inventory.set_selected(next);
            }
        }
        InputKey::Ok => {
            out.push(GameEvent::UseInventorySelected(selected));
        }
        InputKey::Back => {
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
        _ => {}
    }
}

fn apply_dialog_input(ui: &UiState, key: InputKey, out: &mut Vec<GameEvent>) {
    match key {
        InputKey::Back => out.push(GameEvent::ApplyDialogTransition(
            DialogTransition::CloseToExplore,
        )),
        InputKey::Ok => {
            if let Some(dialog_state_ref) = ui.dialog.state.as_ref() {
                if dialog_state_ref.current_line >= dialog_state_ref.lines.len() {
                    out.push(GameEvent::ApplyDialogTransition(
                        DialogTransition::CloseToExplore,
                    ));
                    return;
                }

                let transition = if dialog_state_ref.current_line + 1 < dialog_state_ref.lines.len()
                {
                    DialogTransition::SetLine(dialog_state_ref.current_line + 1)
                } else {
                    DialogTransition::CloseToExplore
                };

                out.push(GameEvent::ApplyDialogTransition(transition));
                if let Some(action) = dialog_state_ref
                    .lines
                    .get(dialog_state_ref.current_line)
                    .and_then(|line| line.action.as_ref())
                    .cloned()
                {
                    match action {
                        DialogAction::OpenShop(shop_id) => {
                            out.push(GameEvent::OpenShopById(shop_id));
                        }
                        _ => out.push(GameEvent::ApplyDialogAction(action)),
                    }
                }
            }
        }
        _ => {}
    }
}

fn apply_shop_buy_selected(
    ui: &UiState,
    session: Option<&WorldState>,
    selected: usize,
    out: &mut Vec<GameEvent>,
) {
    let Some(s) = session else {
        return;
    };
    let shop_items = ui
        .shop
        .state
        .as_ref()
        .map(|state| state.items.as_slice())
        .unwrap_or(&[]);
    if let Some(item) = shop_items.get(selected).cloned()
        && s.leader.stats.gold >= item.price
    {
        out.push(GameEvent::ShopBuyItem(item));
    }
}

fn apply_shop_sell_selected(selected: usize, out: &mut Vec<GameEvent>) {
    out.push(GameEvent::ShopSellSelected(selected));
}

fn apply_menu_input(ui: &mut UiState, key: InputKey, out: &mut Vec<GameEvent>) {
    let selected = ui.menu.selected;
    let items = &ui.menu.state.items;

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
        MenuEvent::None => {}
        MenuEvent::SetSelected(selected) => {
            ui.menu.set_selected(selected);
        }
        MenuEvent::Action(action) => match action {
            MenuAction::NewGame => out.push(GameEvent::StartNewGame),
            MenuAction::Continue => out.push(GameEvent::ContinueGame),
            MenuAction::Exit => out.push(GameEvent::Exit(0)),
        },
    }
}

fn apply_pause_menu_input(ui: &mut UiState, key: InputKey, out: &mut Vec<GameEvent>) {
    let selected = ui.pause_menu.selected;
    let item_count = ui.pause_menu.state.items.len();

    let action = match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                ui.pause_menu.set_selected(next);
            }
            PauseMenuAction::None
        }
        InputKey::Down => {
            let next = step_down(selected, item_count);
            if next != selected {
                ui.pause_menu.set_selected(next);
            }
            PauseMenuAction::None
        }
        InputKey::Ok => match selected {
            0 => PauseMenuAction::OpenInventory,
            1 => PauseMenuAction::OpenStats,
            2 => PauseMenuAction::OpenQuestLog,
            3 => PauseMenuAction::SaveAndReturnExplore,
            _ => PauseMenuAction::None,
        },
        InputKey::Back | InputKey::Key0 => PauseMenuAction::BackToExplore,
        _ => PauseMenuAction::None,
    };

    match action {
        PauseMenuAction::None => {}
        PauseMenuAction::OpenInventory => {
            ui.inventory.reset();
            out.push(GameEvent::Transition(TransitionEvent::ToInventory));
        }
        PauseMenuAction::OpenStats => {
            out.push(GameEvent::Transition(TransitionEvent::ToStats));
        }
        PauseMenuAction::OpenQuestLog => {
            out.push(GameEvent::Transition(TransitionEvent::ToQuestLog));
        }
        PauseMenuAction::SaveAndReturnExplore => {
            ui.shop.reset();
            out.push(GameEvent::SaveWorld);
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
        PauseMenuAction::BackToExplore => {
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
    }
}
