use anyhow::Result;

use crate::data::Tile;
use crate::game::{
    CombatState, GameData, GameEvent, GameState, MovementState, PlayerState, UiState,
};

pub struct SessionState {
    pub player: PlayerState,
    pub combat: CombatState,
    pub movement: MovementState,
    pub skill_cooldowns: [u32; 3],
    pub mp_regen_timer: u32,
}

impl SessionState {
    pub fn spawn_current_map_enemies(&mut self, data: &GameData) {
        if let Some(map) = data.find_map(&self.player.current_map_id) {
            self.combat.spawn_for_map(map, &data.enemies);
        }
    }

    pub fn apply_event(
        &mut self,
        data: &GameData,
        state: &mut GameState,
        ui: &mut UiState,
        event: &GameEvent,
    ) -> Result<()> {
        match event {
            GameEvent::RestoreSessionStats => self.player.restore_stats(),
            GameEvent::PauseMenu(crate::game::PauseMenuEvent::SaveAndReturnExplore)
            | GameEvent::OpenMenuFromExplore => {
                let _ = crate::game::save_game(&self.player);
            }
            GameEvent::OpenShopById(shop_id) => {
                let _ = open_shop_by_id(data, state, ui, shop_id);
            }
            GameEvent::Transition(crate::game::TransitionEvent::MapChanged) => {
                self.spawn_current_map_enemies(data);
            }
            _ => {}
        }

        self.player.apply_event(data, event)?;
        self.movement
            .apply_event(data, state, &mut self.player, event)?;
        self.combat.apply_event(
            data,
            state,
            &mut self.player,
            &mut self.skill_cooldowns,
            &mut self.mp_regen_timer,
            event,
        )?;
        Ok(())
    }
}

pub fn start_new_game(data: &GameData) -> (crate::game::GameState, SessionState) {
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
        crate::game::GameState::Dialog
    } else {
        crate::game::GameState::Explore
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

pub fn continue_game(data: &GameData) -> (crate::game::GameState, SessionState) {
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

            (crate::game::GameState::Explore, session)
        }
        Ok(false) | Err(_) => start_new_game(data),
    }
}

pub fn enter_session(
    state: &mut crate::game::GameState,
    session_slot: &mut Option<SessionState>,
    next_state: crate::game::GameState,
    session: SessionState,
    data: &GameData,
) {
    *session_slot = Some(session);
    crate::game::state::transition_to(state, session_slot, next_state);
    if let Some(s) = session_slot.as_mut() {
        s.spawn_current_map_enemies(data);
    }
}

fn open_shop_by_id(
    data: &GameData,
    state: &mut GameState,
    ui: &mut UiState,
    shop_id: &str,
) -> bool {
    let Some(shop) = data.find_shop(shop_id).cloned() else {
        return false;
    };
    let shop_items = data.get_shop_items(&shop);
    ui.shop.open(crate::game::ShopState::new(shop, shop_items));
    if state.can_transition_to(&GameState::Shop) {
        *state = GameState::Shop;
    } else {
        crate::game::state::set_error(
            state,
            alloc::format!(
                "Invalid state transition: {:?} -> {:?}",
                state,
                GameState::Shop
            ),
        );
    }
    true
}
