use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::Tile;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{
    DialogState, GameData, GameEvent, GameState, PlayerState, SessionEvent, TransitionEvent,
    load_game,
};

#[derive(Clone)]
pub enum LoadingEvent {
    Advance(usize),
    Loaded,
    Error(String),
}

#[derive(Clone, Copy)]
pub enum LifecycleEvent {
    ResetUi,
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
struct StartContinueResolver;

static UPDATE_LOADING_RESOLVER: UpdateLoadingResolver = UpdateLoadingResolver;
static START_CONTINUE_RESOLVER: StartContinueResolver = StartContinueResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&UPDATE_LOADING_RESOLVER, &START_CONTINUE_RESOLVER]
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

        Ok(vec![GameEvent::Loading(resolve_loading(step, load_result))])
    }
}

impl DomainEventResolver for StartContinueResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::StartNewGame | GameEvent::ContinueGame)
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let mut out = Vec::new();
        out.push(GameEvent::Lifecycle(LifecycleEvent::ResetUi));

        match event {
            GameEvent::StartNewGame => {
                out.extend(setup_new_game_events(ctx.data()));
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
                if let Some(dialog_state) = intro_dialog_state(ctx.data()) {
                    out.push(GameEvent::OpenDialogState(dialog_state));
                }
            }
            GameEvent::ContinueGame => {
                out.extend(setup_continue_events(ctx.data()));
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
            }
            _ => {}
        }

        Ok(out)
    }
}

fn intro_dialog_state(data: &GameData) -> Option<DialogState> {
    let (dialog_id, npc_name) = data.newgame.intro_dialog.as_ref()?;
    let dialog = data.find_dialog(dialog_id)?;
    Some(DialogState::from_dialog(npc_name.clone(), dialog))
}

fn setup_new_game_events(data: &GameData) -> Vec<GameEvent> {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);

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

    let mut out = vec![
        GameEvent::Session(SessionEvent::Create),
        GameEvent::Session(SessionEvent::SetPlayerName(player.name.clone())),
        GameEvent::Session(SessionEvent::SetPlayerStats(player.stats.clone())),
        GameEvent::Session(SessionEvent::SetPlayerMap(player.current_map_id.clone())),
        GameEvent::Session(SessionEvent::SetPlayerPosition {
            x: player.x,
            y: player.y,
        }),
        GameEvent::Session(SessionEvent::SetPlayerFacing(player.facing)),
    ];

    for item in &player.inventory {
        out.push(GameEvent::Session(SessionEvent::AddPlayerItem(
            item.clone(),
        )));
    }

    out.push(GameEvent::Session(SessionEvent::SetEquippedWeapon(
        player.equipped_weapon,
    )));
    out.push(GameEvent::Session(SessionEvent::SetEquippedArmor(
        player.equipped_armor,
    )));
    out.push(GameEvent::Session(SessionEvent::SetEquippedAccessory(
        player.equipped_accessory,
    )));

    for quest in &player.quests {
        out.push(GameEvent::Session(SessionEvent::AddQuestProgress(
            quest.clone(),
        )));
    }
    for (map_id, x, y) in &player.opened_treasures {
        out.push(GameEvent::Session(SessionEvent::AddOpenedTreasure {
            map_id: map_id.clone(),
            x: *x,
            y: *y,
        }));
    }

    out.push(GameEvent::Session(SessionEvent::SetSkillCooldowns([0; 3])));
    out.push(GameEvent::Session(SessionEvent::SetMpRegenTimer(0)));
    out.push(GameEvent::Session(SessionEvent::ResetMovement));
    out.push(GameEvent::Session(SessionEvent::ResetCombat));
    out.push(GameEvent::Session(SessionEvent::SpawnCurrentMapEnemies));
    out
}

fn setup_continue_events(data: &GameData) -> Vec<GameEvent> {
    let config = &data.newgame;
    let mut player = PlayerState::new(config.player_name.clone(), &config.start_map);

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

            let mut out = vec![
                GameEvent::Session(SessionEvent::Create),
                GameEvent::Session(SessionEvent::SetPlayerName(player.name.clone())),
                GameEvent::Session(SessionEvent::SetPlayerStats(player.stats.clone())),
                GameEvent::Session(SessionEvent::SetPlayerMap(player.current_map_id.clone())),
                GameEvent::Session(SessionEvent::SetPlayerPosition {
                    x: player.x,
                    y: player.y,
                }),
                GameEvent::Session(SessionEvent::SetPlayerFacing(player.facing)),
            ];

            for item in &player.inventory {
                out.push(GameEvent::Session(SessionEvent::AddPlayerItem(
                    item.clone(),
                )));
            }

            out.push(GameEvent::Session(SessionEvent::SetEquippedWeapon(
                player.equipped_weapon,
            )));
            out.push(GameEvent::Session(SessionEvent::SetEquippedArmor(
                player.equipped_armor,
            )));
            out.push(GameEvent::Session(SessionEvent::SetEquippedAccessory(
                player.equipped_accessory,
            )));

            for quest in &player.quests {
                out.push(GameEvent::Session(SessionEvent::AddQuestProgress(
                    quest.clone(),
                )));
            }
            for (map_id, x, y) in &player.opened_treasures {
                out.push(GameEvent::Session(SessionEvent::AddOpenedTreasure {
                    map_id: map_id.clone(),
                    x: *x,
                    y: *y,
                }));
            }

            out.push(GameEvent::Session(SessionEvent::SetSkillCooldowns([0; 3])));
            out.push(GameEvent::Session(SessionEvent::SetMpRegenTimer(0)));
            out.push(GameEvent::Session(SessionEvent::ResetMovement));
            out.push(GameEvent::Session(SessionEvent::ResetCombat));
            out.push(GameEvent::Session(SessionEvent::SpawnCurrentMapEnemies));
            out
        }
        Ok(false) | Err(_) => setup_new_game_events(data),
    }
}
