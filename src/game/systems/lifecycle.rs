use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::data::Tile;
use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{
    CharacterState, DialogState, GameData, GameEvent, GameEventKind, TransitionEvent, WorldEvent,
    load_game,
};

#[derive(Clone)]
pub enum LoadingEvent {
    Tick,
    Advance(usize),
    Loaded,
    Error(String),
}

#[derive(Clone, Copy)]
pub enum LifecycleEvent {
    ResetUi,
}

struct LifecycleResolver;

static LIFECYCLE_RESOLVER: LifecycleResolver = LifecycleResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&LIFECYCLE_RESOLVER]
}

impl DomainEventResolver for LifecycleResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::StartNewGame, GameEventKind::ContinueGame]
    }

    fn resolve(
        &self,
        ctx: &ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::StartNewGame => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::ResetUi));
                Self::setup_new_game_events(ctx.data(), out);
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
                if let Some(dialog_state) = Self::intro_dialog_state(ctx.data()) {
                    out.push(GameEvent::OpenDialogState(dialog_state));
                }
            }
            GameEvent::ContinueGame => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::ResetUi));
                Self::setup_continue_events(ctx.data(), out);
                out.push(GameEvent::Transition(TransitionEvent::ToExplore));
            }
            _ => {}
        }

        Ok(())
    }
}

impl LifecycleResolver {
    fn intro_dialog_state(data: &GameData) -> Option<DialogState> {
        let (dialog_id, npc_name) = data.newgame.intro_dialog.as_ref()?;
        let dialog = data.find_dialog(dialog_id)?;
        Some(DialogState::from_dialog(npc_name.clone(), dialog))
    }

    fn setup_new_game_events(data: &GameData, out: &mut Vec<GameEvent>) {
        let config = &data.newgame;
        let mut player = CharacterState::new(config.player_name.clone(), &config.start_map);

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

        out.push(GameEvent::World(WorldEvent::Create));
        out.push(GameEvent::World(WorldEvent::SetPlayerName(
            player.name.clone(),
        )));
        out.push(GameEvent::World(WorldEvent::SetPlayerStats(
            player.stats.clone(),
        )));
        out.push(GameEvent::World(WorldEvent::SetPlayerMap(
            player.current_map_id.clone(),
        )));
        out.push(GameEvent::World(WorldEvent::SetPlayerPosition {
            x: player.x,
            y: player.y,
        }));
        out.push(GameEvent::World(WorldEvent::SetPlayerFacing(player.facing)));

        for item in &player.inventory {
            out.push(GameEvent::World(WorldEvent::AddPlayerItem(item.clone())));
        }

        out.push(GameEvent::World(WorldEvent::SetEquippedWeapon(
            player.equipped_weapon,
        )));
        out.push(GameEvent::World(WorldEvent::SetEquippedArmor(
            player.equipped_armor,
        )));
        out.push(GameEvent::World(WorldEvent::SetEquippedAccessory(
            player.equipped_accessory,
        )));

        out.push(GameEvent::World(WorldEvent::SetSkillCooldowns([0; 3])));
        out.push(GameEvent::World(WorldEvent::SetMpRegenTimer(0)));
        out.push(GameEvent::World(WorldEvent::ResetMovement));
        out.push(GameEvent::World(WorldEvent::ResetCombat));
        out.push(GameEvent::Transition(TransitionEvent::MapChanged));
    }

    fn setup_continue_events(data: &GameData, out: &mut Vec<GameEvent>) {
        let config = &data.newgame;
        let mut player = CharacterState::new(config.player_name.clone(), &config.start_map);
        let mut quests = Vec::with_capacity(16);
        let mut opened_treasures = Vec::with_capacity(16);

        match load_game(&mut player, &mut quests, &mut opened_treasures) {
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

                out.push(GameEvent::World(WorldEvent::Create));
                out.push(GameEvent::World(WorldEvent::SetPlayerName(
                    player.name.clone(),
                )));
                out.push(GameEvent::World(WorldEvent::SetPlayerStats(
                    player.stats.clone(),
                )));
                out.push(GameEvent::World(WorldEvent::SetPlayerMap(
                    player.current_map_id.clone(),
                )));
                out.push(GameEvent::World(WorldEvent::SetPlayerPosition {
                    x: player.x,
                    y: player.y,
                }));
                out.push(GameEvent::World(WorldEvent::SetPlayerFacing(player.facing)));

                for item in &player.inventory {
                    out.push(GameEvent::World(WorldEvent::AddPlayerItem(item.clone())));
                }

                out.push(GameEvent::World(WorldEvent::SetEquippedWeapon(
                    player.equipped_weapon,
                )));
                out.push(GameEvent::World(WorldEvent::SetEquippedArmor(
                    player.equipped_armor,
                )));
                out.push(GameEvent::World(WorldEvent::SetEquippedAccessory(
                    player.equipped_accessory,
                )));

                for quest in &quests {
                    out.push(GameEvent::World(WorldEvent::AddQuestProgress(
                        quest.clone(),
                    )));
                }
                for (map_id, x, y) in &opened_treasures {
                    out.push(GameEvent::World(WorldEvent::AddOpenedTreasure {
                        map_id: map_id.clone(),
                        x: *x,
                        y: *y,
                    }));
                }

                out.push(GameEvent::World(WorldEvent::SetSkillCooldowns([0; 3])));
                out.push(GameEvent::World(WorldEvent::SetMpRegenTimer(0)));
                out.push(GameEvent::World(WorldEvent::ResetMovement));
                out.push(GameEvent::World(WorldEvent::ResetCombat));
                out.push(GameEvent::Transition(TransitionEvent::MapChanged));
            }
            Ok(false) | Err(_) => Self::setup_new_game_events(data, out),
        }
    }
}
