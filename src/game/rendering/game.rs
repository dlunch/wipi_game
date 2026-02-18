use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

use wipi::framebuffer::Framebuffer;

use crate::game::state::TimedKind;
use crate::game::ui::{INVENTORY_VISIBLE_ITEMS, SHOP_VISIBLE_ITEMS, ShopMode, UiState};
use crate::game::{
    CombatEvent, EntityEvent, GameData, GameEvent, GameState, SpriteAtlas, WorldEvent, WorldState,
};

use super::dialog::draw_dialog;
use super::explore::draw_explore;
use super::inventory::{draw_inventory, draw_stats};
use super::menu::{draw_menu, draw_pause_menu};
use super::quest::draw_quest_log;
use super::render_fx::RenderFxState;
use super::render_state::{
    EnemyRender, ExploreRender, InventoryRender, QuestLogRender, RenderState, ShopRender,
    StatsRender, StatusRender, TrackedQuestRender, interaction_hint_from_world,
    scroll_for_selection, skill_cooldowns_from_timed,
};
use super::renderer::{
    COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, clear_screen, draw_rect, draw_text, fill_rect,
};
use super::shop::draw_shop;

impl RenderState {
    fn matches_state(&self, state: &GameState) -> bool {
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

    pub fn apply_state(
        &mut self,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) {
        *self = match state {
            GameState::Loading(step) => RenderState::Loading { step: *step },
            GameState::Menu => RenderState::Menu {
                title: ui.menu.state.title,
                items: ui.menu.state.items.clone(),
                selected: ui.menu.selected,
            },
            GameState::Explore => {
                match world.and_then(|world| ExploreRender::from_world(world, ui, data, render_fx))
                {
                    Some(explore) => RenderState::Explore(explore),
                    None => RenderState::NoSession,
                }
            }
            GameState::Inventory => match world {
                Some(world) => RenderState::Inventory(InventoryRender::from_world(world, ui, data)),
                None => RenderState::NoSession,
            },
            GameState::Stats => match world.and_then(StatsRender::from_world) {
                Some(stats) => RenderState::Stats(stats),
                None => RenderState::NoSession,
            },
            GameState::Dialog => {
                if let Some(dialog_state) = ui.dialog.state.as_ref() {
                    let current_text = dialog_state
                        .lines
                        .get(dialog_state.current_line)
                        .map(|line| line.text.clone());
                    RenderState::Dialog {
                        explore: world.and_then(|world| {
                            ExploreRender::from_world(world, ui, data, render_fx)
                        }),
                        npc_name: dialog_state.npc_name.clone(),
                        lines: dialog_state
                            .lines
                            .iter()
                            .map(|line| line.text.clone())
                            .collect(),
                        current_line: dialog_state.current_line,
                        current_text,
                        has_next: dialog_state.current_line + 1 < dialog_state.lines.len(),
                    }
                } else {
                    RenderState::Error(String::from("No dialog state"))
                }
            }
            GameState::Shop => {
                match world.and_then(|world| ShopRender::from_world(world, ui, data, render_fx)) {
                    Some(shop) => RenderState::Shop(shop),
                    None => RenderState::NoSession,
                }
            }
            GameState::QuestLog => match world {
                Some(world) => RenderState::QuestLog(QuestLogRender::from_world(world, ui, data)),
                None => RenderState::NoSession,
            },
            GameState::PauseMenu => RenderState::PauseMenu {
                explore: world
                    .and_then(|world| ExploreRender::from_world(world, ui, data, render_fx)),
                items: ui.pause_menu.state.items.clone(),
                selected: ui.pause_menu.selected,
            },
            GameState::Dead => RenderState::Dead,
            GameState::Error(msg) => RenderState::Error(msg.clone()),
        };
    }

    pub fn apply_game_event(
        &mut self,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        event: &GameEvent,
        render_fx: &RenderFxState,
    ) {
        if !self.matches_state(state) {
            self.apply_state(state, world, ui, data, render_fx);
            return;
        }

        let handled = match self {
            RenderState::Loading { step } => {
                if let GameEvent::Loading(crate::game::LoadingEvent::Advance(next_step)) = event {
                    *step = *next_step;
                }
                true
            }
            RenderState::Menu {
                title,
                items,
                selected,
            } => {
                *title = ui.menu.state.title;
                *items = ui.menu.state.items.clone();
                *selected = ui.menu.selected;
                true
            }
            RenderState::Explore(explore) => {
                apply_event_to_explore_render(explore, world, ui, data, event, render_fx)
            }
            RenderState::Inventory(inventory) => {
                inventory.selected = ui.inventory.selected;
                if matches!(
                    event,
                    GameEvent::Entity(
                        EntityEvent::ClearEntityInventory { .. }
                            | EntityEvent::SetEntityLoadoutSlot { .. }
                            | EntityEvent::ChangeEntityItem { .. }
                            | EntityEvent::RemoveEntity(_)
                            | EntityEvent::CreateEntity { .. }
                    ) | GameEvent::ShopSellSelected(_)
                        | GameEvent::ShopBuyItem(_)
                        | GameEvent::UseInventorySelected(_)
                ) {
                    if let Some(world) = world {
                        *inventory = InventoryRender::from_world(world, ui, data);
                    } else {
                        return;
                    }
                    true
                } else {
                    let inventory_len = world
                        .and_then(|world| {
                            world.leader_entity().map(|leader| leader.inventory.len())
                        })
                        .unwrap_or(0);
                    inventory.scroll = scroll_for_selection(
                        inventory.selected,
                        inventory_len,
                        INVENTORY_VISIBLE_ITEMS,
                    );
                    true
                }
            }
            RenderState::Stats(stats) => sync_stats_render(stats, world),
            RenderState::Dialog {
                explore,
                npc_name,
                lines,
                current_line,
                current_text,
                has_next,
            } => {
                if let Some(dialog_state) = ui.dialog.state.as_ref()
                    && matches!(
                        event,
                        GameEvent::ApplyDialogTransition(_)
                            | GameEvent::OpenDialogState(_)
                            | GameEvent::ApplyDialogAction(_)
                    )
                {
                    *npc_name = dialog_state.npc_name.clone();
                    *lines = dialog_state
                        .lines
                        .iter()
                        .map(|line| line.text.clone())
                        .collect();
                    *current_line = dialog_state.current_line;
                    *current_text = lines.get(*current_line).cloned();
                    *has_next = *current_line + 1 < lines.len();
                }
                if let Some(explore_render) = explore {
                    apply_event_to_explore_render(explore_render, world, ui, data, event, render_fx)
                } else {
                    true
                }
            }
            RenderState::Shop(shop) => {
                shop.mode = ui.shop.mode;
                shop.selected = ui.shop.selected;
                let total = match ui.shop.mode {
                    ShopMode::Select => 2,
                    ShopMode::Buy | ShopMode::ConfirmBuy => shop.buy_items.len(),
                    ShopMode::Sell | ShopMode::ConfirmSell => shop.player_inventory.len(),
                };
                shop.scroll = scroll_for_selection(shop.selected, total, SHOP_VISIBLE_ITEMS);
                if matches!(
                    event,
                    GameEvent::Entity(
                        EntityEvent::ClearEntityInventory { .. }
                            | EntityEvent::SetEntityLoadoutSlot { .. }
                            | EntityEvent::ChangeEntityItem { .. }
                            | EntityEvent::RemoveEntity(_)
                            | EntityEvent::CreateEntity { .. }
                    ) | GameEvent::ShopBuyItem(_)
                        | GameEvent::ShopSellSelected(_)
                        | GameEvent::OpenShopState(_)
                ) {
                    if let Some(world) = world
                        && let Some(next_shop) = ShopRender::from_world(world, ui, data, render_fx)
                    {
                        *shop = next_shop;
                        true
                    } else {
                        return;
                    }
                } else {
                    true
                }
            }
            RenderState::QuestLog(quest_log) => {
                quest_log.tracked_quest_id = ui.quest_log.tracked_quest_id.clone();
                if quest_log.quests.is_empty() {
                    quest_log.selected = 0;
                } else {
                    quest_log.selected = ui.quest_log.selected.min(quest_log.quests.len() - 1);
                }
                if is_quest_world_event(event) {
                    if let Some(world) = world {
                        *quest_log = QuestLogRender::from_world(world, ui, data);
                    } else {
                        return;
                    }
                }
                true
            }
            RenderState::PauseMenu {
                explore,
                items,
                selected,
            } => {
                *items = ui.pause_menu.state.items.clone();
                *selected = ui.pause_menu.selected;
                if let Some(explore_render) = explore {
                    apply_event_to_explore_render(explore_render, world, ui, data, event, render_fx)
                } else {
                    true
                }
            }
            RenderState::Dead => true,
            RenderState::Error(msg) => {
                if let GameState::Error(next) = state {
                    *msg = next.clone();
                }
                true
            }
            RenderState::NoSession => world.is_none(),
        };

        if !handled {
            self.apply_state(state, world, ui, data, render_fx);
        }
    }

    pub fn apply_ui_patch(&mut self, ui: &UiState, world: Option<&WorldState>) {
        match self {
            RenderState::Menu {
                title,
                items,
                selected,
            } => {
                *title = ui.menu.state.title;
                *items = ui.menu.state.items.clone();
                *selected = ui.menu.selected;
            }
            RenderState::PauseMenu {
                items, selected, ..
            } => {
                *items = ui.pause_menu.state.items.clone();
                *selected = ui.pause_menu.selected;
            }
            RenderState::Inventory(inventory) => {
                let inventory_len = world
                    .and_then(|world| world.leader_entity().map(|leader| leader.inventory.len()))
                    .unwrap_or(0);
                inventory.selected = ui.inventory.selected;
                inventory.scroll = scroll_for_selection(
                    inventory.selected,
                    inventory_len,
                    INVENTORY_VISIBLE_ITEMS,
                );
            }
            RenderState::Shop(shop) => {
                let inventory_len = world
                    .and_then(|world| world.leader_entity().map(|leader| leader.inventory.len()))
                    .unwrap_or(0);
                let shop_items_len = ui
                    .shop
                    .state
                    .as_ref()
                    .map(|state| state.items.len())
                    .unwrap_or(0);
                shop.mode = ui.shop.mode;
                shop.selected = ui.shop.selected;
                let total = match ui.shop.mode {
                    ShopMode::Select => 2,
                    ShopMode::Buy | ShopMode::ConfirmBuy => shop_items_len,
                    ShopMode::Sell | ShopMode::ConfirmSell => inventory_len,
                };
                shop.scroll = scroll_for_selection(shop.selected, total, SHOP_VISIBLE_ITEMS);
            }
            RenderState::QuestLog(quest_log) => {
                quest_log.tracked_quest_id = ui.quest_log.tracked_quest_id.clone();
                if quest_log.quests.is_empty() {
                    quest_log.selected = 0;
                } else {
                    quest_log.selected = ui.quest_log.selected.min(quest_log.quests.len() - 1);
                }
            }
            RenderState::Dialog {
                npc_name,
                lines,
                current_line,
                current_text,
                has_next,
                ..
            } => {
                if let Some(dialog_state) = ui.dialog.state.as_ref() {
                    *npc_name = dialog_state.npc_name.clone();
                    *lines = dialog_state
                        .lines
                        .iter()
                        .map(|line| line.text.clone())
                        .collect();
                    *current_line = dialog_state.current_line;
                    *current_text = lines.get(*current_line).cloned();
                    *has_next = *current_line + 1 < lines.len();
                }
            }
            _ => {}
        }
    }

    pub fn apply_tick(&mut self, render_fx: &RenderFxState) {
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
                explore.player_hit_flash = render_fx.player_hit_flash();
                for enemy in &mut explore.enemies {
                    enemy.hit_flash = render_fx.enemy_hit_flash(enemy.enemy_id);
                }
                explore.quest_notice_timer = render_fx.quest_notice_timer();
                explore.anim_tick = render_fx.anim_tick();
            }
            RenderState::Shop(shop) => {
                shop.purchase_notice_timer = render_fx.shop_purchase_notice_timer();
            }
            _ => {}
        }
    }
}

fn sync_stats_render(stats: &mut StatsRender, world: Option<&WorldState>) -> bool {
    let Some(world) = world else {
        return false;
    };
    let Some(leader_id) = world.leader_id() else {
        return false;
    };
    let Some(leader) = world.leader_entity() else {
        return false;
    };
    let Some(combatant) = world.combat.combatant(leader_id) else {
        return false;
    };

    stats.hp = combatant.stats.current_hp as u32;
    stats.max_hp = combatant.stats.max_hp as u32;
    stats.mp = combatant.stats.current_mp as u32;
    stats.max_mp = combatant.stats.max_mp as u32;
    stats.level = leader.stat.level as u32;
    stats.atk = combatant.stats.atk as u32;
    stats.def = combatant.stats.def as u32;
    stats.exp = leader.stat.exp as u32;
    stats.gold = world.gold_amount(leader_id) as u32;
    true
}

fn sync_explore_runtime(
    explore: &mut ExploreRender,
    world: &WorldState,
    ui: &UiState,
    render_fx: &RenderFxState,
) -> bool {
    let Some(leader_id) = world.leader_id() else {
        return false;
    };
    let Some(leader) = world.leader_entity() else {
        return false;
    };
    let Some(leader_combatant) = world.combat.combatant(leader_id) else {
        return false;
    };

    explore.player_x = leader.x;
    explore.player_y = leader.y;
    explore.player_facing = leader.facing;
    explore.player_moving = world.movement.pressed_direction.is_some();
    explore.hp = leader_combatant.stats.current_hp as u32;
    explore.max_hp = leader_combatant.stats.max_hp as u32;
    explore.mp = leader_combatant.stats.current_mp as u32;
    explore.max_mp = leader_combatant.stats.max_mp as u32;
    explore.level = leader.stat.level as u32;
    explore.player_status = StatusRender::from_timed(&leader_combatant.timed);
    explore.skill_cooldowns = skill_cooldowns_from_timed(&leader_combatant.timed);
    explore.key_actions = ui.explore.key_actions;
    explore.player_hit_flash = render_fx.player_hit_flash();
    for enemy in &mut explore.enemies {
        enemy.hit_flash = render_fx.enemy_hit_flash(enemy.enemy_id);
    }
    explore.quest_notice_timer = render_fx.quest_notice_timer();
    explore.anim_tick = render_fx.anim_tick();
    true
}

fn enemy_render_from_world(
    world: &WorldState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
    entity_id: u32,
) -> Option<EnemyRender> {
    let enemy = world
        .combat
        .enemies
        .iter()
        .find(|enemy| enemy.entity_id == entity_id)?;
    let entity = world.entity(entity_id)?;
    let name = data
        .find_enemy(&enemy.source_enemy_id)
        .map(|enemy_data| enemy_data.name.clone())
        .unwrap_or_else(|| enemy.source_enemy_id.clone());
    Some(EnemyRender {
        enemy_id: entity_id,
        name,
        x: entity.x,
        y: entity.y,
        hp: enemy.combatant.stats.current_hp,
        max_hp: enemy.combatant.stats.max_hp,
        attack_cooldown: enemy.combatant.timed.time_left(TimedKind::AttackCooldown),
        hit_flash: render_fx.enemy_hit_flash(entity_id),
        dead: enemy.combatant.stats.current_hp <= 0,
    })
}

fn refresh_first_live_enemy_name(explore: &mut ExploreRender) {
    explore.first_live_enemy_name = explore
        .enemies
        .iter()
        .find(|enemy| enemy.hp > 0)
        .map(|enemy| enemy.name.clone());
}

fn upsert_enemy_render(
    explore: &mut ExploreRender,
    world: &WorldState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
    entity_id: u32,
) {
    let Some(next_enemy) = enemy_render_from_world(world, data, render_fx, entity_id) else {
        return;
    };
    if let Some(existing) = explore
        .enemies
        .iter_mut()
        .find(|enemy| enemy.enemy_id == entity_id)
    {
        *existing = next_enemy;
    } else {
        explore.enemies.push(next_enemy);
    }
}

fn apply_event_to_explore_render(
    explore: &mut ExploreRender,
    world: Option<&WorldState>,
    ui: &UiState,
    data: &Rc<GameData>,
    event: &GameEvent,
    render_fx: &RenderFxState,
) -> bool {
    let Some(world) = world else {
        return false;
    };
    if !sync_explore_runtime(explore, world, ui, render_fx) {
        return false;
    }

    let leader_id = world.leader_id();
    let mut enemy_changed = false;
    let mut requires_hint_refresh = false;

    match event {
        GameEvent::Movement(_) | GameEvent::Explore(_) => {
            requires_hint_refresh = true;
        }
        GameEvent::Transition(crate::game::TransitionEvent::ReleaseMovementDirection(_)) => {
            requires_hint_refresh = true;
        }
        GameEvent::Transition(crate::game::TransitionEvent::MapChanged) => {
            if let Some(next) = ExploreRender::from_world(world, ui, data, render_fx) {
                *explore = next;
                return true;
            }
            return false;
        }
        GameEvent::Combat(combat_event) => match combat_event {
            CombatEvent::MoveEnemy { entity_id, x, y } => {
                if let Some(enemy) = explore
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.enemy_id == *entity_id)
                {
                    enemy.x = *x;
                    enemy.y = *y;
                    enemy_changed = true;
                } else {
                    upsert_enemy_render(explore, world, data, render_fx, *entity_id);
                    enemy_changed = true;
                }
            }
            CombatEvent::SetCombatantMaxHp { entity_id, .. }
            | CombatEvent::SetCombatantCurrentHp { entity_id, .. }
            | CombatEvent::SetCombatantMaxMp { entity_id, .. }
            | CombatEvent::SetCombatantCurrentMp { entity_id, .. }
            | CombatEvent::SetCombatantAtk { entity_id, .. }
            | CombatEvent::SetCombatantDef { entity_id, .. } => {
                if Some(*entity_id) != leader_id {
                    upsert_enemy_render(explore, world, data, render_fx, *entity_id);
                    enemy_changed = true;
                }
            }
            CombatEvent::SetCombatantTimed {
                entity_id, kind, ..
            } => {
                if Some(*entity_id) != leader_id && matches!(kind, TimedKind::AttackCooldown) {
                    upsert_enemy_render(explore, world, data, render_fx, *entity_id);
                    enemy_changed = true;
                }
            }
            CombatEvent::RemoveEnemy(entity_id) => {
                let before = explore.enemies.len();
                explore.enemies.retain(|enemy| enemy.enemy_id != *entity_id);
                enemy_changed = before != explore.enemies.len();
            }
            CombatEvent::ClearEnemies => {
                explore.enemies.clear();
                enemy_changed = true;
            }
            _ => {}
        },
        GameEvent::World(world_event) => match world_event {
            WorldEvent::AddOpenedTreasure { map_id, x, y } => {
                if &explore.map_id == map_id
                    && !explore
                        .opened_treasures
                        .iter()
                        .any(|(m, tx, ty)| m == map_id && *tx == *x && *ty == *y)
                {
                    explore.opened_treasures.push((map_id.clone(), *x, *y));
                }
            }
            WorldEvent::CreateQuestProgress { .. }
            | WorldEvent::ChangeQuestCurrentCount { .. }
            | WorldEvent::SetQuestCompleted { .. }
            | WorldEvent::SetQuestRewarded { .. } => {
                explore.active_quest_count = world
                    .quests
                    .iter()
                    .filter(|quest| !quest.rewarded && !quest.completed)
                    .count();
                explore.tracked_quest = TrackedQuestRender::from_world(
                    world,
                    data,
                    ui.quest_log.tracked_quest_id.as_deref(),
                );
            }
            WorldEvent::SetWorldMap(_) => {
                if let Some(next) = ExploreRender::from_world(world, ui, data, render_fx) {
                    *explore = next;
                    return true;
                }
                return false;
            }
            WorldEvent::CreateWorld => {}
        },
        GameEvent::Entity(entity_event) => match entity_event {
            EntityEvent::SetEntityTransform {
                entity_id,
                map_id,
                position,
                facing,
            } => {
                if map_id.is_some() {
                    if let Some(next) = ExploreRender::from_world(world, ui, data, render_fx) {
                        *explore = next;
                        return true;
                    }
                    return false;
                }
                if Some(*entity_id) == leader_id {
                    if position.is_some() || facing.is_some() {
                        requires_hint_refresh = true;
                    }
                } else {
                    upsert_enemy_render(explore, world, data, render_fx, *entity_id);
                    enemy_changed = true;
                }
            }
            EntityEvent::SetEntityLevel { entity_id, .. } => {
                if Some(*entity_id) == leader_id
                    && let Some(leader) = world.leader_entity()
                {
                    explore.level = leader.stat.level as u32;
                }
            }
            EntityEvent::CreateEntity {
                entity_id, kind, ..
            } => {
                if matches!(kind, crate::game::EntityKind::Enemy) {
                    upsert_enemy_render(explore, world, data, render_fx, *entity_id);
                    enemy_changed = true;
                }
            }
            EntityEvent::RemoveEntity(entity_id) => {
                let before = explore.enemies.len();
                explore.enemies.retain(|enemy| enemy.enemy_id != *entity_id);
                enemy_changed = before != explore.enemies.len();
            }
            EntityEvent::ClearEntityInventory { .. }
            | EntityEvent::SetEntityLoadoutSlot { .. }
            | EntityEvent::SetEntityExp { .. }
            | EntityEvent::SetEntityExpToNext { .. }
            | EntityEvent::SetEntityBaseMaxHp { .. }
            | EntityEvent::SetEntityBaseMaxMp { .. }
            | EntityEvent::SetEntityBaseAtk { .. }
            | EntityEvent::SetEntityBaseDef { .. }
            | EntityEvent::AddEntityExp { .. }
            | EntityEvent::ChangeEntityItem { .. }
            | EntityEvent::SetLeaderEntity(_)
            | EntityEvent::ClearCompanionEntities
            | EntityEvent::AddCompanionEntity(_) => {}
        },
        _ => {}
    }

    if requires_hint_refresh {
        explore.interaction_hint = interaction_hint_from_world(world, data);
    }
    if enemy_changed {
        refresh_first_live_enemy_name(explore);
    }
    true
}

fn is_quest_world_event(event: &GameEvent) -> bool {
    matches!(
        event,
        GameEvent::World(
            WorldEvent::CreateQuestProgress { .. }
                | WorldEvent::ChangeQuestCurrentCount { .. }
                | WorldEvent::SetQuestCompleted { .. }
                | WorldEvent::SetQuestRewarded { .. }
        )
    )
}

pub fn render(state: &RenderState, sprites: &SpriteAtlas, fb: &mut Framebuffer) {
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
        RenderState::NoSession => {
            clear_screen(fb);
            draw_text(fb, 16, 16, "No active session", COLOR_RED);
        }
    }
}

fn draw_loading(fb: &mut Framebuffer, step: usize) {
    clear_screen(fb);
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    draw_text(fb, w / 2 - 30, h / 2 - 10, "LOADING", COLOR_WHITE);
    draw_text(
        fb,
        w / 2 - 20,
        h / 2 + 8,
        &format!("Step {}", step),
        COLOR_WHITE,
    );
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
