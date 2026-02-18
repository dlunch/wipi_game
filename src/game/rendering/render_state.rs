use alloc::collections::BTreeMap;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::data::{Direction, ItemKind, NpcType, SkillType, Tile};
use crate::game::state::{TimedKind, TimedState};
use crate::game::ui::{
    ExploreAction, INVENTORY_VISIBLE_ITEMS, MenuAction, SHOP_VISIBLE_ITEMS, ShopMode, UiState,
};
use crate::game::{GOLD_ITEM_ID, GameData, WorldState};

use super::render_fx::RenderFxState;

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
    pub(super) enemy_indices: BTreeMap<u32, usize>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
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

pub(super) fn scroll_for_selection(selected: usize, total: usize, visible: usize) -> usize {
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

impl StatusRender {
    pub(super) fn from_timed(timed: &TimedState) -> Self {
        Self {
            poison_timer: timed.time_left(TimedKind::Poison),
            stun_timer: timed.time_left(TimedKind::Stun),
            armor_break_timer: timed.time_left(TimedKind::ArmorBreak),
        }
    }
}

pub(super) fn skill_cooldowns_from_timed(timed: &TimedState) -> [u32; 3] {
    [
        timed.time_left(TimedKind::SkillCooldown(0)),
        timed.time_left(TimedKind::SkillCooldown(1)),
        timed.time_left(TimedKind::SkillCooldown(2)),
    ]
}

pub(super) fn interaction_hint_from_world(
    world: &WorldState,
    data: &Rc<GameData>,
) -> Option<String> {
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

impl ExploreRender {
    pub(super) fn from_world(
        world: &WorldState,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Option<Self> {
        let leader_id = world.leader_id()?;
        let leader = world.leader_entity()?;
        let leader_combatant = world.combat.combatant(leader_id)?;
        let map = data.find_map(&leader.map_id)?;

        let mut enemies = Vec::with_capacity(world.combat.enemies.len());
        let mut enemy_indices = BTreeMap::new();
        for enemy in &world.combat.enemies {
            let Some(entity) = world.entity(enemy.entity_id) else {
                continue;
            };
            let name = data
                .find_enemy(&enemy.source_enemy_id)
                .map(|enemy_data| enemy_data.name.clone())
                .unwrap_or_else(|| enemy.source_enemy_id.clone());
            let enemy_index = enemies.len();
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
            enemy_indices.insert(enemy.entity_id, enemy_index);
        }

        let first_live_enemy_name = enemies
            .iter()
            .find(|enemy| enemy.hp > 0)
            .map(|enemy| enemy.name.clone());

        Some(Self {
            data: Rc::clone(data),
            map_id: leader.map_id.clone(),
            player_x: leader.x,
            player_y: leader.y,
            player_facing: leader.facing,
            player_moving: world.movement.pressed_direction.is_some(),
            hp: leader_combatant.stats.current_hp as u32,
            max_hp: leader_combatant.stats.max_hp as u32,
            mp: leader_combatant.stats.current_mp as u32,
            max_mp: leader_combatant.stats.max_mp as u32,
            level: leader.stat.level as u32,
            active_quest_count: world
                .quests
                .iter()
                .filter(|quest| !quest.rewarded && !quest.completed)
                .count(),
            tracked_quest: TrackedQuestRender::from_world(
                world,
                data,
                ui.quest_log.tracked_quest_id.as_deref(),
            ),
            interaction_hint: interaction_hint_from_world(world, data),
            first_live_enemy_name,
            opened_treasures: world.opened_treasures.clone(),
            enemies,
            enemy_indices,
            player_hit_flash: render_fx.player_hit_flash(),
            skill_effects: render_fx
                .skill_effect_iter()
                .map(|(x, y, effect_type)| SkillEffectRender { x, y, effect_type })
                .collect(),
            skill_cooldowns: skill_cooldowns_from_timed(&leader_combatant.timed),
            player_status: StatusRender::from_timed(&leader_combatant.timed),
            key_actions: ui.explore.key_actions,
            peaceful: map.peaceful,
            quest_notice_timer: render_fx.quest_notice_timer(),
            anim_tick: render_fx.anim_tick(),
        })
    }

    pub(super) fn enemy_mut(&mut self, enemy_id: u32) -> Option<&mut EnemyRender> {
        let index = self.enemy_indices.get(&enemy_id).copied()?;
        self.enemies.get_mut(index)
    }

    pub(super) fn remove_enemy(&mut self, enemy_id: u32) -> bool {
        let Some(index) = self.enemy_indices.remove(&enemy_id) else {
            return false;
        };
        if index >= self.enemies.len() {
            return false;
        }
        self.enemies.remove(index);
        self.rebuild_enemy_indices();
        true
    }

    pub(super) fn upsert_enemy(&mut self, enemy: EnemyRender) {
        if let Some(existing) = self.enemy_mut(enemy.enemy_id) {
            *existing = enemy;
            return;
        }
        let index = self.enemies.len();
        self.enemy_indices.insert(enemy.enemy_id, index);
        self.enemies.push(enemy);
    }

    pub(super) fn rebuild_enemy_indices(&mut self) {
        self.enemy_indices.clear();
        for (index, enemy) in self.enemies.iter().enumerate() {
            self.enemy_indices.insert(enemy.enemy_id, index);
        }
    }

    pub(super) fn first_live_enemy_name(&self) -> Option<String> {
        self.enemies
            .iter()
            .find(|enemy| enemy.hp > 0)
            .map(|enemy| enemy.name.clone())
    }
}

impl TrackedQuestRender {
    pub(super) fn from_world(
        world: &WorldState,
        data: &Rc<GameData>,
        tracked_quest_id: Option<&str>,
    ) -> Option<Self> {
        let tracked_quest_id = tracked_quest_id?;
        let progress = world
            .quests
            .iter()
            .find(|quest| quest.quest_id == tracked_quest_id && !quest.rewarded)?;
        let quest_data = data.find_quest(&progress.quest_id)?;

        Some(Self {
            name: quest_data.name.clone(),
            current_count: progress.current_count as u32,
            target_count: quest_data.target_count as u32,
            completed: progress.completed,
        })
    }
}

impl InventoryRender {
    pub(super) fn from_world(world: &WorldState, ui: &UiState, data: &Rc<GameData>) -> Self {
        let Some(leader) = world.leader_entity() else {
            return Self {
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

        Self {
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
}

impl StatsRender {
    pub(super) fn from_world(world: &WorldState) -> Option<Self> {
        let leader_id = world.leader_id()?;
        let leader = world.leader_entity()?;
        let combatant = world.combat.combatant(leader_id)?;

        Some(Self {
            hp: combatant.stats.current_hp as u32,
            max_hp: combatant.stats.max_hp as u32,
            mp: combatant.stats.current_mp as u32,
            max_mp: combatant.stats.max_mp as u32,
            level: leader.stat.level as u32,
            atk: combatant.stats.atk as u32,
            def: combatant.stats.def as u32,
            exp: leader.stat.exp as u32,
            gold: world.gold_amount(leader_id) as u32,
        })
    }
}

impl ShopRender {
    pub(super) fn from_world(
        world: &WorldState,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Option<Self> {
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
            if stack.item_id == GOLD_ITEM_ID {
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

        Some(Self {
            shop_name: shop_state.shop.name.clone(),
            mode: ui.shop.mode,
            selected: ui.shop.selected,
            scroll: scroll_for_selection(ui.shop.selected, total, SHOP_VISIBLE_ITEMS),
            buy_items,
            player_gold: world.gold_amount(leader_id),
            player_inventory,
            purchase_notice_timer: render_fx.shop_purchase_notice_timer(),
        })
    }
}

impl QuestLogRender {
    pub(super) fn from_world(world: &WorldState, ui: &UiState, data: &Rc<GameData>) -> Self {
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
                    current_count: quest.current_count as u32,
                    target_count: quest_data.target_count as u32,
                    completed: quest.completed,
                });
            }
        }

        Self {
            quests,
            selected: ui.quest_log.selected,
            tracked_quest_id: ui.quest_log.tracked_quest_id.clone(),
        }
    }
}
