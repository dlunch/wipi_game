use anyhow::{Result, anyhow};

use crate::data::Tile;
use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{CombatState, GameData, GameEvent, MovementState, PlayerState};

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
}

struct SessionLifecycleApplier;
struct SessionSaveApplier;

static SESSION_LIFECYCLE_APPLIER: SessionLifecycleApplier = SessionLifecycleApplier;
static SESSION_SAVE_APPLIER: SessionSaveApplier = SessionSaveApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&SESSION_LIFECYCLE_APPLIER, &SESSION_SAVE_APPLIER]
}

impl DomainEventApplier for SessionLifecycleApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::StartNewGame | GameEvent::ContinueGame | GameEvent::RestoreSessionStats
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::StartNewGame => {
                let (state, session) = start_new_game(ctx.data);
                enter_session(ctx, state, session);
            }
            GameEvent::ContinueGame => {
                let (state, session) = continue_game(ctx.data);
                enter_session(ctx, state, session);
            }
            GameEvent::RestoreSessionStats => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.player.restore_stats();
            }
            _ => {}
        }
        Ok(())
    }
}

impl DomainEventApplier for SessionSaveApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::PauseMenu(crate::game::PauseMenuEvent::SaveAndReturnExplore)
                | GameEvent::OpenMenuFromExplore
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, _event: &GameEvent) -> Result<()> {
        let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
        let _ = crate::game::save_game(&s.player);
        Ok(())
    }
}

fn start_new_game(data: &GameData) -> (crate::game::GameState, SessionState) {
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

fn continue_game(data: &GameData) -> (crate::game::GameState, SessionState) {
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

fn enter_session(ctx: &mut ApplyContext<'_>, state: crate::game::GameState, session: SessionState) {
    *ctx.session = Some(session);
    ctx.transition_to(state);

    if let Some(s) = ctx.session.as_mut() {
        s.spawn_current_map_enemies(ctx.data);
    }
}
