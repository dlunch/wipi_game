use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{
    CombatState, GameData, GameEvent, GameState, MovementState, PlayerState, SessionState,
};

#[derive(Clone)]
pub enum LoadingEvent {
    Advance(usize),
    Loaded,
    Error(String),
}

pub fn resolve_loading(step: usize, load_result: Result<bool, String>) -> LoadingEvent {
    match load_result {
        Ok(true) => LoadingEvent::Loaded,
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

struct UpdateLoadingResolver;

static UPDATE_LOADING_RESOLVER: UpdateLoadingResolver = UpdateLoadingResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&UPDATE_LOADING_RESOLVER]
}

impl DomainEventResolver for UpdateLoadingResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::UpdateLoading)
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        let step = if let GameState::Loading(step) = ctx.state {
            *step
        } else {
            return Err(anyhow!("Invalid state: expected Loading"));
        };

        let load_result = load_step(ctx.data, step);

        Ok(alloc::vec![GameEvent::Loading(resolve_loading(
            step,
            load_result,
        ))])
    }
}

pub fn start_new_game(data: &GameData) -> (GameState, SessionState) {
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

    let state = if config
        .intro_dialog
        .as_ref()
        .and_then(|(dialog_id, _)| data.find_dialog(dialog_id))
        .is_some()
    {
        GameState::Dialog
    } else {
        GameState::Explore
    };

    let session = SessionState {
        player,
        combat,
        movement: MovementState::default(),
        skill_cooldowns: [0; 3],
        mp_regen_timer: 0,
    };

    (state, session)
}

pub fn continue_game(data: &GameData) -> (GameState, SessionState) {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);
    let combat = CombatState::default();

    match crate::game::load_game(&mut player) {
        Ok(true) => {
            if data.find_map(&player.current_map_id).is_none() {
                let (x, y) = (player.x, player.y);
                player.current_map_id = config.fallback_map.clone();
                player.x = x;
                player.y = y;
            }
            if let Some(map) = data.find_map(&player.current_map_id)
                && (map.get_tile(player.x, player.y) == crate::data::Tile::Wall
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

            (GameState::Explore, session)
        }
        Ok(false) | Err(_) => start_new_game(data),
    }
}

pub fn apply_lifecycle_event(
    event: &GameEvent,
    state: &mut GameState,
    session_slot: &mut Option<SessionState>,
    data: &GameData,
) -> bool {
    match event {
        GameEvent::StartNewGame => {
            let (next_state, session) = start_new_game(data);
            crate::game::enter_session(state, session_slot, next_state, session, data);
            true
        }
        GameEvent::ContinueGame => {
            let (next_state, session) = continue_game(data);
            crate::game::enter_session(state, session_slot, next_state, session, data);
            true
        }
        _ => false,
    }
}
