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

fn render_state_from_game_state(
    state: &GameState,
    world: Option<&WorldState>,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> RenderState {
    match state {
        GameState::Loading(step) => RenderState::Loading { step: *step },
        GameState::Menu => RenderState::Menu {
            title: ui.menu.state.title,
            items: ui.menu.state.items.clone(),
            selected: ui.menu.selected,
        },
        GameState::Explore => {
            let Some(world) = world else {
                return RenderState::NoSession;
            };
            let Some(explore) = build_explore_render(world, ui, data, render_fx) else {
                return RenderState::Error(String::from("Map not found"));
            };
            RenderState::Explore(explore)
        }
        GameState::Inventory => {
            let Some(world) = world else {
                return RenderState::NoSession;
            };
            let Some(leader) = world.leader_entity() else {
                return RenderState::NoSession;
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
            RenderState::Inventory(InventoryRender {
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
            })
        }
        GameState::Stats => {
            let Some(world) = world else {
                return RenderState::NoSession;
            };
            let Some(leader_id) = world.leader_id() else {
                return RenderState::NoSession;
            };
            let Some(leader) = world.leader_entity() else {
                return RenderState::NoSession;
            };
            let Some(combatant) = world.combat.combatant(leader_id) else {
                return RenderState::NoSession;
            };
            RenderState::Stats(StatsRender {
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
        GameState::Dialog => {
            let Some(dialog_state) = ui.dialog.state.as_ref() else {
                return RenderState::Error(String::from("No dialog state"));
            };
            let current_text = dialog_state
                .lines
                .get(dialog_state.current_line)
                .map(|line| line.text.clone());
            RenderState::Dialog {
                explore: world.and_then(|world| build_explore_render(world, ui, data, render_fx)),
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
        }
        GameState::Shop => {
            let Some(world) = world else {
                return RenderState::NoSession;
            };
            let Some(leader_id) = world.leader_id() else {
                return RenderState::NoSession;
            };
            let Some(leader) = world.leader_entity() else {
                return RenderState::NoSession;
            };
            let Some(shop_state) = ui.shop.state.as_ref() else {
                return RenderState::Error(String::from("No shop state"));
            };
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
            RenderState::Shop(ShopRender {
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
        GameState::QuestLog => {
            let Some(world) = world else {
                return RenderState::NoSession;
            };
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
            RenderState::QuestLog(QuestLogRender {
                quests,
                selected: ui.quest_log.selected,
                tracked_quest_id: ui.quest_log.tracked_quest_id.clone(),
            })
        }
        GameState::PauseMenu => RenderState::PauseMenu {
            explore: world.and_then(|world| build_explore_render(world, ui, data, render_fx)),
            items: ui.pause_menu.state.items.clone(),
            selected: ui.pause_menu.selected,
        },
        GameState::Dead => RenderState::Dead,
        GameState::Error(msg) => RenderState::Error(msg.clone()),
    }
}

impl RenderState {
    pub fn rebuild(
        &mut self,
        state: &GameState,
        world: Option<&WorldState>,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) {
        *self = render_state_from_game_state(state, world, ui, data, render_fx);
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
