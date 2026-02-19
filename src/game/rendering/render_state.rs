use alloc::{collections::BTreeMap, format, rc::Rc, string::String, vec::Vec};

use anyhow::{Result, anyhow};

use super::render_fx::RenderFxState;
use crate::{
    data::{Direction, ItemKind, NpcType, SkillType, Tile},
    game::{
        game_data::GameData,
        state::{GOLD_ITEM_ID, TimedKind, TimedState, combat_attack_def},
        ui::state::{
            ExploreAction, INVENTORY_VISIBLE_ITEMS, MenuAction, SHOP_VISIBLE_ITEMS, ShopMode,
            UiState,
        },
        world::WorldState,
    },
};

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
}

#[derive(Default)]
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
    pub enemy_indices: BTreeMap<u32, usize>,
    pub player_hit_flash: u32,
    pub skill_effects: Vec<SkillEffectRender>,
    pub skill_cooldowns: [u32; 3],
    pub player_status: StatusRender,
    pub key_actions: [Option<ExploreAction>; 3],
    pub peaceful: bool,
    pub quest_notice_timer: u32,
    pub anim_tick: u32,
}

#[derive(PartialEq, Eq)]
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

#[derive(PartialEq, Eq)]
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

pub fn scroll_for_selection(selected: usize, total: usize, visible: usize) -> usize {
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
    pub fn from_timed(timed: &TimedState, current_tick: u32) -> Self {
        Self {
            poison_timer: timed.time_left(TimedKind::Poison, current_tick),
            stun_timer: timed.time_left(TimedKind::Stun, current_tick),
            armor_break_timer: timed.time_left(TimedKind::ArmorBreak, current_tick),
        }
    }
}

pub fn skill_cooldowns_from_timed(timed: &TimedState, current_tick: u32) -> [u32; 3] {
    [
        timed.time_left(TimedKind::SkillCooldown(0), current_tick),
        timed.time_left(TimedKind::SkillCooldown(1), current_tick),
        timed.time_left(TimedKind::SkillCooldown(2), current_tick),
    ]
}

pub fn interaction_hint_from_world(
    world: &WorldState,
    data: &Rc<GameData>,
) -> Result<Option<String>> {
    let leader = world.leader_entity()?;
    let map = data.find_map(&leader.map_id)?;
    let (tx, ty) = leader.facing.apply(leader.x, leader.y);

    if let Some(npc) = data.find_npc_at(&leader.map_id, tx, ty) {
        let text = match npc.npc_type {
            NpcType::ShopKeeper => "OK: Shop",
            NpcType::Healer => "OK: Heal",
            NpcType::QuestGiver | NpcType::Villager => "OK: Talk",
        };
        return Ok(Some(String::from(text)));
    }

    let text = match map.get_tile(tx, ty) {
        Tile::Treasure => "OK: Open chest",
        Tile::Exit => "Move: Exit",
        Tile::Dungeon => "Move: Enter dungeon",
        _ => return Ok(None),
    };
    Ok(Some(String::from(text)))
}

fn item_name_or_id(item_id: &str, item_name: Option<&str>) -> String {
    if let Some(item_name) = item_name {
        String::from(item_name)
    } else {
        String::from(item_id)
    }
}

fn stacked_item_label(item_id: &str, amount: i32, item_name: Option<&str>) -> String {
    let name = item_name_or_id(item_id, item_name);
    if amount > 1 {
        format!("{} x{}", name, amount)
    } else {
        name
    }
}

impl ExploreRender {
    pub fn from_world(
        world: &WorldState,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Result<Self> {
        let leader_id = world.leader_id()?;
        let leader = world.leader_entity()?;
        let leader_combatant = world.combat.combatant(leader_id)?;
        let map = data.find_map(&leader.map_id)?;

        let mut enemies = Vec::with_capacity(world.combat.enemies.len());
        let mut enemy_indices = BTreeMap::new();
        for enemy in &world.combat.enemies {
            let entity = world.entity(enemy.entity_id)?;
            let name = data.find_enemy(&enemy.source_enemy_id)?.name.clone();
            let enemy_index = enemies.len();
            enemies.push(EnemyRender {
                enemy_id: enemy.entity_id,
                name,
                x: entity.x,
                y: entity.y,
                hp: entity.current_hp,
                max_hp: entity.stat.base_max_hp,
                attack_cooldown: enemy
                    .combatant
                    .timed
                    .time_left(TimedKind::AttackCooldown, world.tick_counter),
                hit_flash: render_fx.enemy_hit_flash(enemy.entity_id),
                dead: entity.current_hp <= 0,
            });
            enemy_indices.insert(enemy.entity_id, enemy_index);
        }

        let first_live_enemy_name = enemies
            .iter()
            .find(|enemy| enemy.hp > 0)
            .map(|enemy| enemy.name.clone());

        Ok(Self {
            data: Rc::clone(data),
            map_id: leader.map_id.clone(),
            player_x: leader.x,
            player_y: leader.y,
            player_facing: leader.facing,
            player_moving: world.movement.is_moving(),
            hp: leader.current_hp as u32,
            max_hp: leader.stat.base_max_hp as u32,
            mp: leader.current_mp as u32,
            max_mp: leader.stat.base_max_mp as u32,
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
            )?,
            interaction_hint: interaction_hint_from_world(world, data)?,
            first_live_enemy_name,
            opened_treasures: world.opened_treasures.clone(),
            enemies,
            enemy_indices,
            player_hit_flash: render_fx.player_hit_flash(),
            skill_effects: render_fx
                .skill_effect_iter()
                .map(|(x, y, effect_type)| SkillEffectRender { x, y, effect_type })
                .collect(),
            skill_cooldowns: skill_cooldowns_from_timed(
                &leader_combatant.timed,
                world.tick_counter,
            ),
            player_status: StatusRender::from_timed(&leader_combatant.timed, world.tick_counter),
            key_actions: ui.explore.key_actions,
            peaceful: map.peaceful,
            quest_notice_timer: render_fx.quest_notice_timer(),
            anim_tick: render_fx.anim_tick(),
        })
    }

    pub fn enemy_mut(&mut self, enemy_id: u32) -> Option<&mut EnemyRender> {
        let index = self.enemy_indices.get(&enemy_id).copied()?;
        self.enemies.get_mut(index)
    }

    pub fn remove_enemy(&mut self, enemy_id: u32) -> bool {
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

    pub fn upsert_enemy(&mut self, enemy: EnemyRender) {
        if let Some(existing) = self.enemy_mut(enemy.enemy_id) {
            *existing = enemy;
            return;
        }
        let index = self.enemies.len();
        self.enemy_indices.insert(enemy.enemy_id, index);
        self.enemies.push(enemy);
    }

    pub fn rebuild_enemy_indices(&mut self) {
        self.enemy_indices.clear();
        for (index, enemy) in self.enemies.iter().enumerate() {
            self.enemy_indices.insert(enemy.enemy_id, index);
        }
    }

    pub fn first_live_enemy_name(&self) -> Option<String> {
        self.enemies
            .iter()
            .find(|enemy| enemy.hp > 0)
            .map(|enemy| enemy.name.clone())
    }
}

impl TrackedQuestRender {
    pub fn from_world(
        world: &WorldState,
        data: &Rc<GameData>,
        tracked_quest_id: Option<&str>,
    ) -> Result<Option<Self>> {
        let Some(tracked_quest_id) = tracked_quest_id else {
            return Ok(None);
        };
        let Some(progress) = world
            .quests
            .iter()
            .find(|quest| quest.quest_id == tracked_quest_id && !quest.rewarded)
        else {
            return Ok(None);
        };
        let quest_data = data.find_quest(&progress.quest_id)?;

        Ok(Some(Self {
            name: quest_data.name.clone(),
            current_count: progress.current_count as u32,
            target_count: quest_data.target_count as u32,
            completed: progress.completed,
        }))
    }
}

impl InventoryRender {
    pub fn from_world(world: &WorldState, ui: &UiState, data: &Rc<GameData>) -> Result<Self> {
        let leader = world.leader_entity()?;

        let mut items = Vec::with_capacity(leader.inventory.len());
        for stack in &leader.inventory {
            let item = data.find_item(&stack.item_id)?;
            let name = stacked_item_label(&stack.item_id, stack.amount, Some(item.name.as_str()));
            let kind = item.kind;
            items.push(InventoryItemRender { name, kind });
        }

        Ok(Self {
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
}

impl StatsRender {
    pub fn from_world(world: &WorldState, data: &Rc<GameData>) -> Result<Self> {
        let leader_id = world.leader_id()?;
        let leader = world.leader_entity()?;
        let (atk, def) = combat_attack_def(data, leader)?;

        Ok(Self {
            hp: leader.current_hp as u32,
            max_hp: leader.stat.base_max_hp as u32,
            mp: leader.current_mp as u32,
            max_mp: leader.stat.base_max_mp as u32,
            level: leader.stat.level as u32,
            atk: atk as u32,
            def: def as u32,
            exp: leader.stat.exp as u32,
            gold: world.gold_amount(leader_id)? as u32,
        })
    }
}

impl ShopRender {
    pub fn from_world(
        world: &WorldState,
        ui: &UiState,
        data: &Rc<GameData>,
        render_fx: &RenderFxState,
    ) -> Result<Self> {
        let leader_id = world.leader_id()?;
        let leader = world.leader_entity()?;
        let shop_id = ui
            .shop
            .shop_id
            .as_deref()
            .ok_or_else(|| anyhow!("No shop state"))?;
        let shop = data.find_shop(shop_id)?;

        let buy_items = shop
            .items
            .iter()
            .map(|item_id| {
                let item = data.find_item(item_id)?;
                Ok(ShopItemRender {
                    name: item.name.clone(),
                    price: item.price,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut player_inventory = Vec::new();
        for stack in &leader.inventory {
            if stack.item_id == GOLD_ITEM_ID {
                continue;
            }
            let item = data.find_item(&stack.item_id)?;
            let name = item.name.clone();
            let sell_price = item.price / 2;
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

        Ok(Self {
            shop_name: shop.name.clone(),
            mode: ui.shop.mode,
            selected: ui.shop.selected,
            scroll: scroll_for_selection(ui.shop.selected, total, SHOP_VISIBLE_ITEMS),
            buy_items,
            player_gold: world.gold_amount(leader_id)?,
            player_inventory,
            purchase_notice_timer: render_fx.shop_purchase_notice_timer(),
        })
    }
}

impl QuestLogRender {
    pub fn from_world(world: &WorldState, ui: &UiState, data: &Rc<GameData>) -> Result<Self> {
        let mut quests = Vec::with_capacity(world.quests.len());
        for quest in &world.quests {
            if quest.rewarded {
                continue;
            }
            let quest_data = data.find_quest(&quest.quest_id)?;
            quests.push(QuestEntryRender {
                quest_id: quest.quest_id.clone(),
                name: quest_data.name.clone(),
                description: quest_data.description.clone(),
                current_count: quest.current_count as u32,
                target_count: quest_data.target_count as u32,
                completed: quest.completed,
            });
        }

        Ok(Self {
            quests,
            selected: ui.quest_log.selected,
            tracked_quest_id: ui.quest_log.tracked_quest_id.clone(),
        })
    }
}
