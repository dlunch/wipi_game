use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use wipi::framebuffer::Framebuffer;

use crate::game::state::TimedKind;
use crate::game::ui::{INVENTORY_VISIBLE_ITEMS, SHOP_VISIBLE_ITEMS, ShopMode, UiState};
use crate::game::{
    CombatEvent, EntityEvent, GameData, GameEvent, GameState, LoadingEvent, MovementEvent,
    SpriteAtlas, TransitionEvent, WorldEvent, WorldState,
};

use super::dialog::draw_dialog;
use super::explore::draw_explore;
use super::inventory::{draw_inventory, draw_stats};
use super::menu::{draw_menu, draw_pause_menu};
use super::quest::draw_quest_log;
use super::render_fx::RenderFxState;
use super::render_state::{
    ExploreRender, InventoryRender, QuestLogRender, RenderState, ShopRender, SkillEffectRender,
    StatsRender, TrackedQuestRender, interaction_hint_from_world, scroll_for_selection,
};
use super::renderer::{
    COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, clear_screen, draw_rect, draw_text, fill_rect,
};
use super::shop::draw_shop;

impl RenderState {
    pub fn on_state_changed(
        &mut self,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> bool {
        *self = match state {
            GameState::Loading(step) => Self::enter_loading(*step),
            GameState::Menu => Self::enter_menu(ui),
            GameState::Explore => Self::enter_explore(world, ui, data, render_fx),
            GameState::Inventory => Self::enter_inventory(world, ui, data),
            GameState::Stats => Self::enter_stats(world),
            GameState::Dialog => Self::enter_dialog(world, ui, data, render_fx),
            GameState::Shop => Self::enter_shop(world, ui, data, render_fx),
            GameState::QuestLog => Self::enter_quest_log(world, ui, data),
            GameState::PauseMenu => Self::enter_pause_menu(world, ui, data, render_fx),
            GameState::Dead => Self::enter_dead(),
            GameState::Error(msg) => Self::enter_error(msg),
        };
        true
    }

    pub fn apply_game_event_patch(
        &mut self,
        event: &GameEvent,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> bool {
        match self {
            RenderState::Loading { step } => {
                if let GameEvent::Loading(LoadingEvent::Advance(next_step)) = event
                    && *step != *next_step
                {
                    *step = *next_step;
                    return true;
                }
                false
            }
            RenderState::Menu { .. } => false,
            RenderState::Explore(explore) => {
                patch_explore(explore, event, world, ui, data, render_fx, state)
            }
            RenderState::Inventory(inventory) => {
                if matches!(
                    event,
                    GameEvent::UseInventorySelected(_)
                        | GameEvent::Entity(_)
                        | GameEvent::Transition(_)
                        | GameEvent::OpenShopState(_)
                ) && let Some(world) = world
                {
                    *inventory = InventoryRender::from_world(world, ui, data);
                    return true;
                }
                false
            }
            RenderState::Stats(stats) => {
                if matches!(event, GameEvent::Combat(_) | GameEvent::Entity(_))
                    && let Some(world) = world
                    && let Some(next) = StatsRender::from_world(world)
                {
                    *stats = next;
                    return true;
                }
                false
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
                    changed |= patch_explore(explore, event, world, ui, data, render_fx, state);
                }
                match event {
                    GameEvent::ApplyDialogTransition(crate::game::DialogTransition::SetLine(
                        line,
                    )) => {
                        if *current_line != *line {
                            *current_line = *line;
                            *current_text = lines.get(*current_line).cloned();
                            *has_next = *current_line + 1 < lines.len();
                            changed = true;
                        }
                    }
                    GameEvent::OpenDialogState(dialog_state) => {
                        let next_lines: Vec<String> = dialog_state
                            .lines
                            .iter()
                            .map(|line| line.text.clone())
                            .collect();
                        let next_line = dialog_state
                            .current_line
                            .min(next_lines.len().saturating_sub(1));
                        let next_text = next_lines.get(next_line).cloned();
                        let next_has_next = next_line + 1 < next_lines.len();
                        if *npc_name != dialog_state.npc_name {
                            *npc_name = dialog_state.npc_name.clone();
                            changed = true;
                        }
                        if *lines != next_lines {
                            *lines = next_lines;
                            changed = true;
                        }
                        if *current_line != next_line {
                            *current_line = next_line;
                            changed = true;
                        }
                        if *current_text != next_text {
                            *current_text = next_text;
                            changed = true;
                        }
                        if *has_next != next_has_next {
                            *has_next = next_has_next;
                            changed = true;
                        }
                    }
                    _ => {}
                }
                changed
            }
            RenderState::Shop(shop) => {
                if matches!(
                    event,
                    GameEvent::ShopBuyItem(_)
                        | GameEvent::ShopSellSelected(_)
                        | GameEvent::Entity(_)
                        | GameEvent::OpenShopState(_)
                        | GameEvent::Transition(_)
                ) && let Some(world) = world
                    && let Some(next) = ShopRender::from_world(world, ui, data, render_fx)
                {
                    *shop = next;
                    return true;
                }
                false
            }
            RenderState::QuestLog(quest_log) => {
                if matches!(event, GameEvent::World(_) | GameEvent::Transition(_))
                    && let Some(world) = world
                {
                    *quest_log = QuestLogRender::from_world(world, ui, data);
                    return true;
                }
                false
            }
            RenderState::PauseMenu { explore, .. } => {
                if let Some(explore) = explore.as_mut() {
                    return patch_explore(explore, event, world, ui, data, render_fx, state);
                }
                false
            }
            RenderState::Dead | RenderState::Error(_) | RenderState::NoSession => false,
        }
    }

    pub fn apply_ui_patch(&mut self, ui: &UiState, world: Option<&WorldState>) -> bool {
        match self {
            RenderState::Menu {
                title,
                items,
                selected,
            } => {
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
                changed
            }
            RenderState::Inventory(inventory) => {
                let inventory_len = world
                    .and_then(|world| world.leader_entity().map(|leader| leader.inventory.len()))
                    .unwrap_or(0);
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
                changed
            }
            RenderState::Shop(shop) => {
                let inventory_len = world
                    .and_then(|world| world.leader_entity().map(|leader| leader.inventory.len()))
                    .unwrap_or(0);
                let shop_items_len = ui
                    .shop
                    .state
                    .as_ref()
                    .map(|shop_state| shop_state.items.len())
                    .unwrap_or(0);
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
                changed
            }
            RenderState::QuestLog(quest_log) => {
                let mut changed = false;
                if quest_log.tracked_quest_id != ui.quest_log.tracked_quest_id {
                    quest_log.tracked_quest_id = ui.quest_log.tracked_quest_id.clone();
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
                changed
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
                    changed
                } else {
                    false
                }
            }
            _ => false,
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
                let next_skill_effects: Vec<SkillEffectRender> = render_fx
                    .skill_effect_iter()
                    .map(|(x, y, effect_type)| SkillEffectRender { x, y, effect_type })
                    .collect();
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

    fn enter_loading(step: usize) -> Self {
        RenderState::Loading { step }
    }

    fn enter_menu(ui: &UiState) -> Self {
        RenderState::Menu {
            title: ui.menu.state.title,
            items: ui.menu.state.items.clone(),
            selected: ui.menu.selected,
        }
    }

    fn enter_explore(
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Self {
        match world.and_then(|world| ExploreRender::from_world(world, ui, data, render_fx)) {
            Some(explore) => RenderState::Explore(explore),
            None => RenderState::NoSession,
        }
    }

    fn enter_inventory(world: Option<&WorldState>, ui: &UiState, data: &Rc<GameData>) -> Self {
        match world {
            Some(world) => RenderState::Inventory(InventoryRender::from_world(world, ui, data)),
            None => RenderState::NoSession,
        }
    }

    fn enter_stats(world: Option<&WorldState>) -> Self {
        match world.and_then(StatsRender::from_world) {
            Some(stats) => RenderState::Stats(stats),
            None => RenderState::NoSession,
        }
    }

    fn enter_dialog(
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Self {
        if let Some(dialog_state) = ui.dialog.state.as_ref() {
            let lines: Vec<String> = dialog_state
                .lines
                .iter()
                .map(|line| line.text.clone())
                .collect();
            let current_line = dialog_state.current_line.min(lines.len().saturating_sub(1));
            let current_text = lines.get(current_line).cloned();
            return RenderState::Dialog {
                explore: world
                    .and_then(|world| ExploreRender::from_world(world, ui, data, render_fx)),
                npc_name: dialog_state.npc_name.clone(),
                lines,
                current_line,
                current_text,
                has_next: current_line + 1 < dialog_state.lines.len(),
            };
        }
        RenderState::Error(String::from("No dialog state"))
    }

    fn enter_shop(
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Self {
        match world.and_then(|world| ShopRender::from_world(world, ui, data, render_fx)) {
            Some(shop) => RenderState::Shop(shop),
            None => RenderState::NoSession,
        }
    }

    fn enter_quest_log(world: Option<&WorldState>, ui: &UiState, data: &Rc<GameData>) -> Self {
        match world {
            Some(world) => RenderState::QuestLog(QuestLogRender::from_world(world, ui, data)),
            None => RenderState::NoSession,
        }
    }

    fn enter_pause_menu(
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Self {
        RenderState::PauseMenu {
            explore: world.and_then(|world| ExploreRender::from_world(world, ui, data, render_fx)),
            items: ui.pause_menu.state.items.clone(),
            selected: ui.pause_menu.selected,
        }
    }

    fn enter_dead() -> Self {
        RenderState::Dead
    }

    fn enter_error(msg: &str) -> Self {
        RenderState::Error(msg.into())
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
) -> bool {
    let Some(world) = world else {
        return false;
    };

    match event {
        GameEvent::World(WorldEvent::SetWorldMap(_))
        | GameEvent::Transition(TransitionEvent::MapChanged) => {
            if let Some(next) = ExploreRender::from_world(world, ui, data, render_fx) {
                *explore = next;
                return true;
            }
            false
        }
        GameEvent::Movement(MovementEvent::Tick(..))
        | GameEvent::Entity(EntityEvent::SetEntityTransform { .. })
        | GameEvent::Transition(TransitionEvent::ReleaseMovementDirection(_))
        | GameEvent::Explore(crate::game::ExploreEvent::MoveDirection(_)) => {
            sync_explore_player(explore, world, ui, data)
        }
        GameEvent::Entity(EntityEvent::SetEntityLevel { entity_id, .. }) => {
            if Some(*entity_id) == world.leader_id() {
                return sync_explore_player(explore, world, ui, data);
            }
            false
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
                return false;
            }
            if !explore
                .opened_treasures
                .iter()
                .any(|(m, tx, ty)| m == map_id && *tx == *x && *ty == *y)
            {
                explore.opened_treasures.push((map_id.clone(), *x, *y));
                explore.interaction_hint = interaction_hint_from_world(world, data);
                return true;
            }
            false
        }
        GameEvent::ApplyDialogAction(_) | GameEvent::ApplyDialogTransition(_) => {
            if matches!(
                state,
                GameState::Explore | GameState::Dialog | GameState::PauseMenu
            ) {
                return sync_explore_quest(explore, world, ui, data);
            }
            false
        }
        _ => false,
    }
}

fn patch_explore_combat(
    explore: &mut ExploreRender,
    event: &CombatEvent,
    world: &WorldState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> bool {
    let leader_id = world.leader_id();
    match event {
        CombatEvent::ClearEnemies => {
            if explore.enemies.is_empty() {
                return false;
            }
            explore.enemies.clear();
            explore.enemy_indices.clear();
            explore.first_live_enemy_name = None;
            true
        }
        CombatEvent::MoveEnemy { entity_id, .. }
        | CombatEvent::SetCombatantCurrentHp { entity_id, .. }
        | CombatEvent::SetCombatantMaxHp { entity_id, .. }
        | CombatEvent::SetCombatantTimed { entity_id, .. }
        | CombatEvent::RemoveEnemy(entity_id) => {
            if Some(*entity_id) == leader_id {
                return sync_explore_player_stats(explore, world);
            }
            if let CombatEvent::RemoveEnemy(entity_id) = event {
                if explore.remove_enemy(*entity_id) {
                    explore.first_live_enemy_name = explore.first_live_enemy_name();
                    return true;
                }
                return false;
            }
            patch_or_insert_enemy(explore, *entity_id, world, data, render_fx)
        }
        CombatEvent::SetCombatantCurrentMp { entity_id, .. }
        | CombatEvent::SetCombatantMaxMp { entity_id, .. } => {
            if Some(*entity_id) == leader_id {
                return sync_explore_player_stats(explore, world);
            }
            false
        }
        CombatEvent::SetCombatantAtk { .. } | CombatEvent::SetCombatantDef { .. } => false,
        CombatEvent::SetActive(_)
        | CombatEvent::ClearAllies
        | CombatEvent::SetUpdateCounter(_)
        | CombatEvent::SetRespawnTimer(_)
        | CombatEvent::GrantKillReward { .. }
        | CombatEvent::RecoverMp { .. }
        | CombatEvent::Heal { .. }
        | CombatEvent::TakeDamage { .. } => false,
    }
}

fn patch_or_insert_enemy(
    explore: &mut ExploreRender,
    entity_id: u32,
    world: &WorldState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> bool {
    let Some(enemy) = world
        .combat
        .enemies
        .iter()
        .find(|enemy| enemy.entity_id == entity_id)
    else {
        return explore.remove_enemy(entity_id);
    };
    let Some(entity) = world.entity(entity_id) else {
        return explore.remove_enemy(entity_id);
    };
    if entity.map_id != explore.map_id {
        return explore.remove_enemy(entity_id);
    }

    let name = data
        .find_enemy(&enemy.source_enemy_id)
        .map(|enemy_data| enemy_data.name.clone())
        .unwrap_or_else(|| enemy.source_enemy_id.clone());
    let next = super::render_state::EnemyRender {
        enemy_id: entity_id,
        name,
        x: entity.x,
        y: entity.y,
        hp: enemy.combatant.stats.current_hp,
        max_hp: enemy.combatant.stats.max_hp,
        attack_cooldown: enemy.combatant.timed.time_left(TimedKind::AttackCooldown),
        hit_flash: render_fx.enemy_hit_flash(entity_id),
        dead: enemy.combatant.stats.current_hp <= 0,
    };

    let changed = if let Some(existing) = explore.enemy_mut(entity_id) {
        let mut event_changed = false;
        if existing.x != next.x {
            existing.x = next.x;
            event_changed = true;
        }
        if existing.y != next.y {
            existing.y = next.y;
            event_changed = true;
        }
        if existing.hp != next.hp {
            existing.hp = next.hp;
            event_changed = true;
        }
        if existing.max_hp != next.max_hp {
            existing.max_hp = next.max_hp;
            event_changed = true;
        }
        if existing.attack_cooldown != next.attack_cooldown {
            existing.attack_cooldown = next.attack_cooldown;
            event_changed = true;
        }
        if existing.dead != next.dead {
            existing.dead = next.dead;
            event_changed = true;
        }
        if existing.name != next.name {
            existing.name = next.name;
            event_changed = true;
        }
        event_changed
    } else {
        explore.upsert_enemy(next);
        true
    };

    if changed {
        explore.first_live_enemy_name = explore.first_live_enemy_name();
    }
    changed
}

fn sync_explore_player(
    explore: &mut ExploreRender,
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
) -> bool {
    let Some(leader_id) = world.leader_id() else {
        return false;
    };
    let Some(leader) = world.leader_entity() else {
        return false;
    };
    let Some(combatant) = world.combat.combatant(leader_id) else {
        return false;
    };

    let mut changed = false;
    if explore.map_id != leader.map_id {
        explore.map_id = leader.map_id.clone();
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
    let next_player_moving = world.movement.pressed_direction.is_some();
    if explore.player_moving != next_player_moving {
        explore.player_moving = next_player_moving;
        changed = true;
    }
    if explore.level != leader.stat.level as u32 {
        explore.level = leader.stat.level as u32;
        changed = true;
    }
    let next_peaceful = data
        .find_map(&leader.map_id)
        .map(|map| map.peaceful)
        .unwrap_or(false);
    if explore.peaceful != next_peaceful {
        explore.peaceful = next_peaceful;
        changed = true;
    }
    if explore.key_actions != ui.explore.key_actions {
        explore.key_actions = ui.explore.key_actions;
        changed = true;
    }

    let next_hint = interaction_hint_from_world(world, data);
    if explore.interaction_hint != next_hint {
        explore.interaction_hint = next_hint;
        changed = true;
    }

    let next_hp = combatant.stats.current_hp as u32;
    let next_max_hp = combatant.stats.max_hp as u32;
    let next_mp = combatant.stats.current_mp as u32;
    let next_max_mp = combatant.stats.max_mp as u32;
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
    let next_status = super::render_state::StatusRender::from_timed(&combatant.timed);
    if explore.player_status.poison_timer != next_status.poison_timer
        || explore.player_status.stun_timer != next_status.stun_timer
        || explore.player_status.armor_break_timer != next_status.armor_break_timer
    {
        explore.player_status = next_status;
        changed = true;
    }
    let next_cooldowns = super::render_state::skill_cooldowns_from_timed(&combatant.timed);
    if explore.skill_cooldowns != next_cooldowns {
        explore.skill_cooldowns = next_cooldowns;
        changed = true;
    }
    changed
}

fn sync_explore_player_stats(explore: &mut ExploreRender, world: &WorldState) -> bool {
    let Some(leader_id) = world.leader_id() else {
        return false;
    };
    let Some(combatant) = world.combat.combatant(leader_id) else {
        return false;
    };
    let mut changed = false;
    let next_hp = combatant.stats.current_hp as u32;
    let next_max_hp = combatant.stats.max_hp as u32;
    let next_mp = combatant.stats.current_mp as u32;
    let next_max_mp = combatant.stats.max_mp as u32;
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
    let next_status = super::render_state::StatusRender::from_timed(&combatant.timed);
    if explore.player_status.poison_timer != next_status.poison_timer
        || explore.player_status.stun_timer != next_status.stun_timer
        || explore.player_status.armor_break_timer != next_status.armor_break_timer
    {
        explore.player_status = next_status;
        changed = true;
    }
    let next_cooldowns = super::render_state::skill_cooldowns_from_timed(&combatant.timed);
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
) -> bool {
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
    let next_tracked =
        TrackedQuestRender::from_world(world, data, ui.quest_log.tracked_quest_id.as_deref());
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
    changed
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
