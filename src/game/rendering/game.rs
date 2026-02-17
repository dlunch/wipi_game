use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use wipi::framebuffer::Framebuffer;

use crate::data::{Direction, ItemKind, NpcType, SkillType, Tile};
use crate::game::state::{TimedKind, TimedState};
use crate::game::ui::{
    ExploreAction, INVENTORY_VISIBLE_ITEMS, MenuAction, SHOP_VISIBLE_ITEMS, ShopMode, UiState,
};
use crate::game::{
    CombatEvent, GameData, GameEvent, GameState, SpriteAtlas, WorldEvent, WorldState,
};

use super::dialog::draw_dialog;
use super::explore::draw_explore;
use super::inventory::{draw_inventory, draw_stats};
use super::menu::{draw_menu, draw_pause_menu};
use super::quest::draw_quest_log;
use super::renderer::{
    COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, clear_screen, draw_rect, draw_text, fill_rect,
};
use super::shop::draw_shop;

pub enum RenderState {
    Loading {
        step: usize,
    },
    Menu {
        title: &'static str,
        items: Vec<(&'static str, MenuAction)>,
        selected: usize,
    },
    Explore(ExploreRender),
    Inventory(InventoryRender),
    Stats(StatsRender),
    Dialog {
        explore: Option<ExploreRender>,
        npc_name: String,
        lines: Vec<String>,
        current_line: usize,
        current_text: Option<String>,
        has_next: bool,
    },
    Shop(ShopRender),
    QuestLog(QuestLogRender),
    PauseMenu {
        explore: Option<ExploreRender>,
        items: Vec<&'static str>,
        selected: usize,
    },
    Dead,
    Error(String),
    NoSession,
}

#[derive(Clone, Copy, Default)]
pub struct StatusRender {
    pub poison_timer: u32,
    pub stun_timer: u32,
    pub armor_break_timer: u32,
}

pub struct ExploreRender {
    pub data: Rc<GameData>,
    pub map_id: String,
    pub player_x: usize,
    pub player_y: usize,
    pub player_facing: Direction,
    pub player_moving: bool,
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub level: u32,
    pub active_quest_count: usize,
    pub tracked_quest: Option<TrackedQuestRender>,
    pub interaction_hint: Option<String>,
    pub first_live_enemy_name: Option<String>,
    pub opened_treasures: Vec<(String, usize, usize)>,
    pub enemies: Vec<EnemyRender>,
    pub player_hit_flash: u32,
    pub skill_effects: Vec<SkillEffectRender>,
    pub skill_cooldowns: [u32; 3],
    pub player_status: StatusRender,
    pub key_actions: [Option<ExploreAction>; 3],
    pub peaceful: bool,
    pub quest_notice_timer: u32,
    pub anim_tick: u32,
}

pub struct EnemyRender {
    pub enemy_id: u32,
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub attack_cooldown: u32,
    pub hit_flash: u32,
    pub dead: bool,
}

pub struct SkillEffectRender {
    pub x: usize,
    pub y: usize,
    pub effect_type: SkillType,
}

pub struct InventoryRender {
    pub items: Vec<InventoryItemRender>,
    pub equipped_weapon: Option<usize>,
    pub equipped_armor: Option<usize>,
    pub equipped_accessory: Option<usize>,
    pub selected: usize,
    pub scroll: usize,
}

pub struct InventoryItemRender {
    pub name: String,
    pub kind: ItemKind,
}

pub struct StatsRender {
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub level: u32,
    pub atk: u32,
    pub def: u32,
    pub exp: u32,
    pub gold: u32,
}

pub struct ShopRender {
    pub shop_name: String,
    pub mode: ShopMode,
    pub selected: usize,
    pub scroll: usize,
    pub buy_items: Vec<ShopItemRender>,
    pub player_gold: i32,
    pub player_inventory: Vec<ShopItemRender>,
    pub purchase_notice_timer: u32,
}

pub struct ShopItemRender {
    pub name: String,
    pub price: i32,
}

pub struct QuestLogRender {
    pub quests: Vec<QuestEntryRender>,
    pub selected: usize,
    pub tracked_quest_id: Option<String>,
}

pub struct QuestEntryRender {
    pub quest_id: String,
    pub name: String,
    pub description: String,
    pub current_count: u32,
    pub target_count: u32,
    pub completed: bool,
}

pub struct TrackedQuestRender {
    pub name: String,
    pub current_count: u32,
    pub target_count: u32,
    pub completed: bool,
}

#[derive(Default)]
pub struct RenderFxState {
    player_hit_flash: u32,
    enemy_hit_flashes: Vec<(u32, u32)>,
    quest_notice_timer: u32,
    shop_purchase_notice_timer: u32,
    anim_tick: u32,
}

const QUEST_NOTICE_DURATION: u32 = 90;
const SHOP_PURCHASE_NOTICE_DURATION: u32 = 45;

impl RenderFxState {
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        self.anim_tick = self.anim_tick.wrapping_add(1);
        if self.player_hit_flash > 0 {
            self.player_hit_flash -= 1;
            changed = true;
        }

        for (_, timer) in &mut self.enemy_hit_flashes {
            if *timer > 0 {
                *timer -= 1;
                changed = true;
            }
        }
        let before = self.enemy_hit_flashes.len();
        self.enemy_hit_flashes.retain(|(_, timer)| *timer > 0);
        if self.quest_notice_timer > 0 {
            self.quest_notice_timer -= 1;
            changed = true;
        }
        if self.shop_purchase_notice_timer > 0 {
            self.shop_purchase_notice_timer -= 1;
            changed = true;
        }
        changed || before != self.enemy_hit_flashes.len()
    }

    pub fn apply_event(&mut self, state: &GameState, event: &GameEvent) -> bool {
        match event {
            GameEvent::Combat(CombatEvent::SetEntityHitFlash { entity_id, timer }) => {
                let _ = entity_id;
                if self.player_hit_flash == *timer {
                    return false;
                }
                self.player_hit_flash = *timer;
                true
            }
            GameEvent::Combat(CombatEvent::EnemyHitFlashSet {
                entity_id,
                hit_flash,
            }) => {
                if let Some((_, timer)) = self
                    .enemy_hit_flashes
                    .iter_mut()
                    .find(|(id, _)| *id == *entity_id)
                {
                    if *timer == *hit_flash {
                        return false;
                    }
                    *timer = *hit_flash;
                    return true;
                }
                self.enemy_hit_flashes.push((*entity_id, *hit_flash));
                true
            }
            GameEvent::World(WorldEvent::AddQuestProgress(progress))
                if matches!(state, GameState::Explore | GameState::Dialog)
                    && progress.current_count == 0
                    && !progress.completed
                    && !progress.rewarded =>
            {
                let changed = self.quest_notice_timer != QUEST_NOTICE_DURATION;
                self.quest_notice_timer = QUEST_NOTICE_DURATION;
                changed
            }
            GameEvent::ShopBuyItem(_) if matches!(state, GameState::Shop) => {
                let changed = self.shop_purchase_notice_timer != SHOP_PURCHASE_NOTICE_DURATION;
                self.shop_purchase_notice_timer = SHOP_PURCHASE_NOTICE_DURATION;
                changed
            }
            _ => false,
        }
    }

    fn enemy_hit_flash(&self, enemy_id: u32) -> u32 {
        self.enemy_hit_flashes
            .iter()
            .find_map(|(id, timer)| (*id == enemy_id).then_some(*timer))
            .unwrap_or(0)
    }
}

fn as_u32(value: i32) -> u32 {
    value.max(0) as u32
}

fn scroll_for_selection(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let max_scroll = total - visible;
    let desired = if selected + 1 > visible {
        selected - (visible - 1)
    } else {
        0
    };
    desired.min(max_scroll)
}

fn timed_to_status(timed: &TimedState) -> StatusRender {
    StatusRender {
        poison_timer: timed.time_left(TimedKind::Poison),
        stun_timer: timed.time_left(TimedKind::Stun),
        armor_break_timer: timed.time_left(TimedKind::ArmorBreak),
    }
}

fn skill_cooldowns_from_timed(timed: &TimedState) -> [u32; 3] {
    [
        timed.time_left(TimedKind::SkillCooldown(0)),
        timed.time_left(TimedKind::SkillCooldown(1)),
        timed.time_left(TimedKind::SkillCooldown(2)),
    ]
}

fn build_explore_render(
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Option<ExploreRender> {
    let leader_id = world.leader_id()?;
    let leader = world.leader_entity()?;
    let leader_combatant = world.combat.combatant(leader_id)?;
    let map = data.find_map(&leader.map_id)?;

    let mut enemies = Vec::with_capacity(world.combat.enemies.len());
    for enemy in &world.combat.enemies {
        let Some(entity) = world.entity(enemy.entity_id) else {
            continue;
        };
        let name = data
            .find_enemy(&enemy.source_enemy_id)
            .map(|enemy_data| enemy_data.name.clone())
            .unwrap_or_else(|| enemy.source_enemy_id.clone());
        enemies.push(EnemyRender {
            enemy_id: enemy.entity_id,
            name,
            x: entity.x,
            y: entity.y,
            hp: enemy.combatant.stats.current_hp,
            max_hp: enemy.combatant.stats.max_hp,
            attack_cooldown: enemy.combatant.timed.time_left(TimedKind::AttackCooldown),
            hit_flash: render_fx.enemy_hit_flash(enemy.entity_id),
            dead: enemy.combatant.stats.current_hp <= 0,
        });
    }

    let first_live_enemy_name = enemies
        .iter()
        .find(|enemy| enemy.hp > 0)
        .map(|enemy| enemy.name.clone());

    Some(ExploreRender {
        data: Rc::clone(data),
        map_id: leader.map_id.clone(),
        player_x: leader.x,
        player_y: leader.y,
        player_facing: leader.facing,
        player_moving: world.movement.pressed_direction.is_some(),
        hp: as_u32(leader_combatant.stats.current_hp),
        max_hp: as_u32(leader_combatant.stats.max_hp),
        mp: as_u32(leader_combatant.stats.current_mp),
        max_mp: as_u32(leader_combatant.stats.max_mp),
        level: as_u32(leader.stat.level),
        active_quest_count: world
            .quests
            .iter()
            .filter(|quest| !quest.rewarded && !quest.completed)
            .count(),
        tracked_quest: build_tracked_quest_render(
            world,
            data,
            ui.quest_log.tracked_quest_id.as_deref(),
        ),
        interaction_hint: build_interaction_hint(world, data),
        first_live_enemy_name,
        opened_treasures: world.opened_treasures.clone(),
        enemies,
        player_hit_flash: render_fx.player_hit_flash,
        skill_effects: Vec::new(),
        skill_cooldowns: skill_cooldowns_from_timed(&leader_combatant.timed),
        player_status: timed_to_status(&leader_combatant.timed),
        key_actions: ui.explore.key_actions,
        peaceful: map.peaceful,
        quest_notice_timer: render_fx.quest_notice_timer,
        anim_tick: render_fx.anim_tick,
    })
}

fn build_tracked_quest_render(
    world: &WorldState,
    data: &Rc<GameData>,
    tracked_quest_id: Option<&str>,
) -> Option<TrackedQuestRender> {
    let tracked_quest_id = tracked_quest_id?;
    let progress = world
        .quests
        .iter()
        .find(|quest| quest.quest_id == tracked_quest_id && !quest.rewarded)?;
    let quest_data = data.find_quest(&progress.quest_id)?;

    Some(TrackedQuestRender {
        name: quest_data.name.clone(),
        current_count: as_u32(progress.current_count),
        target_count: as_u32(quest_data.target_count),
        completed: progress.completed,
    })
}

fn build_interaction_hint(world: &WorldState, data: &Rc<GameData>) -> Option<String> {
    let leader = world.leader_entity()?;
    let map = data.find_map(&leader.map_id)?;
    let (tx, ty) = leader.facing.apply(leader.x, leader.y);

    if let Some(npc) = data.find_npc_at(&leader.map_id, tx, ty) {
        let text = match npc.npc_type {
            NpcType::ShopKeeper => "OK: Shop",
            NpcType::Healer => "OK: Heal",
            NpcType::QuestGiver | NpcType::Villager => "OK: Talk",
        };
        return Some(String::from(text));
    }

    let text = match map.get_tile(tx, ty) {
        Tile::Treasure => "OK: Open chest",
        Tile::Exit => "Move: Exit",
        Tile::Dungeon => "Move: Enter dungeon",
        _ => return None,
    };
    Some(String::from(text))
}

fn build_inventory_render(
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
) -> InventoryRender {
    let Some(leader) = world.leader_entity() else {
        return InventoryRender {
            items: Vec::new(),
            equipped_weapon: None,
            equipped_armor: None,
            equipped_accessory: None,
            selected: 0,
            scroll: 0,
        };
    };

    let mut items = Vec::with_capacity(leader.inventory.len());
    for stack in &leader.inventory {
        let item = data.find_item(&stack.item_id);
        let name = if let Some(item) = item {
            if stack.amount > 1 {
                format!("{} x{}", item.name, stack.amount)
            } else {
                item.name.clone()
            }
        } else if stack.amount > 1 {
            format!("{} x{}", stack.item_id, stack.amount)
        } else {
            stack.item_id.clone()
        };
        let kind = item.map(|item| item.kind).unwrap_or(ItemKind::Consumable);
        items.push(InventoryItemRender { name, kind });
    }

    InventoryRender {
        items,
        equipped_weapon: leader.loadout.weapon,
        equipped_armor: leader.loadout.armor,
        equipped_accessory: leader.loadout.accessory,
        selected: ui.inventory.selected,
        scroll: scroll_for_selection(
            ui.inventory.selected,
            leader.inventory.len(),
            INVENTORY_VISIBLE_ITEMS,
        ),
    }
}

fn build_stats_render(world: &WorldState) -> Option<StatsRender> {
    let leader_id = world.leader_id()?;
    let leader = world.leader_entity()?;
    let combatant = world.combat.combatant(leader_id)?;

    Some(StatsRender {
        hp: as_u32(combatant.stats.current_hp),
        max_hp: as_u32(combatant.stats.max_hp),
        mp: as_u32(combatant.stats.current_mp),
        max_mp: as_u32(combatant.stats.max_mp),
        level: as_u32(leader.stat.level),
        atk: as_u32(combatant.stats.atk),
        def: as_u32(combatant.stats.def),
        exp: as_u32(leader.stat.exp),
        gold: as_u32(world.gold_amount(leader_id)),
    })
}

fn build_shop_render(
    world: &WorldState,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Option<ShopRender> {
    let leader_id = world.leader_id()?;
    let leader = world.leader_entity()?;
    let shop_state = ui.shop.state.as_ref()?;

    let mut buy_items = Vec::with_capacity(shop_state.items.len());
    for item in &shop_state.items {
        buy_items.push(ShopItemRender {
            name: item.name.clone(),
            price: item.price,
        });
    }
    let mut player_inventory = Vec::new();
    for stack in &leader.inventory {
        if stack.item_id == crate::game::GOLD_ITEM_ID {
            continue;
        }
        let name = data
            .find_item(&stack.item_id)
            .map(|item| item.name.clone())
            .unwrap_or_else(|| stack.item_id.clone());
        let sell_price = data
            .find_item(&stack.item_id)
            .map(|item| item.price / 2)
            .unwrap_or(0);
        for _ in 0..stack.amount.max(0) {
            player_inventory.push(ShopItemRender {
                name: name.clone(),
                price: sell_price,
            });
        }
    }
    let total = match ui.shop.mode {
        ShopMode::Select => 2,
        ShopMode::Buy | ShopMode::ConfirmBuy => buy_items.len(),
        ShopMode::Sell | ShopMode::ConfirmSell => player_inventory.len(),
    };

    Some(ShopRender {
        shop_name: shop_state.shop.name.clone(),
        mode: ui.shop.mode,
        selected: ui.shop.selected,
        scroll: scroll_for_selection(ui.shop.selected, total, SHOP_VISIBLE_ITEMS),
        buy_items,
        player_gold: world.gold_amount(leader_id),
        player_inventory,
        purchase_notice_timer: render_fx.shop_purchase_notice_timer,
    })
}

fn build_quest_log_render(world: &WorldState, ui: &UiState, data: &Rc<GameData>) -> QuestLogRender {
    let mut quests = Vec::with_capacity(world.quests.len());
    for quest in &world.quests {
        if quest.rewarded {
            continue;
        }
        if let Some(quest_data) = data.find_quest(&quest.quest_id) {
            quests.push(QuestEntryRender {
                quest_id: quest.quest_id.clone(),
                name: quest_data.name.clone(),
                description: quest_data.description.clone(),
                current_count: as_u32(quest.current_count),
                target_count: as_u32(quest_data.target_count),
                completed: quest.completed,
            });
        }
    }

    QuestLogRender {
        quests,
        selected: ui.quest_log.selected,
        tracked_quest_id: ui.quest_log.tracked_quest_id.clone(),
    }
}

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
                match world.and_then(|world| build_explore_render(world, ui, data, render_fx)) {
                    Some(explore) => RenderState::Explore(explore),
                    None => RenderState::NoSession,
                }
            }
            GameState::Inventory => match world {
                Some(world) => RenderState::Inventory(build_inventory_render(world, ui, data)),
                None => RenderState::NoSession,
            },
            GameState::Stats => match world.and_then(build_stats_render) {
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
                        explore: world
                            .and_then(|world| build_explore_render(world, ui, data, render_fx)),
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
                match world.and_then(|world| build_shop_render(world, ui, data, render_fx)) {
                    Some(shop) => RenderState::Shop(shop),
                    None => RenderState::NoSession,
                }
            }
            GameState::QuestLog => match world {
                Some(world) => RenderState::QuestLog(build_quest_log_render(world, ui, data)),
                None => RenderState::NoSession,
            },
            GameState::PauseMenu => RenderState::PauseMenu {
                explore: world.and_then(|world| build_explore_render(world, ui, data, render_fx)),
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
                    GameEvent::World(
                        WorldEvent::SetEntityInventory { .. }
                            | WorldEvent::SetEntityLoadout { .. }
                            | WorldEvent::AddEntityItem { .. }
                            | WorldEvent::RemoveEntityItem { .. }
                            | WorldEvent::RemoveEntity(_)
                            | WorldEvent::UpsertEntity(_)
                    ) | GameEvent::ShopSellSelected(_)
                        | GameEvent::ShopBuyItem(_)
                        | GameEvent::UseInventorySelected(_)
                ) {
                    if let Some(world) = world {
                        *inventory = build_inventory_render(world, ui, data);
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
                    GameEvent::World(
                        WorldEvent::SetEntityInventory { .. }
                            | WorldEvent::SetEntityLoadout { .. }
                            | WorldEvent::AddEntityItem { .. }
                            | WorldEvent::RemoveEntityItem { .. }
                            | WorldEvent::RemoveEntity(_)
                            | WorldEvent::UpsertEntity(_)
                    ) | GameEvent::ShopBuyItem(_)
                        | GameEvent::ShopSellSelected(_)
                        | GameEvent::OpenShopState(_)
                ) {
                    if let Some(world) = world
                        && let Some(next_shop) = build_shop_render(world, ui, data, render_fx)
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
                if matches!(event, GameEvent::World(WorldEvent::AddQuestProgress(_))) {
                    if let Some(world) = world {
                        *quest_log = build_quest_log_render(world, ui, data);
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
                explore.player_hit_flash = render_fx.player_hit_flash;
                for enemy in &mut explore.enemies {
                    enemy.hit_flash = render_fx.enemy_hit_flash(enemy.enemy_id);
                }
                explore.quest_notice_timer = render_fx.quest_notice_timer;
                explore.anim_tick = render_fx.anim_tick;
            }
            RenderState::Shop(shop) => {
                shop.purchase_notice_timer = render_fx.shop_purchase_notice_timer;
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

    stats.hp = as_u32(combatant.stats.current_hp);
    stats.max_hp = as_u32(combatant.stats.max_hp);
    stats.mp = as_u32(combatant.stats.current_mp);
    stats.max_mp = as_u32(combatant.stats.max_mp);
    stats.level = as_u32(leader.stat.level);
    stats.atk = as_u32(combatant.stats.atk);
    stats.def = as_u32(combatant.stats.def);
    stats.exp = as_u32(leader.stat.exp);
    stats.gold = as_u32(world.gold_amount(leader_id));
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
    explore.hp = as_u32(leader_combatant.stats.current_hp);
    explore.max_hp = as_u32(leader_combatant.stats.max_hp);
    explore.mp = as_u32(leader_combatant.stats.current_mp);
    explore.max_mp = as_u32(leader_combatant.stats.max_mp);
    explore.level = as_u32(leader.stat.level);
    explore.player_status = timed_to_status(&leader_combatant.timed);
    explore.skill_cooldowns = skill_cooldowns_from_timed(&leader_combatant.timed);
    explore.key_actions = ui.explore.key_actions;
    explore.player_hit_flash = render_fx.player_hit_flash;
    explore.quest_notice_timer = render_fx.quest_notice_timer;
    explore.anim_tick = render_fx.anim_tick;
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
            if let Some(next) = build_explore_render(world, ui, data, render_fx) {
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
            CombatEvent::SetCombatantStats { entity_id, .. } => {
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
            CombatEvent::UpsertEnemy(enemy_state) => {
                upsert_enemy_render(explore, world, data, render_fx, enemy_state.entity_id);
                enemy_changed = true;
            }
            CombatEvent::RemoveEnemy(entity_id) => {
                let before = explore.enemies.len();
                explore.enemies.retain(|enemy| enemy.enemy_id != *entity_id);
                enemy_changed = before != explore.enemies.len();
            }
            CombatEvent::SetEnemies(_) => {
                if let Some(next) = build_explore_render(world, ui, data, render_fx) {
                    *explore = next;
                    return true;
                }
                return false;
            }
            CombatEvent::EnemyHitFlashSet {
                entity_id,
                hit_flash,
            } => {
                if let Some(enemy) = explore
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.enemy_id == *entity_id)
                {
                    enemy.hit_flash = *hit_flash;
                }
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
            WorldEvent::AddQuestProgress(_) => {
                explore.active_quest_count = world
                    .quests
                    .iter()
                    .filter(|quest| !quest.rewarded && !quest.completed)
                    .count();
                explore.tracked_quest = build_tracked_quest_render(
                    world,
                    data,
                    ui.quest_log.tracked_quest_id.as_deref(),
                );
            }
            WorldEvent::SetEntityPosition { entity_id, .. }
            | WorldEvent::SetEntityFacing { entity_id, .. } => {
                if Some(*entity_id) == leader_id {
                    requires_hint_refresh = true;
                } else {
                    upsert_enemy_render(explore, world, data, render_fx, *entity_id);
                    enemy_changed = true;
                }
            }
            WorldEvent::SetEntityMap { .. } | WorldEvent::SetWorldMap(_) => {
                if let Some(next) = build_explore_render(world, ui, data, render_fx) {
                    *explore = next;
                    return true;
                }
                return false;
            }
            WorldEvent::SetEntityStat { entity_id, .. } => {
                if Some(*entity_id) == leader_id
                    && let Some(leader) = world.leader_entity()
                {
                    explore.level = as_u32(leader.stat.level);
                }
            }
            WorldEvent::RemoveEntity(entity_id) => {
                let before = explore.enemies.len();
                explore.enemies.retain(|enemy| enemy.enemy_id != *entity_id);
                enemy_changed = before != explore.enemies.len();
            }
            WorldEvent::ResetCombat => {
                explore.enemies.clear();
                enemy_changed = true;
            }
            WorldEvent::UpsertEntity(_)
            | WorldEvent::SetEntityInventory { .. }
            | WorldEvent::SetEntityLoadout { .. }
            | WorldEvent::AddEntityItem { .. }
            | WorldEvent::RemoveEntityItem { .. }
            | WorldEvent::SetParty(_)
            | WorldEvent::CreateWorld
            | WorldEvent::ResetMovement => {}
        },
        _ => {}
    }

    if requires_hint_refresh {
        explore.interaction_hint = build_interaction_hint(world, data);
    }
    if enemy_changed {
        refresh_first_live_enemy_name(explore);
    }
    true
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
