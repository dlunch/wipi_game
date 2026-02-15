use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

use crate::data::Tile;
use crate::game::{
    CombatState, DialogState, GameData, GameState, MenuState, MovementState, PlayerState,
    SessionState, combat, load_game,
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
    let mut player = PlayerState::new(String::from("Hero"), "village");
    let mut combat = CombatState::default();

    if let Some(sword) = data.find_item("wooden_sword").cloned() {
        let idx = player.inventory.len();
        player.inventory.push(sword);
        player.equipped_weapon = Some(idx);
    }
    if let Some(armor) = data.find_item("cloth").cloned() {
        let idx = player.inventory.len();
        player.inventory.push(armor);
        player.equipped_armor = Some(idx);
    }
    if let Some(potion) = data.find_item("potion").cloned() {
        player.inventory.push(potion.clone());
        player.inventory.push(potion);
    }

    if let Some(map) = data.find_map("village") {
        let (x, y) = map.find_player_start().unwrap_or((player.x, player.y));
        player.current_map_id = map.id.clone();
        player.x = x;
        player.y = y;
        combat::spawn_for_map(&mut combat, map, &data.enemies);
    }

    let (state, dialog_state) = if let Some(dialog) = data.find_dialog("dialog_guide") {
        (
            GameState::Dialog,
            Some(DialogState::new(String::from("마을 안내원"), dialog)),
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
    let mut player = PlayerState::new(String::from("Hero"), "village");
    let mut combat = CombatState::default();

    match load_game(&mut player) {
        Ok(true) => {
            if data.find_map(&player.current_map_id).is_none() {
                let (x, y) = (player.x, player.y);
                player.current_map_id = String::from("village");
                player.x = x;
                player.y = y;
            }
            if let Some(map) = data.find_map(&player.current_map_id) {
                if (map.get_tile(player.x, player.y) == Tile::Wall
                    || player.x >= map.width
                    || player.y >= map.height)
                    && let Some((x, y)) = map.find_player_start()
                {
                    player.x = x;
                    player.y = y;
                }
                combat::spawn_for_map(&mut combat, map, &data.enemies);
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
