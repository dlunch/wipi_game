use anyhow::{Result, anyhow};

use crate::engine::GameEngine;
use crate::game::{
    AppExploreEvent, AppMovementEvent, DialogActionResult, GameState, LoadingEvent, MenuEvent,
    MenuState, PauseMenuEvent, RuntimeEvent, ShopIntent, has_save_data,
};

pub fn apply_loading(engine: &mut GameEngine, event: crate::game::LoadingEvent) {
    match event {
        LoadingEvent::Advance(step) => engine.transition_to(GameState::Loading(step)),
        LoadingEvent::Loaded => {
            engine.transition_to(GameState::Menu);
            engine
                .ui_mut()
                .menu
                .set_menu(MenuState::new(has_save_data()));
        }
        LoadingEvent::Error(msg) => engine.set_error(msg),
    }
}

pub fn apply_start_new_game(engine: &mut GameEngine) {
    let data = engine.data_rc();
    let (state, session, intro) = crate::game::lifecycle::start_new_game(&data);
    engine.enter_session(state, session, intro);
}

pub fn apply_continue_game(engine: &mut GameEngine) {
    let data = engine.data_rc();
    let (state, session, intro) = crate::game::lifecycle::continue_game(&data);
    engine.enter_session(state, session, intro);
}

pub fn apply_open_pause_menu(engine: &mut GameEngine) {
    engine.ui_mut().pause_menu.reset();
    engine.transition_to(GameState::PauseMenu);
}

pub fn apply_open_menu_from_explore(engine: &mut GameEngine) -> Result<()> {
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

pub fn apply_open_dialog_state(engine: &mut GameEngine, dialog_state: crate::game::DialogState) {
    engine.ui_mut().dialog.open(dialog_state);
    engine.transition_to(GameState::Dialog);
}

pub fn apply_open_shop_by_id(engine: &mut GameEngine, shop_id: &str) {
    let _ = engine.open_shop_by_id(shop_id);
}

pub fn apply_restore_session_stats(engine: &mut GameEngine) -> Result<()> {
    let s = engine
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.restore_stats();
    Ok(())
}

pub fn apply_dialog_action(
    engine: &mut GameEngine,
    action: &crate::data::DialogAction,
) -> Result<()> {
    let open_shop = {
        let data = engine.data_rc();
        let s = engine
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

        if let DialogActionResult::OpenShop(shop_id) = s.apply_dialog_action(&data, action) {
            Some(shop_id)
        } else {
            None
        }
    };

    if let Some(shop_id) = open_shop {
        let _ = engine.open_shop_by_id(&shop_id);
    }

    Ok(())
}

pub fn apply_dialog_transition(engine: &mut GameEngine, transition: crate::game::DialogTransition) {
    match transition {
        crate::game::DialogTransition::SetLine(line) => {
            if let Some(dialog_state) = engine.ui_mut().dialog.state.as_mut() {
                dialog_state.current_line = line;
            }
            engine.transition_to(GameState::Dialog);
        }
        crate::game::DialogTransition::CloseToExplore => {
            engine.ui_mut().dialog.close();
            engine.transition_to(GameState::Explore);
        }
    }
}

pub fn apply_movement(engine: &mut GameEngine, event: AppMovementEvent) -> Result<()> {
    let data = engine.data_rc();
    let s = engine
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;

    let AppMovementEvent::Tick(movement_event, tile_event) = event;
    s.apply_movement_tick(&data, movement_event, tile_event);
    Ok(())
}

pub fn apply_menu(engine: &mut GameEngine, event: MenuEvent) {
    match event {
        MenuEvent::None => {}
        MenuEvent::SetSelected(selected) => engine.ui_mut().menu.set_selected(selected),
        MenuEvent::Action(_) => {}
    }
}

pub fn apply_explore(engine: &mut GameEngine, event: AppExploreEvent) -> Result<()> {
    match event {
        AppExploreEvent::MoveDirection(direction) => {
            let s = engine
                .session_mut()
                .ok_or_else(|| anyhow!("No active session"))?;
            s.on_direction_pressed(direction);
        }
        AppExploreEvent::Npc(_)
        | AppExploreEvent::UseAction(_)
        | AppExploreEvent::EnterPauseMenu
        | AppExploreEvent::EnterMenu => {}
    }

    Ok(())
}

pub fn apply_inventory(engine: &mut GameEngine, event: crate::game::InventoryEvent) -> Result<()> {
    match event {
        crate::game::InventoryEvent::None => {}
        crate::game::InventoryEvent::SetSelected(selected) => {
            engine.ui_mut().inventory.set_selected(selected)
        }
        crate::game::InventoryEvent::UseSelected(index) => {
            let s = engine
                .session_mut()
                .ok_or_else(|| anyhow!("No active session"))?;
            s.use_inventory_item(index);
        }
        crate::game::InventoryEvent::CloseToExplore => engine.transition_to(GameState::Explore),
    }

    Ok(())
}

pub fn apply_dialog(_engine: &mut GameEngine, _event: crate::game::DialogEvent) {}

pub fn apply_shop(engine: &mut GameEngine, event: crate::game::ShopEvent) -> Result<()> {
    match event {
        crate::game::ShopEvent::None => {}
        crate::game::ShopEvent::BuyItem(item) => {
            let s = engine
                .session_mut()
                .ok_or_else(|| anyhow!("No active session"))?;
            s.buy_shop_item(item);
        }
        crate::game::ShopEvent::SellSelected(index) => {
            let (selected, len_after) = {
                let s = engine
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                let sold = s.sell_inventory_item(index).is_some();
                let len_after = s.player.inventory.len();
                (sold, len_after)
            };

            if selected {
                let current_selected = engine.ui().shop.selected;
                if current_selected >= len_after && current_selected > 0 {
                    engine.ui_mut().shop.set_selected(current_selected - 1);
                }
            }
        }
        crate::game::ShopEvent::CloseToExplore => engine.transition_to(GameState::Explore),
    }

    Ok(())
}

pub fn apply_pause_menu(engine: &mut GameEngine, event: PauseMenuEvent) -> Result<()> {
    match event {
        PauseMenuEvent::None => {}
        PauseMenuEvent::SetSelected(selected) => engine.ui_mut().pause_menu.set_selected(selected),
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

pub fn apply_combat_player_action(
    engine: &mut GameEngine,
    action: crate::game::ExploreAction,
) -> Result<()> {
    let data = engine.data_rc();
    let s = engine
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.apply_explore_action(&data, action);
    Ok(())
}

pub fn apply_runtime_event(engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
    match event {
        RuntimeEvent::Loading(event) => apply_loading(engine, event.clone()),
        RuntimeEvent::Movement(event) => apply_movement(engine, event.clone())?,
        RuntimeEvent::Menu(event) => apply_menu(engine, *event),
        RuntimeEvent::Explore(event) => apply_explore(engine, event.clone())?,
        RuntimeEvent::Inventory(event) => apply_inventory(engine, event.clone())?,
        RuntimeEvent::Dialog(event) => apply_dialog(engine, event.clone()),
        RuntimeEvent::Shop(event) => apply_shop(engine, event.clone())?,
        RuntimeEvent::PauseMenu(event) => apply_pause_menu(engine, *event)?,
        RuntimeEvent::CombatPlayerAction(action) => apply_combat_player_action(engine, *action)?,
        RuntimeEvent::Combat(_) => {}
        RuntimeEvent::Transition(event) => crate::game::transition::apply(engine, *event)?,
        RuntimeEvent::Exit(code) => wipi::kernel::exit(*code),
        _ => {}
    }

    Ok(())
}

pub fn resolve_shop_input(
    engine: &GameEngine,
    intent: ShopIntent,
) -> Result<alloc::vec::Vec<RuntimeEvent>> {
    let s = engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;
    let shop_items = engine
        .ui()
        .shop
        .state
        .as_ref()
        .map(|state| state.items.as_slice())
        .unwrap_or(&[]);

    Ok(
        crate::game::shop::resolve_many(intent, s.player.stats.gold, shop_items)
            .into_iter()
            .map(RuntimeEvent::Shop)
            .collect(),
    )
}
