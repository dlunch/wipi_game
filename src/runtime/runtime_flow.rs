use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::{
    AppExploreEvent, Direction, GameRuntime, GameState, MenuAction, MenuEvent, MenuState,
    RuntimeEvent, TransitionEvent, has_save_data,
};
use crate::game::{SceneIntent, ShopIntent};

pub(super) fn apply_menu_event(runtime: &mut GameRuntime, event: MenuEvent) {
    match event {
        MenuEvent::None => {}
        MenuEvent::SetSelected(selected) => runtime.ui.menu.set_selected(selected),
        MenuEvent::Action(action) => match action {
            MenuAction::NewGame => {
                let (state, session, intro) = crate::game::lifecycle::start_new_game(&runtime.data);
                runtime.enter_session(state, session, intro);
            }
            MenuAction::Continue => {
                let (state, session, intro) = crate::game::lifecycle::continue_game(&runtime.data);
                runtime.enter_session(state, session, intro);
            }
            MenuAction::Exit => runtime.apply_event(RuntimeEvent::Exit(0)),
        },
    }
}

pub(super) fn apply_explore_event(runtime: &mut GameRuntime, event: AppExploreEvent) {
    match event {
        AppExploreEvent::MoveDirection(direction) => {
            let Some(s) = runtime.session.as_mut() else {
                runtime.state = GameState::Error(String::from("No active session"));
                return;
            };
            s.on_direction_pressed(direction);
        }
        AppExploreEvent::Npc(npc_event) => {
            let Some(s) = runtime.session.as_mut() else {
                runtime.state = GameState::Error(String::from("No active session"));
                return;
            };
            match npc_event {
                crate::game::NpcEvent::OpenDialog(dialog_spec) => {
                    if dialog_spec.restore {
                        s.restore_stats();
                    }
                    runtime.ui.dialog.open(crate::game::DialogState::new(
                        dialog_spec.npc_name,
                        dialog_spec.lines,
                    ));
                    runtime.transition_to(GameState::Dialog);
                }
                crate::game::NpcEvent::OpenShop(shop_id) => {
                    let _ = runtime.open_shop_by_id(&shop_id);
                }
                crate::game::NpcEvent::RestoreStats => {
                    s.restore_stats();
                }
            }
        }
        AppExploreEvent::UseAction(action) => {
            let Some(s) = runtime.session.as_mut() else {
                runtime.state = GameState::Error(String::from("No active session"));
                return;
            };
            s.apply_explore_action(&runtime.data, action);
        }
        AppExploreEvent::EnterPauseMenu => {
            runtime.ui.pause_menu.reset();
            runtime.transition_to(GameState::PauseMenu);
        }
        AppExploreEvent::EnterMenu => {
            let Some(s) = runtime.session.as_ref() else {
                runtime.state = GameState::Error(String::from("No active session"));
                return;
            };
            let _ = crate::game::save_game(&s.player);
            runtime.ui.menu.set_menu(MenuState::new(has_save_data()));
            runtime.transition_to(GameState::Menu);
        }
    }
}

pub(super) fn apply_inventory_event(runtime: &mut GameRuntime, event: crate::game::InventoryEvent) {
    let Some(s) = runtime.session.as_mut() else {
        runtime.state = GameState::Error(String::from("No active session"));
        return;
    };

    match event {
        crate::game::InventoryEvent::None => {}
        crate::game::InventoryEvent::SetSelected(selected) => {
            runtime.ui.inventory.set_selected(selected)
        }
        crate::game::InventoryEvent::UseSelected(index) => {
            s.use_inventory_item(index);
        }
        crate::game::InventoryEvent::CloseToExplore => runtime.transition_to(GameState::Explore),
    }
}

pub(super) fn apply_dialog_event(runtime: &mut GameRuntime, event: crate::game::DialogEvent) {
    let Some(s) = runtime.session.as_mut() else {
        runtime.state = GameState::Error(String::from("No active session"));
        return;
    };

    match event {
        crate::game::DialogEvent::None => {}
        crate::game::DialogEvent::Transition(transition) => match transition {
            crate::game::DialogTransition::SetLine(line) => {
                if let Some(dialog_state) = runtime.ui.dialog.state.as_mut() {
                    dialog_state.current_line = line;
                }
                runtime.transition_to(GameState::Dialog);
            }
            crate::game::DialogTransition::CloseToExplore => {
                runtime.ui.dialog.close();
                runtime.transition_to(GameState::Explore);
            }
        },
        crate::game::DialogEvent::Action(action, transition) => {
            match s.apply_dialog_action(&runtime.data, &action) {
                crate::game::DialogActionResult::None => {}
                crate::game::DialogActionResult::OpenShop(shop_id) => {
                    if runtime.open_shop_by_id(&shop_id) {
                        return;
                    }
                }
            }

            match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = runtime.ui.dialog.state.as_mut() {
                        dialog_state.current_line = line;
                    }
                    runtime.transition_to(GameState::Dialog);
                }
                crate::game::DialogTransition::CloseToExplore => {
                    runtime.ui.dialog.close();
                    runtime.transition_to(GameState::Explore);
                }
            }
        }
    }
}

pub(super) fn apply_shop_event(runtime: &mut GameRuntime, event: crate::game::ShopEvent) {
    let Some(s) = runtime.session.as_mut() else {
        runtime.state = GameState::Error(String::from("No active session"));
        return;
    };

    match event {
        crate::game::ShopEvent::None => {}
        crate::game::ShopEvent::BuyItem(item) => {
            s.buy_shop_item(item);
        }
        crate::game::ShopEvent::SellSelected(index) => {
            if s.sell_inventory_item(index).is_some() {
                let inv_len = s.player.inventory.len();
                if runtime.ui.shop.selected >= inv_len && runtime.ui.shop.selected > 0 {
                    runtime.ui.shop.set_selected(runtime.ui.shop.selected - 1);
                }
            }
        }
        crate::game::ShopEvent::CloseToExplore => runtime.transition_to(GameState::Explore),
    }
}

pub(super) fn apply_pause_menu_event(
    runtime: &mut GameRuntime,
    event: crate::game::PauseMenuEvent,
) {
    let Some(s) = runtime.session.as_mut() else {
        runtime.state = GameState::Error(String::from("No active session"));
        return;
    };

    match event {
        crate::game::PauseMenuEvent::None => {}
        crate::game::PauseMenuEvent::SetSelected(selected) => {
            runtime.ui.pause_menu.set_selected(selected)
        }
        crate::game::PauseMenuEvent::OpenInventory => {
            runtime.ui.inventory.reset();
            runtime.transition_to(GameState::Inventory);
        }
        crate::game::PauseMenuEvent::OpenStats => runtime.transition_to(GameState::Stats),
        crate::game::PauseMenuEvent::OpenQuestLog => runtime.transition_to(GameState::QuestLog),
        crate::game::PauseMenuEvent::SaveAndReturnExplore => {
            let _ = crate::game::save_game(&s.player);
            runtime.ui.shop.reset();
            runtime.transition_to(GameState::Explore);
        }
        crate::game::PauseMenuEvent::BackToExplore => runtime.transition_to(GameState::Explore),
    }
}

pub(super) fn apply_transition_event(runtime: &mut GameRuntime, event: TransitionEvent) {
    match event {
        TransitionEvent::MapChanged => runtime.apply_map_changed(),
        TransitionEvent::ToExplore => runtime.transition_to(GameState::Explore),
        TransitionEvent::ToMenuFromGameOver => {
            runtime.transition_to(GameState::Menu);
            runtime.ui.menu.set_menu(MenuState::new(has_save_data()));
        }
        TransitionEvent::ReleaseMovementDirection(direction) => {
            apply_release_movement_direction(runtime, direction)
        }
    }
}

pub(super) fn apply_release_movement_direction(runtime: &mut GameRuntime, direction: Direction) {
    if !matches!(runtime.state, GameState::Explore) {
        return;
    }
    let Some(s) = runtime.session.as_mut() else {
        return;
    };
    s.on_direction_released(direction);
}

pub(super) fn resolve_menu_intent(
    runtime: &GameRuntime,
    intent: crate::game::MenuIntent,
) -> Vec<RuntimeEvent> {
    if !matches!(runtime.state, GameState::Menu) {
        return vec![RuntimeEvent::None];
    }

    vec![RuntimeEvent::Domain(super::DomainEvent::Menu(
        crate::game::menu::resolve(
            runtime.ui.menu.selected,
            &runtime.ui.menu.state.items,
            intent,
        ),
    ))]
}

pub(super) fn resolve_explore_intent(
    runtime: &GameRuntime,
    intent: crate::game::ExploreIntent,
) -> Vec<RuntimeEvent> {
    if !matches!(runtime.state, GameState::Explore) {
        return vec![RuntimeEvent::None];
    }
    let Some(s) = runtime.session.as_ref() else {
        return vec![RuntimeEvent::Error(String::from("No active session"))];
    };

    let is_peaceful = runtime
        .data
        .find_map(&s.player.current_map_id)
        .is_some_and(|map| map.peaceful);

    let event = match crate::game::explore::resolve(is_peaceful, intent) {
        crate::game::ExploreEvent::None => RuntimeEvent::None,
        crate::game::ExploreEvent::MoveDirection(direction) => RuntimeEvent::Domain(
            super::DomainEvent::Explore(AppExploreEvent::MoveDirection(direction)),
        ),
        crate::game::ExploreEvent::TryNpcInteract {
            facing,
            fallback_action,
        } => {
            if let Some(npc_event) = crate::game::npc::resolve(
                &s.player,
                &runtime.data,
                crate::game::npc::NpcIntent::Interact { facing },
            ) {
                RuntimeEvent::Domain(super::DomainEvent::Explore(AppExploreEvent::Npc(npc_event)))
            } else if let Some(action) = fallback_action {
                RuntimeEvent::Domain(super::DomainEvent::Explore(AppExploreEvent::UseAction(
                    action,
                )))
            } else {
                RuntimeEvent::None
            }
        }
        crate::game::ExploreEvent::UseAction(action) => RuntimeEvent::Domain(
            super::DomainEvent::Explore(AppExploreEvent::UseAction(action)),
        ),
        crate::game::ExploreEvent::EnterPauseMenu => {
            RuntimeEvent::Domain(super::DomainEvent::Explore(AppExploreEvent::EnterPauseMenu))
        }
        crate::game::ExploreEvent::EnterMenu => {
            RuntimeEvent::Domain(super::DomainEvent::Explore(AppExploreEvent::EnterMenu))
        }
    };
    vec![event]
}

pub(super) fn resolve_inventory_intent(
    runtime: &GameRuntime,
    intent: crate::game::InventoryIntent,
) -> Vec<RuntimeEvent> {
    if !matches!(runtime.state, GameState::Inventory) {
        return vec![RuntimeEvent::None];
    }
    let Some(s) = runtime.session.as_ref() else {
        return vec![RuntimeEvent::Error(String::from("No active session"))];
    };

    vec![RuntimeEvent::Domain(super::DomainEvent::Inventory(
        crate::game::inventory::resolve(
            runtime.ui.inventory.selected,
            s.player.inventory.len(),
            intent,
        ),
    ))]
}

pub(super) fn resolve_dialog_intent(
    runtime: &GameRuntime,
    intent: crate::game::DialogIntent,
) -> Vec<RuntimeEvent> {
    if !matches!(runtime.state, GameState::Dialog) {
        return vec![RuntimeEvent::None];
    }
    if !runtime.session.is_active() {
        return vec![RuntimeEvent::Error(String::from("No active session"))];
    }

    vec![RuntimeEvent::Domain(super::DomainEvent::Dialog(
        crate::game::dialog::resolve(runtime.ui.dialog.state.as_ref(), intent),
    ))]
}

pub(super) fn resolve_shop_intent(runtime: &GameRuntime, intent: ShopIntent) -> Vec<RuntimeEvent> {
    if !matches!(runtime.state, GameState::Shop) {
        return vec![RuntimeEvent::None];
    }
    let Some(s) = runtime.session.as_ref() else {
        return vec![RuntimeEvent::Error(String::from("No active session"))];
    };
    let shop_items = runtime
        .ui
        .shop
        .state
        .as_ref()
        .map(|state| state.items.as_slice())
        .unwrap_or(&[]);

    vec![RuntimeEvent::Domain(super::DomainEvent::Shop(
        crate::game::shop::resolve(intent, s.player.stats.gold, shop_items),
    ))]
}

pub(super) fn resolve_pause_menu_intent(
    runtime: &GameRuntime,
    intent: crate::game::PauseMenuIntent,
) -> Vec<RuntimeEvent> {
    if !matches!(runtime.state, GameState::PauseMenu) {
        return vec![RuntimeEvent::None];
    }
    if !runtime.session.is_active() {
        return vec![RuntimeEvent::Error(String::from("No active session"))];
    }

    vec![RuntimeEvent::Domain(super::DomainEvent::PauseMenu(
        crate::game::menu::resolve_pause(
            runtime.ui.pause_menu.selected,
            runtime.ui.pause_menu.state.items.len(),
            intent,
        ),
    ))]
}

pub(super) fn resolve_scene_intent(
    runtime: &GameRuntime,
    scene_intent: SceneIntent,
) -> Vec<RuntimeEvent> {
    match scene_intent {
        SceneIntent::Menu(intent) => resolve_menu_intent(runtime, intent),
        SceneIntent::Explore(intent) => resolve_explore_intent(runtime, intent),
        SceneIntent::Inventory(intent) => resolve_inventory_intent(runtime, intent),
        SceneIntent::Dialog(intent) => resolve_dialog_intent(runtime, intent),
        SceneIntent::Shop(intent) => resolve_shop_intent(runtime, intent),
        SceneIntent::PauseMenu(intent) => resolve_pause_menu_intent(runtime, intent),
    }
}
