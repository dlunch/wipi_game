use alloc::vec::Vec;

use super::state::{DialogTransition, InputKey, MenuAction, ShopMode, UiEvent, UiState};
use crate::data::DialogAction;
use crate::game::selection::{step_down, step_up};
use crate::game::{ExploreEvent, GameEvent, TransitionEvent, WorldState};

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
    match key {
        InputKey::Up => out.push(GameEvent::Explore(ExploreEvent::MoveDirection(
            crate::data::Direction::Up,
        ))),
        InputKey::Down => out.push(GameEvent::Explore(ExploreEvent::MoveDirection(
            crate::data::Direction::Down,
        ))),
        InputKey::Left => out.push(GameEvent::Explore(ExploreEvent::MoveDirection(
            crate::data::Direction::Left,
        ))),
        InputKey::Right => out.push(GameEvent::Explore(ExploreEvent::MoveDirection(
            crate::data::Direction::Right,
        ))),
        InputKey::Ok => {
            if let Some(s) = session {
                out.push(GameEvent::Explore(ExploreEvent::TryNpcInteract {
                    facing: s.leader.facing,
                    fallback_action: Some(ui.explore.ok_action),
                }));
            }
        }
        InputKey::Key1 => {
            if let Some(action) = ui.explore.key_actions.first().and_then(|a| *a) {
                out.push(GameEvent::CombatPlayerAction(action));
            }
        }
        InputKey::Key2 => {
            if let Some(action) = ui.explore.key_actions.get(1).and_then(|a| *a) {
                out.push(GameEvent::CombatPlayerAction(action));
            }
        }
        InputKey::Key3 => {
            if let Some(action) = ui.explore.key_actions.get(2).and_then(|a| *a) {
                out.push(GameEvent::CombatPlayerAction(action));
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
            let next = step_down(selected, s.leader.inventory.len());
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
    let inventory_len = session.map(|s| s.leader.inventory.len()).unwrap_or(0);

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
                apply_shop_buy_selected(ui, session, ui.shop.selected, out);
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
                out.push(GameEvent::ShopSellSelected(ui.shop.selected));
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
            ui.menu.selected = selected;
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
                ui.pause_menu.selected = next;
            }
            PauseMenuAction::None
        }
        InputKey::Down => {
            let next = step_down(selected, item_count);
            if next != selected {
                ui.pause_menu.selected = next;
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
            ui.inventory.selected = 0;
            out.push(GameEvent::Transition(TransitionEvent::ToInventory));
        }
        PauseMenuAction::OpenStats => {
            out.push(GameEvent::Transition(TransitionEvent::ToStats));
        }
        PauseMenuAction::OpenQuestLog => {
            out.push(GameEvent::Transition(TransitionEvent::ToQuestLog));
        }
        PauseMenuAction::SaveAndReturnExplore => {
            ui.shop = Default::default();
            out.push(GameEvent::SaveWorld);
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
        PauseMenuAction::BackToExplore => {
            out.push(GameEvent::Transition(TransitionEvent::ToExplore));
        }
    }
}
