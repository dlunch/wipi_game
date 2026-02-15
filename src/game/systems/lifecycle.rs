use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

use crate::data::Tile;
use crate::game::{
    CombatState, DialogState, GameData, GameState, MenuState, MovementState, PlayerState,
    SessionState, load_game,
};

pub enum LoadingEvent {
    None,
    Advance(usize),
    Loaded(MenuState),
    Error(String),
}

pub fn reduce_loading(
    state: &GameState,
    load_result: Result<bool, String>,
    has_save: bool,
) -> LoadingEvent {
    let GameState::Loading(step) = *state else {
        return LoadingEvent::None;
    };

    match load_result {
        Ok(true) => LoadingEvent::Loaded(MenuState::new(has_save)),
        Ok(false) => LoadingEvent::Advance(step + 1),
        Err(e) => LoadingEvent::Error(e),
    }
}

pub fn load_step(data: &mut Rc<GameData>, step: usize) -> Result<bool, String> {
    let Some(data_mut) = Rc::get_mut(data) else {
        return Err(String::from("Load error: data is shared"));
    };

    data_mut
        .load_step(step)
        .map_err(|e| format!("Load error: {}", e))
}

pub fn start_new_game(data: &GameData) -> (GameState, SessionState, Option<DialogState>) {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);
    let combat = CombatState::default();

    if let Some(ref weapon_id) = config.equip_weapon
        && let Some(weapon) = data.find_item(weapon_id).cloned()
    {
        let idx = player.inventory.len();
        player.inventory.push(weapon);
        player.equipped_weapon = Some(idx);
    }
    if let Some(ref armor_id) = config.equip_armor
        && let Some(armor) = data.find_item(armor_id).cloned()
    {
        let idx = player.inventory.len();
        player.inventory.push(armor);
        player.equipped_armor = Some(idx);
    }
    for start_item in &config.items {
        if let Some(item) = data.find_item(&start_item.item_id).cloned() {
            for _ in 0..start_item.count {
                player.inventory.push(item.clone());
            }
        }
    }

    if let Some(map) = data.find_map(&config.start_map) {
        let (x, y) = map.find_player_start().unwrap_or((player.x, player.y));
        player.current_map_id = map.id.clone();
        player.x = x;
        player.y = y;
    }

    let (state, dialog_state) = if let Some((ref dialog_id, ref npc_name)) = config.intro_dialog
        && let Some(dialog) = data.find_dialog(dialog_id)
    {
        (
            GameState::Dialog,
            Some(DialogState::new(npc_name.clone(), dialog)),
        )
    } else {
        (GameState::Explore, None)
    };

    let session = SessionState {
        player,
        combat,
        movement: MovementState::default(),
        skill_cooldowns: [0; 3],
        mp_regen_timer: 0,
    };

    (state, session, dialog_state)
}

pub fn continue_game(data: &GameData) -> (GameState, SessionState, Option<DialogState>) {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);
    let combat = CombatState::default();

    match load_game(&mut player) {
        Ok(true) => {
            if data.find_map(&player.current_map_id).is_none() {
                let (x, y) = (player.x, player.y);
                player.current_map_id = config.fallback_map.clone();
                player.x = x;
                player.y = y;
            }
            if let Some(map) = data.find_map(&player.current_map_id)
                && (map.get_tile(player.x, player.y) == Tile::Wall
                    || player.x >= map.width
                    || player.y >= map.height)
                && let Some((x, y)) = map.find_player_start()
            {
                player.x = x;
                player.y = y;
            }

            let session = SessionState {
                player,
                combat,
                movement: MovementState::default(),
                skill_cooldowns: [0; 3],
                mp_regen_timer: 0,
            };

            (GameState::Explore, session, None)
        }
        Ok(false) | Err(_) => start_new_game(data),
    }
}
