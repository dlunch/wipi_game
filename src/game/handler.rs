use crate::game::{
    self, CombatState, DialogIntent, GameData, GameState, InventoryIntent, InventoryState,
    MenuAction, MenuEvent, MenuIntent, PauseMenuIntent, PlayerState, ShopIntent, update,
};

pub(crate) fn handle_menu_input(
    state: &mut GameState,
    player: &mut PlayerState,
    combat: &mut CombatState,
    data: &GameData,
    intent: MenuIntent,
) {
    if let MenuEvent::Action(action) = game::menu::reduce(state, intent) {
        match action {
            MenuAction::NewGame => update::start_new_game(state, player, combat, data),
            MenuAction::Continue => update::continue_game(state, player, combat, data),
            MenuAction::Exit => wipi::kernel::exit(0),
        }
    }
}

pub(crate) fn handle_inventory_input(
    state: &mut GameState,
    player: &mut PlayerState,
    inventory_state: &mut InventoryState,
    intent: InventoryIntent,
) {
    game::inventory::reduce(state, player, inventory_state, intent);
}

pub(crate) fn handle_dialog_input(
    state: &mut GameState,
    player: &mut PlayerState,
    data: &GameData,
    intent: DialogIntent,
) {
    game::dialog::reduce(state, player, data, intent);
}

pub(crate) fn handle_shop_input(
    state: &mut GameState,
    player: &mut PlayerState,
    intent: ShopIntent,
) {
    game::shop::reduce(state, player, intent);
}

pub(crate) fn handle_pause_menu_input(
    state: &mut GameState,
    player: &PlayerState,
    inventory_state: &mut InventoryState,
    intent: PauseMenuIntent,
) {
    game::menu::reduce_pause(state, player, inventory_state, intent);
}
