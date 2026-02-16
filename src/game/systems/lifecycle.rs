use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::Tile;
use crate::engine::GameEngine;
use crate::game::systems::runtime::{DomainEventApplier, DomainEventResolver};
use crate::game::{
    CombatState, GameData, GameState, MenuState, MovementState, PlayerState, RuntimeEvent,
    SessionState, has_save_data, load_game,
};

pub struct IntroDialogSpec {
    pub dialog_id: String,
    pub npc_name: String,
}

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

pub fn start_new_game(data: &GameData) -> (GameState, SessionState, Option<IntroDialogSpec>) {
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
            Some(IntroDialogSpec {
                dialog_id: dialog.id.clone(),
                npc_name: npc_name.clone(),
            }),
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

pub fn continue_game(data: &GameData) -> (GameState, SessionState, Option<IntroDialogSpec>) {
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

struct UpdateLoadingResolver;
struct LoadingApplier;
struct StartNewGameApplier;
struct ContinueGameApplier;

static UPDATE_LOADING_RESOLVER: UpdateLoadingResolver = UpdateLoadingResolver;
static LOADING_APPLIER: LoadingApplier = LoadingApplier;
static START_NEW_GAME_APPLIER: StartNewGameApplier = StartNewGameApplier;
static CONTINUE_GAME_APPLIER: ContinueGameApplier = ContinueGameApplier;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&UPDATE_LOADING_RESOLVER]
}

pub fn appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![
        &LOADING_APPLIER,
        &START_NEW_GAME_APPLIER,
        &CONTINUE_GAME_APPLIER,
    ]
}

impl DomainEventResolver for UpdateLoadingResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::UpdateLoading)
    }

    fn resolve(&self, engine: &mut GameEngine, _event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        let step = if let GameState::Loading(step) = engine.state() {
            *step
        } else {
            return Err(anyhow!("Invalid state: expected Loading"));
        };

        let mut data = engine.data_rc();
        let load_result = load_step(&mut data, step);
        engine.replace_data(data);

        Ok(alloc::vec![RuntimeEvent::Loading(resolve_loading(
            step,
            load_result,
        ))])
    }
}

impl DomainEventApplier for LoadingApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Loading(_))
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Loading(event) = event else {
            return Ok(());
        };
        match event {
            LoadingEvent::Advance(step) => engine.transition_to(GameState::Loading(*step)),
            LoadingEvent::Loaded => {
                engine.transition_to(GameState::Menu);
                engine
                    .ui_mut()
                    .menu
                    .set_menu(MenuState::new(has_save_data()));
            }
            LoadingEvent::Error(msg) => engine.set_error(msg.clone()),
        }
        Ok(())
    }
}

impl DomainEventApplier for StartNewGameApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::StartNewGame)
    }

    fn apply(&self, engine: &mut GameEngine, _event: &RuntimeEvent) -> Result<()> {
        let data = engine.data_rc();
        let (state, session, intro) = start_new_game(&data);
        engine.enter_session(state, session, intro);
        Ok(())
    }
}

impl DomainEventApplier for ContinueGameApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::ContinueGame)
    }

    fn apply(&self, engine: &mut GameEngine, _event: &RuntimeEvent) -> Result<()> {
        let data = engine.data_rc();
        let (state, session, intro) = continue_game(&data);
        engine.enter_session(state, session, intro);
        Ok(())
    }
}
