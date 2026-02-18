use alloc::string::String;
use alloc::vec::Vec;

use super::state::{DialogTransition, InputKey, MenuAction, ShopMode, UiEvent, UiState};
use crate::data::DialogAction;
use crate::game::selection::{step_down, step_up};
use crate::game::{ExploreEvent, GOLD_ITEM_ID, GameEvent, TransitionEvent, WorldState};

pub trait UiEventApplier {
    fn apply_ui_event(
        &mut self,
        session: Option<&WorldState>,
        event: UiEvent,
        out: &mut Vec<GameEvent>,
    );
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
            UiEvent::ReviveRequested => out.push(GameEvent::RevivePlayer),
            UiEvent::ErrorConfirmRequested => out.push(GameEvent::Exit(1)),
            UiEvent::MovementKeyReleased(direction) => out.push(GameEvent::Transition(
                TransitionEvent::ReleaseMovementDirection(direction),
            )),
            UiEvent::MenuInput(key) => apply_menu_input(self, key, out),
            UiEvent::PauseMenuInput(key) => apply_pause_menu_input(self, key, out),
            UiEvent::ExploreInput(key) => apply_explore_input(self, session, key, out),
            UiEvent::InventoryInput(key) => apply_inventory_input(self, session, key, out),
            UiEvent::QuestLogInput(key) => apply_quest_log_input(self, session, key, out),
            UiEvent::DialogInput(key) => apply_dialog_input(self, key, out),
            UiEvent::ShopInput(key) => apply_shop_input(self, session, key, out),
        };
    }
}

fn apply_explore_input(
    ui: &UiState,
    session: Option<&WorldState>,
    key: InputKey,
    out: &mut Vec<GameEvent>,
) {
    if let Some(direction) = key.direction() {
        out.push(GameEvent::Explore(ExploreEvent::MoveDirection(direction)));
        return;
    }

    if let Some(action_index) = explore_action_index(key) {
        if let Some(action) = ui.explore.key_actions.get(action_index).and_then(|a| *a) {
            out.push(GameEvent::CombatPlayerAction(action));
        }
        return;
    }

    match key {
        InputKey::Ok => {
            if let Some(s) = session
                && let Ok(leader) = s.leader_entity()
            {
                out.push(GameEvent::Explore(ExploreEvent::TryNpcInteract {
                    facing: leader.facing,
                    fallback_action: Some(ui.explore.ok_action),
                }));
            }
        }
        InputKey::Key0 => out.push(GameEvent::Transition(TransitionEvent::ToPauseMenu)),
        InputKey::Back => {
            out.push(GameEvent::SaveWorld);
            out.push(GameEvent::Transition(TransitionEvent::ToMenu));
        }
        _ => {}
    }
}

fn explore_action_index(key: InputKey) -> Option<usize> {
    match key {
        InputKey::Key1 => Some(0),
        InputKey::Key2 => Some(1),
        InputKey::Key3 => Some(2),
        _ => None,
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
                ui.inventory.selected = next;
            }
        }
        InputKey::Down => {
            let inventory_len = s
                .leader_entity()
                .ok()
                .map(|leader| leader.inventory.len())
                .unwrap_or(0);
            let next = step_down(selected, inventory_len);
            if next != selected {
                ui.inventory.selected = next;
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

fn apply_quest_log_input(
    ui: &mut UiState,
    session: Option<&WorldState>,
    key: InputKey,
    out: &mut Vec<GameEvent>,
) {
    let Some(s) = session else {
        return;
    };

    let mut active_quest_ids = Vec::with_capacity(s.quests.len());
    for quest in &s.quests {
        if !quest.rewarded {
            active_quest_ids.push(quest.quest_id.clone());
        }
    }

    match key {
        InputKey::Up => {
            ui.quest_log.selected = step_up(ui.quest_log.selected);
        }
        InputKey::Down => {
            ui.quest_log.selected = step_down(ui.quest_log.selected, active_quest_ids.len());
        }
        InputKey::Ok => {
            let Some(quest_id) = active_quest_ids.get(ui.quest_log.selected).cloned() else {
                return;
            };
            if ui.quest_log.tracked_quest_id.as_deref() == Some(quest_id.as_str()) {
                ui.quest_log.tracked_quest_id = None;
            } else {
                ui.quest_log.tracked_quest_id = Some(quest_id);
            }
        }
        InputKey::Back => {
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
        _ => {}
    }
}

fn apply_shop_input(
    ui: &mut UiState,
    session: Option<&WorldState>,
    key: InputKey,
    out: &mut Vec<GameEvent>,
) {
    let shop_items_len = ui
        .shop
        .state
        .as_ref()
        .map(|state| state.items.len())
        .unwrap_or(0);
    let inventory_len = session
        .and_then(|s| s.leader_entity().ok().map(|leader| leader.inventory.len()))
        .unwrap_or(0);

    match ui.shop.mode {
        ShopMode::Select => match key {
            InputKey::Up => {
                ui.shop.selected = step_up(ui.shop.selected);
            }
            InputKey::Down => {
                ui.shop.selected = step_down(ui.shop.selected, 2);
            }
            InputKey::Ok => {
                if ui.shop.selected == 0 {
                    ui.shop.mode = ShopMode::Buy;
                } else {
                    ui.shop.mode = ShopMode::Sell;
                }
                ui.shop.selected = 0;
            }
            InputKey::Back => out.push(GameEvent::Transition(TransitionEvent::ToExplore)),
            _ => {}
        },
        ShopMode::Buy => match key {
            InputKey::Up => {
                ui.shop.selected = step_up(ui.shop.selected);
            }
            InputKey::Down => {
                ui.shop.selected = step_down(ui.shop.selected, shop_items_len);
            }
            InputKey::Ok => {
                if shop_items_len > 0 {
                    ui.shop.mode = ShopMode::ConfirmBuy;
                }
            }
            InputKey::Back => {
                ui.shop.mode = ShopMode::Select;
                ui.shop.selected = 0;
            }
            _ => {}
        },
        ShopMode::ConfirmBuy => match key {
            InputKey::Ok => {
                if let Some(s) = session
                    && let Ok(leader_id) = s.leader_id()
                {
                    let shop_items = ui
                        .shop
                        .state
                        .as_ref()
                        .map(|state| state.items.as_slice())
                        .unwrap_or(&[]);
                    if let Some(item) = shop_items.get(ui.shop.selected).cloned()
                        && s.gold_amount(leader_id) >= item.price
                    {
                        out.push(GameEvent::ShopBuyItem(item.id));
                    } else {
                        out.push(GameEvent::SoftError(String::from("Not enough gold")));
                    }
                } else {
                    out.push(GameEvent::SoftError(String::from("No active world")));
                }
                ui.shop.mode = ShopMode::Buy;
            }
            InputKey::Back => {
                ui.shop.mode = ShopMode::Buy;
            }
            _ => {}
        },
        ShopMode::Sell => match key {
            InputKey::Up => {
                ui.shop.selected = step_up(ui.shop.selected);
            }
            InputKey::Down => {
                ui.shop.selected = step_down(ui.shop.selected, inventory_len);
            }
            InputKey::Ok => {
                if inventory_len > 0 {
                    ui.shop.mode = ShopMode::ConfirmSell;
                }
            }
            InputKey::Back => {
                ui.shop.mode = ShopMode::Select;
                ui.shop.selected = 0;
            }
            _ => {}
        },
        ShopMode::ConfirmSell => match key {
            InputKey::Ok => {
                if let Some(s) = session
                    && let Ok(leader) = s.leader_entity()
                    && let Some(item) = leader.inventory.get(ui.shop.selected)
                {
                    if item.item_id == GOLD_ITEM_ID {
                        out.push(GameEvent::SoftError(String::from("Cannot sell gold")));
                    } else {
                        out.push(GameEvent::ShopSellSelected(ui.shop.selected));
                    }
                } else {
                    out.push(GameEvent::SoftError(String::from("Invalid item selection")));
                }
                ui.shop.mode = ShopMode::Sell;
            }
            InputKey::Back => {
                ui.shop.mode = ShopMode::Sell;
            }
            _ => {}
        },
    }
}

fn apply_menu_input(ui: &mut UiState, key: InputKey, out: &mut Vec<GameEvent>) {
    let selected = ui.menu.selected;
    let items = &ui.menu.state.items;

    match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                ui.menu.selected = next;
            }
        }
        InputKey::Down => {
            let next = step_down(selected, items.len());
            if next != selected {
                ui.menu.selected = next;
            }
        }
        InputKey::Ok => {
            if let Some((_, action)) = items.get(selected).copied() {
                match action {
                    MenuAction::NewGame => out.push(GameEvent::StartNewGame),
                    MenuAction::Continue => out.push(GameEvent::ContinueGame),
                    MenuAction::Exit => out.push(GameEvent::Exit(0)),
                }
            }
        }
        _ => {}
    }
}

fn apply_pause_menu_input(ui: &mut UiState, key: InputKey, out: &mut Vec<GameEvent>) {
    let selected = ui.pause_menu.selected;
    let item_count = ui.pause_menu.state.items.len();

    match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                ui.pause_menu.selected = next;
            }
        }
        InputKey::Down => {
            let next = step_down(selected, item_count);
            if next != selected {
                ui.pause_menu.selected = next;
            }
        }
        InputKey::Ok => match selected {
            0 => {
                ui.inventory.selected = 0;
                out.push(GameEvent::Transition(TransitionEvent::ToInventory));
            }
            1 => out.push(GameEvent::Transition(TransitionEvent::ToStats)),
            2 => out.push(GameEvent::Transition(TransitionEvent::ToQuestLog)),
            3 => {
                ui.shop = Default::default();
                out.push(GameEvent::SaveWorld);
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
            }
            _ => {}
        },
        InputKey::Back | InputKey::Key0 => {
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
        _ => {}
    }
}
