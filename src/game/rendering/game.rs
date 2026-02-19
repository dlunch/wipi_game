use alloc::{format, rc::Rc, string::String, vec::Vec};

use anyhow::{Result, anyhow};
use wipi::framebuffer::Framebuffer;

use super::{
    dialog::draw_dialog,
    explore::draw_explore,
    inventory::{draw_inventory, draw_stats},
    menu::{draw_menu, draw_pause_menu},
    quest::draw_quest_log,
    render_fx::RenderFxState,
    render_state::{
        ExploreRender, InventoryRender, QuestLogRender, RenderState, ShopRender, SkillEffectRender,
        StatsRender, TrackedQuestRender, interaction_hint_from_world, scroll_for_selection,
    },
    renderer::{
        COLOR_BLUE, COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, clear_screen, draw_rect, draw_text,
        fill_rect,
    },
    shop::draw_shop,
    sprites::SpriteAtlas,
};
use crate::game::{
    game_data::GameData,
    game_event::{
        CombatEvent, EntityEvent, ExploreEvent, GameEvent, MovementEvent, TransitionEvent,
        WorldEvent,
    },
    state::{CombatantState, EntityState, GameState, TimedKind},
    systems::lifecycle::{LifecycleEvent, LoadingEvent},
    ui::state::{INVENTORY_VISIBLE_ITEMS, MenuAction, SHOP_VISIBLE_ITEMS, ShopMode, UiState},
    world::WorldState,
};

struct DialogRenderFields {
    explore: Option<ExploreRender>,
    npc_name: String,
    lines: Vec<String>,
    current_line: usize,
    current_text: Option<String>,
    has_next: bool,
}

fn build_dialog_render_fields(
    world: Option<&WorldState>,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Result<DialogRenderFields> {
    let dialog_state = ui
        .dialog
        .state
        .as_ref()
        .ok_or_else(|| anyhow!("No dialog state"))?;
    let lines = dialog_state
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>();
    let current_line = dialog_state.current_line.min(lines.len().saturating_sub(1));
    let current_text = lines.get(current_line).cloned();
    let explore = world
        .map(|world| ExploreRender::from_world(world, ui, data, render_fx))
        .transpose()?;

    Ok(DialogRenderFields {
        explore,
        npc_name: dialog_state.npc_name.clone(),
        lines,
        current_line,
        current_text,
        has_next: current_line + 1 < dialog_state.lines.len(),
    })
}

impl RenderState {
    pub fn apply_game_event_patch(
        &mut self,
        event: &GameEvent,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Result<bool> {
        if self.apply_state_transition_patch(event, state, world, ui, data, render_fx)? {
            return Ok(true);
        }

        if !self.matches_state_variant(state) {
            return Err(anyhow!(
                "Render state mismatch without transition patch: render={}, game={}",
                self.variant_name(),
                state.kind_name()
            ));
        }

        match self {
            RenderState::Loading { step } => {
                if let GameEvent::Loading(LoadingEvent::Advance(next_step)) = event
                    && *step != *next_step
                {
                    *step = *next_step;
                    return Ok(true);
                }
                Ok(false)
            }
            RenderState::Menu {
                title,
                items,
                selected,
            } => {
                if !matches!(
                    event,
                    GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(_))
                ) {
                    return Ok(false);
                }
                Ok(sync_menu_fields(title, items, selected, ui))
            }
            RenderState::Explore(explore) => {
                patch_explore(explore, event, world, ui, data, render_fx, state)
            }
            RenderState::Inventory(inventory) => {
                if matches!(
                    event,
                    GameEvent::UseInventorySelected(_)
                        | GameEvent::Entity(_)
                        | GameEvent::Transition(_)
                        | GameEvent::OpenShopById(_)
                ) {
                    let next = InventoryRender::from_world(
                        world.ok_or_else(|| anyhow!("No active world"))?,
                        ui,
                        data,
                    )?;
                    *inventory = next;
                    return Ok(true);
                }
                Ok(false)
            }
            RenderState::Stats(stats) => {
                if matches!(event, GameEvent::Combat(_) | GameEvent::Entity(_)) {
                    let next = StatsRender::from_world(
                        world.ok_or_else(|| anyhow!("No active world"))?,
                        data,
                    )?;
                    *stats = next;
                    return Ok(true);
                }
                Ok(false)
            }
            RenderState::Dialog {
                explore,
                npc_name,
                lines,
                current_line,
                current_text,
                has_next,
            } => {
                let mut changed = false;
                if let Some(explore) = explore.as_mut() {
                    changed |= patch_explore(explore, event, world, ui, data, render_fx, state)?;
                }
                if let GameEvent::OpenDialog { .. } = event {
                    let next = build_dialog_render_fields(world, ui, data, render_fx)?;
                    *explore = next.explore;
                    *npc_name = next.npc_name;
                    *lines = next.lines;
                    *current_line = next.current_line;
                    *current_text = next.current_text;
                    *has_next = next.has_next;
                    changed = true;
                }
                Ok(changed)
            }
            RenderState::Shop(shop) => {
                if matches!(
                    event,
                    GameEvent::ShopBuyItem(_)
                        | GameEvent::ShopSellItem(_)
                        | GameEvent::Entity(_)
                        | GameEvent::OpenShopById(_)
                        | GameEvent::Transition(_)
                ) {
                    let next = ShopRender::from_world(
                        world.ok_or_else(|| anyhow!("No active world"))?,
                        ui,
                        data,
                        render_fx,
                    )?;
                    *shop = next;
                    return Ok(true);
                }
                Ok(false)
            }
            RenderState::QuestLog(quest_log) => {
                if matches!(event, GameEvent::World(_) | GameEvent::Transition(_)) {
                    let next = QuestLogRender::from_world(
                        world.ok_or_else(|| anyhow!("No active world"))?,
                        ui,
                        data,
                    )?;
                    *quest_log = next;
                    return Ok(true);
                }
                Ok(false)
            }
            RenderState::PauseMenu { explore, .. } => {
                if let Some(explore) = explore.as_mut() {
                    return patch_explore(explore, event, world, ui, data, render_fx, state);
                }
                Ok(false)
            }
            RenderState::Dead | RenderState::Error(_) => Ok(false),
        }
    }

    fn apply_state_transition_patch(
        &mut self,
        event: &GameEvent,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Result<bool> {
        if GameState::transition_target_from_event(event).is_none()
            || self.matches_state_variant(state)
        {
            return Ok(false);
        }

        *self = Self::build_for_state(state, world, ui, data, render_fx)?;
        Ok(true)
    }

    pub fn apply_ui_patch(&mut self, ui: &UiState, world: Option<&WorldState>) -> Result<bool> {
        match self {
            RenderState::Menu {
                title,
                items,
                selected,
            } => Ok(sync_menu_fields(title, items, selected, ui)),
            RenderState::PauseMenu {
                items, selected, ..
            } => {
                let mut changed = false;
                if *items != ui.pause_menu.state.items {
                    *items = ui.pause_menu.state.items.clone();
                    changed = true;
                }
                if *selected != ui.pause_menu.selected {
                    *selected = ui.pause_menu.selected;
                    changed = true;
                }
                Ok(changed)
            }
            RenderState::Inventory(inventory) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                let inventory_len = world.leader_entity()?.inventory.len();
                let mut changed = false;
                if inventory.selected != ui.inventory.selected {
                    inventory.selected = ui.inventory.selected;
                    changed = true;
                }
                let next_scroll = scroll_for_selection(
                    inventory.selected,
                    inventory_len,
                    INVENTORY_VISIBLE_ITEMS,
                );
                if inventory.scroll != next_scroll {
                    inventory.scroll = next_scroll;
                    changed = true;
                }
                Ok(changed)
            }
            RenderState::Shop(shop) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                let inventory_len = world.leader_entity()?.inventory.len();
                let shop_items_len = shop.buy_items.len();
                let total = match ui.shop.mode {
                    ShopMode::Select => 2,
                    ShopMode::Buy | ShopMode::ConfirmBuy => shop_items_len,
                    ShopMode::Sell | ShopMode::ConfirmSell => inventory_len,
                };
                let next_scroll = scroll_for_selection(ui.shop.selected, total, SHOP_VISIBLE_ITEMS);
                let mut changed = false;
                if shop.mode != ui.shop.mode {
                    shop.mode = ui.shop.mode;
                    changed = true;
                }
                if shop.selected != ui.shop.selected {
                    shop.selected = ui.shop.selected;
                    changed = true;
                }
                if shop.scroll != next_scroll {
                    shop.scroll = next_scroll;
                    changed = true;
                }
                Ok(changed)
            }
            RenderState::QuestLog(quest_log) => {
                let mut changed = false;
                if quest_log.tracked_quest_id != ui.quest_log.tracked_quest_id {
                    quest_log.tracked_quest_id = ui.quest_log.tracked_quest_id;
                    changed = true;
                }
                let next_selected = if quest_log.quests.is_empty() {
                    0
                } else {
                    ui.quest_log.selected.min(quest_log.quests.len() - 1)
                };
                if quest_log.selected != next_selected {
                    quest_log.selected = next_selected;
                    changed = true;
                }
                Ok(changed)
            }
            RenderState::Dialog {
                current_line,
                current_text,
                has_next,
                lines,
                ..
            } => {
                if let Some(dialog_state) = ui.dialog.state.as_ref() {
                    let next_line = dialog_state.current_line.min(lines.len().saturating_sub(1));
                    let mut changed = false;
                    if *current_line != next_line {
                        *current_line = next_line;
                        changed = true;
                    }
                    let next_text = lines.get(*current_line).cloned();
                    if *current_text != next_text {
                        *current_text = next_text;
                        changed = true;
                    }
                    let next_has_next = *current_line + 1 < lines.len();
                    if *has_next != next_has_next {
                        *has_next = next_has_next;
                        changed = true;
                    }
                    Ok(changed)
                } else {
                    Err(anyhow!("No dialog state"))
                }
            }
            _ => Ok(false),
        }
    }

    pub fn apply_tick(&mut self, render_fx: &RenderFxState) -> bool {
        match self {
            RenderState::Explore(explore)
            | RenderState::Dialog {
                explore: Some(explore),
                ..
            }
            | RenderState::PauseMenu {
                explore: Some(explore),
                ..
            } => {
                let mut changed = false;
                if explore.player_hit_flash != render_fx.player_hit_flash() {
                    explore.player_hit_flash = render_fx.player_hit_flash();
                    changed = true;
                }
                for enemy in &mut explore.enemies {
                    let next_hit_flash = render_fx.enemy_hit_flash(enemy.enemy_id);
                    if enemy.hit_flash != next_hit_flash {
                        enemy.hit_flash = next_hit_flash;
                        changed = true;
                    }
                }
                let next_skill_effects = render_fx
                    .skill_effect_iter()
                    .map(|(x, y, effect_type)| SkillEffectRender { x, y, effect_type })
                    .collect::<Vec<_>>();
                if explore.skill_effects != next_skill_effects {
                    explore.skill_effects = next_skill_effects;
                    changed = true;
                }
                if explore.quest_notice_timer != render_fx.quest_notice_timer() {
                    explore.quest_notice_timer = render_fx.quest_notice_timer();
                    changed = true;
                }
                if explore.anim_tick != render_fx.anim_tick() {
                    explore.anim_tick = render_fx.anim_tick();
                    changed = true;
                }
                changed
            }
            RenderState::Shop(shop) => {
                let next_notice = render_fx.shop_purchase_notice_timer();
                if shop.purchase_notice_timer != next_notice {
                    shop.purchase_notice_timer = next_notice;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn enter_dialog(
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Result<Self> {
        let fields = build_dialog_render_fields(world, ui, data, render_fx)?;
        Ok(RenderState::Dialog {
            explore: fields.explore,
            npc_name: fields.npc_name,
            lines: fields.lines,
            current_line: fields.current_line,
            current_text: fields.current_text,
            has_next: fields.has_next,
        })
    }

    fn build_for_state(
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Result<Self> {
        match state {
            GameState::Loading(step) => Ok(RenderState::Loading { step: *step }),
            GameState::Menu => Ok(RenderState::Menu {
                title: ui.menu.state.title,
                items: ui.menu.state.items.clone(),
                selected: ui.menu.selected,
            }),
            GameState::Explore => Ok(RenderState::Explore(ExploreRender::from_world(
                world.ok_or_else(|| anyhow!("No active world"))?,
                ui,
                data,
                render_fx,
            )?)),
            GameState::Inventory => Ok(RenderState::Inventory(InventoryRender::from_world(
                world.ok_or_else(|| anyhow!("No active world"))?,
                ui,
                data,
            )?)),
            GameState::Stats => Ok(RenderState::Stats(StatsRender::from_world(
                world.ok_or_else(|| anyhow!("No active world"))?,
                data,
            )?)),
            GameState::Dialog => Self::enter_dialog(world, ui, data, render_fx),
            GameState::Shop => Ok(RenderState::Shop(ShopRender::from_world(
                world.ok_or_else(|| anyhow!("No active world"))?,
                ui,
                data,
                render_fx,
            )?)),
            GameState::QuestLog => Ok(RenderState::QuestLog(QuestLogRender::from_world(
                world.ok_or_else(|| anyhow!("No active world"))?,
                ui,
                data,
            )?)),
            GameState::PauseMenu => {
                let explore = ExploreRender::from_world(
                    world.ok_or_else(|| anyhow!("No active world"))?,
                    ui,
                    data,
                    render_fx,
                )?;
                Ok(RenderState::PauseMenu {
                    explore: Some(explore),
                    items: ui.pause_menu.state.items.clone(),
                    selected: ui.pause_menu.selected,
                })
            }
            GameState::Dead => Ok(RenderState::Dead),
            GameState::Error(msg) => Ok(RenderState::Error(msg.into())),
        }
    }

    fn matches_state_variant(&self, state: &GameState) -> bool {
        matches!(
            (self, state),
            (RenderState::Loading { .. }, GameState::Loading(_))
                | (RenderState::Menu { .. }, GameState::Menu)
                | (RenderState::Explore(_), GameState::Explore)
                | (RenderState::Inventory(_), GameState::Inventory)
                | (RenderState::Stats(_), GameState::Stats)
                | (RenderState::Dialog { .. }, GameState::Dialog)
                | (RenderState::Shop(_), GameState::Shop)
                | (RenderState::QuestLog(_), GameState::QuestLog)
                | (RenderState::PauseMenu { .. }, GameState::PauseMenu)
                | (RenderState::Dead, GameState::Dead)
                | (RenderState::Error(_), GameState::Error(_))
        )
    }

    fn variant_name(&self) -> &'static str {
        match self {
            RenderState::Loading { .. } => "Loading",
            RenderState::Menu { .. } => "Menu",
            RenderState::Explore(_) => "Explore",
            RenderState::Inventory(_) => "Inventory",
            RenderState::Stats(_) => "Stats",
            RenderState::Dialog { .. } => "Dialog",
            RenderState::Shop(_) => "Shop",
            RenderState::QuestLog(_) => "QuestLog",
            RenderState::PauseMenu { .. } => "PauseMenu",
            RenderState::Dead => "Dead",
            RenderState::Error(_) => "Error",
        }
    }
}

fn patch_explore(
    explore: &mut ExploreRender,
    event: &GameEvent,
    world: Option<&WorldState>,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
    state: &GameState,
) -> Result<bool> {
    let world = world.ok_or_else(|| anyhow!("No active world"))?;

    match event {
        GameEvent::World(WorldEvent::SetWorldMap(_))
        | GameEvent::Transition(TransitionEvent::MapChanged) => {
            let next = ExploreRender::from_world(world, ui, data, render_fx)?;
            *explore = next;
            Ok(true)
        }
        GameEvent::Movement(MovementEvent::Tick(..))
        | GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(_))
        | GameEvent::Explore(ExploreEvent::MoveDirection(_)) => {
            sync_explore_player(explore, world, ui, data)
        }
        GameEvent::Entity(entity_event) => {
            patch_explore_entity(explore, entity_event, world, ui, data, render_fx)
        }
        GameEvent::Combat(combat_event) => {
            patch_explore_combat(explore, combat_event, world, data, render_fx)
        }
        GameEvent::World(
            WorldEvent::CreateQuestProgress { .. }
            | WorldEvent::ChangeQuestCurrentCount { .. }
            | WorldEvent::SetQuestCompleted { .. }
            | WorldEvent::SetQuestRewarded { .. },
        ) => sync_explore_quest(explore, world, ui, data),
        GameEvent::World(WorldEvent::AddOpenedTreasure { map_id, x, y }) => {
            if *map_id != explore.map_id {
                return Ok(false);
            }
            if !explore
                .opened_treasures
                .iter()
                .any(|(m, tx, ty)| m == map_id && *tx == *x && *ty == *y)
            {
                explore.opened_treasures.push((*map_id, *x, *y));
                explore.interaction_hint = interaction_hint_from_world(world, data)?;
                return Ok(true);
            }
            Ok(false)
        }
        GameEvent::ApplyDialogAction(_) => {
            if matches!(
                state,
                GameState::Explore | GameState::Dialog | GameState::PauseMenu
            ) {
                return sync_explore_quest(explore, world, ui, data);
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn sync_menu_fields(
    title: &mut &'static str,
    items: &mut Vec<(&'static str, MenuAction)>,
    selected: &mut usize,
    ui: &UiState,
) -> bool {
    let mut changed = false;
    if *title != ui.menu.state.title {
        *title = ui.menu.state.title;
        changed = true;
    }
    if *items != ui.menu.state.items {
        *items = ui.menu.state.items.clone();
        changed = true;
    }
    if *selected != ui.menu.selected {
        *selected = ui.menu.selected;
        changed = true;
    }
    changed
}

fn patch_explore_entity(
    explore: &mut ExploreRender,
    event: &EntityEvent,
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Result<bool> {
    match event {
        EntityEvent::SetEntityLevel { entity_id, .. }
        | EntityEvent::SetEntityExp { entity_id, .. }
        | EntityEvent::SetEntityExpToNext { entity_id, .. }
        | EntityEvent::SetEntityBaseMaxHp { entity_id, .. }
        | EntityEvent::SetEntityBaseMaxMp { entity_id, .. }
        | EntityEvent::SetEntityBaseAtk { entity_id, .. }
        | EntityEvent::SetEntityBaseDef { entity_id, .. }
        | EntityEvent::SetEntityCurrentHp { entity_id, .. }
        | EntityEvent::ChangeEntityHp { entity_id, .. }
        | EntityEvent::SetEntityCurrentMp { entity_id, .. }
        | EntityEvent::ChangeEntityMp { entity_id, .. }
        | EntityEvent::SetEntityLoadoutSlot { entity_id, .. }
        | EntityEvent::ChangeEntityItem { entity_id, .. }
        | EntityEvent::ClearEntityInventory { entity_id }
        | EntityEvent::SetEntityTransform { entity_id, .. } => {
            patch_explore_entity_target(explore, *entity_id, world, ui, data, render_fx, true)
        }
        EntityEvent::CreateEntity { entity_id, .. } => {
            patch_explore_entity_target(explore, *entity_id, world, ui, data, render_fx, false)
        }
        EntityEvent::SetLeaderEntity(_)
        | EntityEvent::ClearCompanionEntities
        | EntityEvent::AddCompanionEntity(_) => Ok(false),
        EntityEvent::AddEntityExp { entity_id, .. } => {
            if *entity_id == world.leader_id()? {
                return sync_explore_player(explore, world, ui, data);
            }
            Ok(false)
        }
    }
}

fn patch_explore_combat(
    explore: &mut ExploreRender,
    event: &CombatEvent,
    world: &WorldState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Result<bool> {
    let leader_id = world.leader_id()?;
    match event {
        CombatEvent::ClearEnemies => {
            if explore.enemies.is_empty() {
                return Ok(false);
            }
            explore.enemies.clear();
            explore.enemy_indices.clear();
            explore.first_live_enemy_name = None;
            Ok(true)
        }
        CombatEvent::MoveEnemy { entity_id, .. }
        | CombatEvent::SetCombatantTimed { entity_id, .. } => {
            if *entity_id == leader_id {
                return sync_explore_player_stats(explore, world);
            }
            patch_or_insert_enemy(explore, *entity_id, world, data, render_fx)
        }
        CombatEvent::RemoveEnemy(entity_id) => {
            if *entity_id == leader_id {
                return sync_explore_player_stats(explore, world);
            }
            if explore.remove_enemy(*entity_id) {
                explore.first_live_enemy_name = explore.first_live_enemy_name();
                return Ok(true);
            }
            Ok(false)
        }
        CombatEvent::SetActive(_)
        | CombatEvent::SetRespawnTimer(_)
        | CombatEvent::GrantKillReward { .. } => Ok(false),
    }
}

fn patch_explore_entity_target(
    explore: &mut ExploreRender,
    entity_id: u32,
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
    sync_leader: bool,
) -> Result<bool> {
    let leader_id = world.leader_id()?;
    if sync_leader && entity_id == leader_id {
        return sync_explore_player(explore, world, ui, data);
    }
    if world
        .combat
        .enemies
        .iter()
        .any(|enemy| enemy.entity_id == entity_id)
    {
        return patch_or_insert_enemy(explore, entity_id, world, data, render_fx);
    }
    Ok(false)
}

fn patch_or_insert_enemy(
    explore: &mut ExploreRender,
    entity_id: u32,
    world: &WorldState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Result<bool> {
    let Some(enemy) = world
        .combat
        .enemies
        .iter()
        .find(|enemy| enemy.entity_id == entity_id)
    else {
        return Ok(explore.remove_enemy(entity_id));
    };
    let entity = world.entity(entity_id)?;
    if entity.map_id != explore.map_id {
        return Ok(explore.remove_enemy(entity_id));
    }

    let name = data.find_enemy(enemy.source_enemy_id)?.name.clone();
    let next = super::render_state::EnemyRender {
        enemy_id: entity_id,
        name,
        x: entity.x,
        y: entity.y,
        hp: entity.current_hp,
        max_hp: entity.stat.base_max_hp,
        attack_cooldown: enemy
            .combatant
            .timed
            .time_left(TimedKind::AttackCooldown, world.tick_counter),
        hit_flash: render_fx.enemy_hit_flash(entity_id),
        dead: entity.current_hp <= 0,
    };

    let changed = if let Some(existing) = explore.enemy_mut(entity_id) {
        if *existing == next {
            false
        } else {
            *existing = next;
            true
        }
    } else {
        explore.upsert_enemy(next);
        true
    };

    if changed {
        explore.first_live_enemy_name = explore.first_live_enemy_name();
    }
    Ok(changed)
}

fn sync_explore_player(
    explore: &mut ExploreRender,
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
) -> Result<bool> {
    let leader_id = world.leader_id()?;
    let leader = world.leader_entity()?;
    let combatant = world.combat.combatant(leader_id)?;

    let mut changed = false;
    if explore.map_id != leader.map_id {
        explore.map_id = leader.map_id;
        changed = true;
    }
    if explore.player_x != leader.x {
        explore.player_x = leader.x;
        changed = true;
    }
    if explore.player_y != leader.y {
        explore.player_y = leader.y;
        changed = true;
    }
    if explore.player_facing != leader.facing {
        explore.player_facing = leader.facing;
        changed = true;
    }
    let next_player_moving = world.movement.is_moving();
    if explore.player_moving != next_player_moving {
        explore.player_moving = next_player_moving;
        changed = true;
    }
    if explore.level != leader.stat.level as u32 {
        explore.level = leader.stat.level as u32;
        changed = true;
    }
    let next_peaceful = data.find_map(leader.map_id)?.peaceful;
    if explore.peaceful != next_peaceful {
        explore.peaceful = next_peaceful;
        changed = true;
    }
    if explore.key_actions != ui.explore.key_actions {
        explore.key_actions = ui.explore.key_actions;
        changed = true;
    }

    let next_hint = interaction_hint_from_world(world, data)?;
    if explore.interaction_hint != next_hint {
        explore.interaction_hint = next_hint;
        changed = true;
    }
    changed |= sync_explore_combatant_stats(explore, leader, combatant, world.tick_counter);
    Ok(changed)
}

fn sync_explore_player_stats(explore: &mut ExploreRender, world: &WorldState) -> Result<bool> {
    let leader_id = world.leader_id()?;
    let leader = world.leader_entity()?;
    let combatant = world.combat.combatant(leader_id)?;
    Ok(sync_explore_combatant_stats(
        explore,
        leader,
        combatant,
        world.tick_counter,
    ))
}

fn sync_explore_combatant_stats(
    explore: &mut ExploreRender,
    entity: &EntityState,
    combatant: &CombatantState,
    current_tick: u32,
) -> bool {
    let mut changed = false;
    let next_hp = entity.current_hp as u32;
    let next_max_hp = entity.stat.base_max_hp as u32;
    let next_mp = entity.current_mp as u32;
    let next_max_mp = entity.stat.base_max_mp as u32;
    if explore.hp != next_hp {
        explore.hp = next_hp;
        changed = true;
    }
    if explore.max_hp != next_max_hp {
        explore.max_hp = next_max_hp;
        changed = true;
    }
    if explore.mp != next_mp {
        explore.mp = next_mp;
        changed = true;
    }
    if explore.max_mp != next_max_mp {
        explore.max_mp = next_max_mp;
        changed = true;
    }
    let next_status = super::render_state::StatusRender::from_timed(&combatant.timed, current_tick);
    if explore.player_status.poison_timer != next_status.poison_timer
        || explore.player_status.stun_timer != next_status.stun_timer
        || explore.player_status.armor_break_timer != next_status.armor_break_timer
    {
        explore.player_status = next_status;
        changed = true;
    }
    let next_cooldowns =
        super::render_state::skill_cooldowns_from_timed(&combatant.timed, current_tick);
    if explore.skill_cooldowns != next_cooldowns {
        explore.skill_cooldowns = next_cooldowns;
        changed = true;
    }
    changed
}

fn sync_explore_quest(
    explore: &mut ExploreRender,
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
) -> Result<bool> {
    let mut changed = false;
    let next_active_count = world
        .quests
        .iter()
        .filter(|quest| !quest.rewarded && !quest.completed)
        .count();
    if explore.active_quest_count != next_active_count {
        explore.active_quest_count = next_active_count;
        changed = true;
    }
    let next_tracked = TrackedQuestRender::from_world(world, data, ui.quest_log.tracked_quest_id)?;
    let tracked_changed = match (&explore.tracked_quest, &next_tracked) {
        (None, None) => false,
        (Some(current), Some(next)) => {
            current.name != next.name
                || current.current_count != next.current_count
                || current.target_count != next.target_count
                || current.completed != next.completed
        }
        _ => true,
    };
    if tracked_changed {
        explore.tracked_quest = next_tracked;
        changed = true;
    }
    Ok(changed)
}

pub fn render(
    state: &RenderState,
    sprites: &SpriteAtlas,
    render_fx: &RenderFxState,
    fb: &mut Framebuffer,
) {
    match state {
        RenderState::Loading { step } => draw_loading(fb, *step),
        RenderState::Menu {
            title,
            items,
            selected,
        } => draw_menu(fb, title, items, *selected),
        RenderState::Explore(explore) => draw_explore(fb, explore, sprites),
        RenderState::Inventory(inventory) => draw_inventory(fb, inventory),
        RenderState::Stats(stats) => draw_stats(fb, stats),
        RenderState::Dialog {
            explore,
            npc_name,
            current_text,
            has_next,
            ..
        } => {
            if let Some(explore_state) = explore {
                draw_explore(fb, explore_state, sprites);
            }
            draw_dialog(fb, npc_name, current_text.as_deref(), *has_next);
        }
        RenderState::Shop(shop) => draw_shop(fb, shop),
        RenderState::QuestLog(quest_log) => draw_quest_log(fb, quest_log),
        RenderState::PauseMenu {
            explore,
            items,
            selected,
        } => {
            if let Some(explore_state) = explore {
                draw_explore(fb, explore_state, sprites);
            }
            draw_pause_menu(fb, items, *selected);
        }
        RenderState::Dead => draw_dead(fb),
        RenderState::Error(msg) => draw_error(fb, msg),
    }

    if render_fx.soft_error_notice_timer() > 0
        && let Some(message) = render_fx.soft_error_message()
    {
        draw_soft_error_notice(fb, message);
    }
}

fn draw_loading(fb: &mut Framebuffer, step: usize) {
    const TOTAL_LOADING_STEPS: usize = 8;

    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let clamped_step = step.min(TOTAL_LOADING_STEPS);
    let percent = (clamped_step * 100) / TOTAL_LOADING_STEPS;

    draw_text(fb, w / 2 - 30, h / 2 - 10, "LOADING", COLOR_WHITE);
    let bar_w = (w - 40).max(80);
    let bar_h = 12;
    let bar_x = (w - bar_w) / 2;
    let bar_y = h / 2 + 12;
    fill_rect(fb, bar_x, bar_y, bar_w, bar_h, COLOR_DARK_GRAY);
    let fill_w = ((bar_w - 2) as usize * clamped_step / TOTAL_LOADING_STEPS) as i32;
    if fill_w > 0 {
        fill_rect(fb, bar_x + 1, bar_y + 1, fill_w, bar_h - 2, COLOR_BLUE);
    }
    draw_rect(fb, bar_x, bar_y, bar_w, bar_h, COLOR_WHITE);

    draw_text(
        fb,
        w / 2 - 28,
        h / 2 + 30,
        &format!("{}% ({}/{})", percent, clamped_step, TOTAL_LOADING_STEPS),
        COLOR_WHITE,
    );
}

fn draw_soft_error_notice(fb: &mut Framebuffer, message: &str) {
    let width = fb.width() as i32;
    let box_x = 12;
    let box_y = 8;
    let box_w = width - 24;
    let box_h = 24;
    fill_rect(fb, box_x, box_y, box_w, box_h, COLOR_DARK_GRAY);
    draw_rect(fb, box_x, box_y, box_w, box_h, COLOR_RED);
    draw_text(fb, box_x + 8, box_y + 8, message, COLOR_WHITE);
}

fn draw_dead(fb: &mut Framebuffer) {
    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    fill_rect(fb, w / 2 - 52, h / 2 - 24, 104, 48, COLOR_DARK_GRAY);
    draw_rect(fb, w / 2 - 52, h / 2 - 24, 104, 48, COLOR_RED);
    draw_text(fb, w / 2 - 35, h / 2 - 10, "YOU DIED", COLOR_RED);
    draw_text(fb, w / 2 - 43, h / 2 + 8, "OK: Revive", COLOR_WHITE);
}

fn draw_error(fb: &mut Framebuffer, msg: &str) {
    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    fill_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_DARK_GRAY);
    draw_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_RED);
    draw_text(fb, 16, h / 2 - 20, "ERROR", COLOR_RED);
    draw_text(fb, 16, h / 2 - 4, msg, COLOR_WHITE);
    draw_text(fb, 16, h / 2 + 16, "OK:Exit", COLOR_WHITE);
}
