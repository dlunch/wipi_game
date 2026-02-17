use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use wipi::framebuffer::Framebuffer;

use crate::data::{Direction, Item, ItemKind, SkillType};
use crate::game::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, CombatEvent, ExploreAction,
    GameData, GameEvent, GameState, INVENTORY_VISIBLE_ITEMS, MenuAction, MovementEvent,
    SHOP_VISIBLE_ITEMS, SessionEvent, SessionState, ShopMode, UiState, clear_screen, draw_dialog,
    draw_explore, draw_inventory, draw_menu, draw_pause_menu, draw_quest_log, draw_rect, draw_shop,
    draw_stats, draw_text, fill_rect,
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
    GameOver,
    Error(String),
    NoSession,
}

pub struct ExploreRender {
    pub data: Rc<GameData>,
    pub map_id: String,
    pub player_x: usize,
    pub player_y: usize,
    pub player_facing: Direction,
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub level: u32,
    pub active_quest_count: usize,
    pub first_live_enemy_name: Option<String>,
    pub opened_treasures: Vec<(String, usize, usize)>,
    pub enemies: Vec<EnemyRender>,
    pub player_hit_flash: u32,
    pub skill_effects: Vec<SkillEffectRender>,
    pub skill_cooldowns: [u32; 3],
    pub key_actions: [Option<ExploreAction>; 3],
    pub peaceful: bool,
}

pub struct EnemyRender {
    pub enemy_id: u32,
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub hit_flash: u32,
    pub dead: bool,
}

pub struct SkillEffectRender {
    pub x: usize,
    pub y: usize,
    pub effect_type: SkillType,
    pub timer: u32,
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
}

pub struct ShopItemRender {
    pub name: String,
    pub price: i32,
}

pub struct QuestLogRender {
    pub quests: Vec<QuestEntryRender>,
}

pub struct QuestEntryRender {
    pub quest_id: String,
    pub name: String,
    pub description: String,
    pub current_count: u32,
    pub target_count: u32,
    pub completed: bool,
}

#[derive(Default)]
pub struct RenderFxState {
    player_hit_flash: u32,
    enemy_hit_flashes: Vec<(u32, u32)>,
}

impl RenderFxState {
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
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
        changed || before != self.enemy_hit_flashes.len()
    }

    pub fn apply_event(&mut self, event: &GameEvent) -> bool {
        let GameEvent::Combat(combat) = event else {
            return false;
        };
        match combat {
            CombatEvent::SetPlayerHitFlash(timer) => {
                if self.player_hit_flash == *timer {
                    return false;
                }
                self.player_hit_flash = *timer;
                true
            }
            CombatEvent::EnemyHitFlashSet {
                enemy_id,
                hit_flash,
            } => {
                if *hit_flash == 0 {
                    let before = self.enemy_hit_flashes.len();
                    self.enemy_hit_flashes.retain(|(id, _)| *id != *enemy_id);
                    return before != self.enemy_hit_flashes.len();
                }

                if let Some((_, timer)) = self
                    .enemy_hit_flashes
                    .iter_mut()
                    .find(|(id, _)| *id == *enemy_id)
                {
                    if *timer == *hit_flash {
                        return false;
                    }
                    *timer = *hit_flash;
                } else {
                    self.enemy_hit_flashes.push((*enemy_id, *hit_flash));
                }
                true
            }
            CombatEvent::EnemyDespawn(enemy_id) => {
                let before = self.enemy_hit_flashes.len();
                self.enemy_hit_flashes.retain(|(id, _)| *id != *enemy_id);
                before != self.enemy_hit_flashes.len()
            }
            CombatEvent::SetMapEnemies { .. } => {
                if self.enemy_hit_flashes.is_empty() && self.player_hit_flash == 0 {
                    return false;
                }
                self.enemy_hit_flashes.clear();
                self.player_hit_flash = 0;
                true
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

fn build_explore_render(
    session: &SessionState,
    ui: &UiState,
    data: &Rc<GameData>,
    render_fx: &RenderFxState,
) -> Option<ExploreRender> {
    let _party_size = 1 + session.companions.len();

    let map = data.find_map(&session.leader.current_map_id)?;

    let first_live_enemy_name = session
        .combat
        .enemies
        .iter()
        .find(|enemy| enemy.hp > 0)
        .map(|enemy| enemy.data.name.clone());

    let mut enemies = Vec::with_capacity(session.combat.enemies.len());
    for enemy in &session.combat.enemies {
        enemies.push(EnemyRender {
            enemy_id: enemy.instance_id,
            name: enemy.data.name.clone(),
            x: enemy.x,
            y: enemy.y,
            hp: enemy.hp,
            max_hp: enemy.data.hp,
            hit_flash: render_fx.enemy_hit_flash(enemy.instance_id),
            dead: enemy.hp <= 0,
        });
    }

    let mut skill_effects = Vec::with_capacity(session.combat.skill_effects.len());
    for effect in &session.combat.skill_effects {
        skill_effects.push(SkillEffectRender {
            x: effect.x,
            y: effect.y,
            effect_type: effect.effect_type,
            timer: effect.timer,
        });
    }

    Some(ExploreRender {
        data: Rc::clone(data),
        map_id: session.leader.current_map_id.clone(),
        player_x: session.leader.x,
        player_y: session.leader.y,
        player_facing: session.leader.facing,
        hp: as_u32(session.leader.stats.current_hp),
        max_hp: as_u32(session.leader.stats.max_hp),
        mp: as_u32(session.leader.stats.current_mp),
        max_mp: as_u32(session.leader.stats.max_mp),
        level: as_u32(session.leader.stats.level),
        active_quest_count: session
            .quests
            .iter()
            .filter(|quest| !quest.rewarded && !quest.completed)
            .count(),
        first_live_enemy_name,
        opened_treasures: session.opened_treasures.clone(),
        enemies,
        player_hit_flash: render_fx.player_hit_flash,
        skill_effects,
        skill_cooldowns: session.skill_cooldowns,
        key_actions: ui.explore.key_actions,
        peaceful: map.peaceful,
    })
}

fn refresh_first_live_enemy_name(explore: &mut ExploreRender) {
    explore.first_live_enemy_name = explore
        .enemies
        .iter()
        .find(|enemy| !enemy.dead && enemy.hp > 0)
        .map(|enemy| enemy.name.clone());
}

fn item_to_inventory_render(item: &Item) -> InventoryItemRender {
    InventoryItemRender {
        name: item.name.clone(),
        kind: item.kind,
    }
}

fn item_to_shop_render(item: &Item) -> ShopItemRender {
    ShopItemRender {
        name: item.name.clone(),
        price: item.price / 2,
    }
}

fn dialog_text_at(lines: &[String], line: usize) -> Option<String> {
    lines.get(line).cloned()
}

fn apply_explore_render_event(
    explore: &mut ExploreRender,
    event: &GameEvent,
    render_fx: &RenderFxState,
) {
    match event {
        GameEvent::Movement(MovementEvent::Tick(movement, _)) => {
            if let Some((dx, dy)) = movement.step {
                explore.player_x = (explore.player_x as i32 + dx) as usize;
                explore.player_y = (explore.player_y as i32 + dy) as usize;
            }
            if let Some((dx, dy)) = movement.facing {
                explore.player_facing = match (dx, dy) {
                    (0, -1) => Direction::Up,
                    (0, 1) => Direction::Down,
                    (-1, 0) => Direction::Left,
                    (1, 0) => Direction::Right,
                    _ => explore.player_facing,
                };
            }
        }
        GameEvent::Session(SessionEvent::SetPlayerMap(map_id)) => {
            explore.map_id = map_id.clone();
        }
        GameEvent::Session(SessionEvent::SetPlayerPosition { x, y }) => {
            explore.player_x = *x;
            explore.player_y = *y;
        }
        GameEvent::Session(SessionEvent::SetPlayerFacing(facing)) => {
            explore.player_facing = *facing;
        }
        GameEvent::Session(SessionEvent::SetPlayerStats(stats)) => {
            explore.hp = as_u32(stats.current_hp);
            explore.max_hp = as_u32(stats.max_hp);
            explore.mp = as_u32(stats.current_mp);
            explore.max_mp = as_u32(stats.max_mp);
            explore.level = as_u32(stats.level);
        }
        GameEvent::Session(SessionEvent::SetSkillCooldowns(cooldowns)) => {
            explore.skill_cooldowns = *cooldowns;
        }
        GameEvent::Session(SessionEvent::AddOpenedTreasure { map_id, x, y }) => {
            if !explore
                .opened_treasures
                .iter()
                .any(|(m, tx, ty)| m == map_id && *tx == *x && *ty == *y)
            {
                explore.opened_treasures.push((map_id.clone(), *x, *y));
            }
        }
        GameEvent::Combat(CombatEvent::SetMapEnemies { enemies, .. }) => {
            explore.enemies.clear();
            explore.enemies.reserve(enemies.len());
            for enemy in enemies {
                explore.enemies.push(EnemyRender {
                    enemy_id: enemy.instance_id,
                    name: enemy.data.name.clone(),
                    x: enemy.x,
                    y: enemy.y,
                    hp: enemy.hp,
                    max_hp: enemy.data.hp,
                    hit_flash: render_fx.enemy_hit_flash(enemy.instance_id),
                    dead: enemy.hp <= 0,
                });
            }
            refresh_first_live_enemy_name(explore);
        }
        GameEvent::Combat(CombatEvent::EnemySpawn(enemy)) => {
            explore.enemies.push(EnemyRender {
                enemy_id: enemy.instance_id,
                name: enemy.data.name.clone(),
                x: enemy.x,
                y: enemy.y,
                hp: enemy.hp,
                max_hp: enemy.data.hp,
                hit_flash: render_fx.enemy_hit_flash(enemy.instance_id),
                dead: enemy.hp <= 0,
            });
            refresh_first_live_enemy_name(explore);
        }
        GameEvent::Combat(CombatEvent::EnemyDespawn(enemy_id)) => {
            explore.enemies.retain(|enemy| enemy.enemy_id != *enemy_id);
            refresh_first_live_enemy_name(explore);
        }
        GameEvent::Combat(CombatEvent::EnemyMove { enemy_id, x, y }) => {
            if let Some(enemy) = explore.enemies.iter_mut().find(|e| e.enemy_id == *enemy_id) {
                enemy.x = *x;
                enemy.y = *y;
            }
        }
        GameEvent::Combat(CombatEvent::EnemyHpSet { enemy_id, hp }) => {
            if let Some(enemy) = explore.enemies.iter_mut().find(|e| e.enemy_id == *enemy_id) {
                enemy.hp = *hp;
                enemy.dead = *hp <= 0;
                refresh_first_live_enemy_name(explore);
            }
        }
        GameEvent::Combat(CombatEvent::EnemyHitFlashSet { enemy_id, .. }) => {
            if let Some(enemy) = explore.enemies.iter_mut().find(|e| e.enemy_id == *enemy_id) {
                enemy.hit_flash = render_fx.enemy_hit_flash(*enemy_id);
            }
        }
        GameEvent::Combat(CombatEvent::SetPlayerHitFlash(_)) => {
            explore.player_hit_flash = render_fx.player_hit_flash;
        }
        GameEvent::Combat(CombatEvent::SetSkillEffects(effects)) => {
            explore.skill_effects.clear();
            explore.skill_effects.reserve(effects.len());
            for effect in effects {
                explore.skill_effects.push(SkillEffectRender {
                    x: effect.x,
                    y: effect.y,
                    effect_type: effect.effect_type,
                    timer: effect.timer,
                });
            }
        }
        GameEvent::Combat(CombatEvent::TickSkillEffects) => {
            for effect in &mut explore.skill_effects {
                if effect.timer > 0 {
                    effect.timer -= 1;
                }
            }
            explore.skill_effects.retain(|e| e.timer > 0);
        }
        _ => {}
    }
}

pub fn apply_render_event(
    render_state: &mut RenderState,
    state: &GameState,
    session: Option<&SessionState>,
    ui: &UiState,
    data: &Rc<GameData>,
    event: &GameEvent,
    render_fx: &RenderFxState,
) {
    if matches!(
        event,
        GameEvent::Lifecycle(_)
            | GameEvent::Loading(_)
            | GameEvent::Transition(_)
            | GameEvent::OpenDialogState(_)
            | GameEvent::OpenShopState(_)
            | GameEvent::ApplyDialogTransition(_)
    ) {
        *render_state = render_state_from_game_state(state, session, ui, data, render_fx);
        return;
    }

    if let GameEvent::Loading(crate::game::LoadingEvent::Advance(step)) = event
        && let RenderState::Loading { step: render_step } = render_state
    {
        if *render_step != *step {
            *render_step = *step;
        }
        return;
    }

    if let GameEvent::Session(session_event) = event {
        match session_event {
            SessionEvent::SetPlayerInventory(player_inventory) => {
                if let RenderState::Inventory(inventory) = render_state {
                    inventory.items.clear();
                    inventory.items.reserve(player_inventory.len());
                    for item in player_inventory {
                        inventory.items.push(item_to_inventory_render(item));
                    }
                    if !inventory.items.is_empty() && inventory.selected >= inventory.items.len() {
                        inventory.selected = inventory.items.len() - 1;
                    }
                    inventory.scroll = scroll_for_selection(
                        inventory.selected,
                        inventory.items.len(),
                        INVENTORY_VISIBLE_ITEMS,
                    );
                    return;
                }
                if let RenderState::Shop(shop) = render_state {
                    shop.player_inventory.clear();
                    shop.player_inventory.reserve(player_inventory.len());
                    for item in player_inventory {
                        shop.player_inventory.push(item_to_shop_render(item));
                    }
                    return;
                }
            }
            SessionEvent::AddPlayerItem(item) => {
                if let RenderState::Inventory(inventory) = render_state {
                    inventory.items.push(item_to_inventory_render(item));
                    inventory.scroll = scroll_for_selection(
                        inventory.selected,
                        inventory.items.len(),
                        INVENTORY_VISIBLE_ITEMS,
                    );
                    return;
                }
                if let RenderState::Shop(shop) = render_state {
                    shop.player_inventory.push(item_to_shop_render(item));
                    return;
                }
            }
            SessionEvent::SetEquippedWeapon(index) => {
                if let RenderState::Inventory(inventory) = render_state {
                    inventory.equipped_weapon = *index;
                    return;
                }
            }
            SessionEvent::SetEquippedArmor(index) => {
                if let RenderState::Inventory(inventory) = render_state {
                    inventory.equipped_armor = *index;
                    return;
                }
            }
            SessionEvent::SetEquippedAccessory(index) => {
                if let RenderState::Inventory(inventory) = render_state {
                    inventory.equipped_accessory = *index;
                    return;
                }
            }
            SessionEvent::SetPlayerStats(stats) => {
                if let RenderState::Shop(shop) = render_state {
                    shop.player_gold = stats.gold;
                    return;
                }
                if let RenderState::Stats(stats_render) = render_state {
                    stats_render.hp = as_u32(stats.current_hp);
                    stats_render.max_hp = as_u32(stats.max_hp);
                    stats_render.mp = as_u32(stats.current_mp);
                    stats_render.max_mp = as_u32(stats.max_mp);
                    stats_render.level = as_u32(stats.level);
                    stats_render.exp = as_u32(stats.exp);
                    stats_render.gold = as_u32(stats.gold);
                    return;
                }
            }
            SessionEvent::AddQuestProgress(progress) => {
                if let RenderState::QuestLog(quest_log) = render_state {
                    if progress.rewarded {
                        quest_log
                            .quests
                            .retain(|entry| entry.quest_id != progress.quest_id);
                        return;
                    }

                    if let Some(entry) = quest_log
                        .quests
                        .iter_mut()
                        .find(|entry| entry.quest_id == progress.quest_id)
                    {
                        entry.current_count = as_u32(progress.current_count);
                        entry.completed = progress.completed;
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    if let GameEvent::OpenDialogState(dialog_state) = event
        && let RenderState::Dialog {
            npc_name,
            lines,
            current_line,
            current_text,
            has_next,
            ..
        } = render_state
    {
        *npc_name = dialog_state.npc_name.clone();
        *lines = dialog_state
            .lines
            .iter()
            .map(|line| line.text.clone())
            .collect();
        *current_line = dialog_state.current_line;
        *current_text = dialog_text_at(lines, *current_line);
        *has_next = *current_line + 1 < lines.len();
        return;
    }

    if let GameEvent::ApplyDialogTransition(crate::game::DialogTransition::SetLine(line)) = event
        && let RenderState::Dialog {
            lines,
            current_line,
            current_text,
            has_next,
            ..
        } = render_state
    {
        *current_line = *line;
        *current_text = dialog_text_at(lines, *current_line);
        *has_next = *current_line + 1 < lines.len();
        return;
    }

    match render_state {
        RenderState::Explore(explore) => apply_explore_render_event(explore, event, render_fx),
        RenderState::Dialog {
            explore: Some(explore),
            ..
        } => apply_explore_render_event(explore, event, render_fx),
        RenderState::PauseMenu {
            explore: Some(explore),
            ..
        } => apply_explore_render_event(explore, event, render_fx),
        _ => {}
    }
}

pub fn apply_ui_render_patch(
    render_state: &mut RenderState,
    ui: &UiState,
    session: Option<&SessionState>,
) {
    match render_state {
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
            let Some(s) = session else {
                return;
            };
            inventory.selected = ui.inventory.selected;
            inventory.scroll = scroll_for_selection(
                inventory.selected,
                s.leader.inventory.len(),
                INVENTORY_VISIBLE_ITEMS,
            );
        }
        RenderState::Shop(shop) => {
            let Some(s) = session else {
                return;
            };
            let Some(shop_state) = ui.shop.state.as_ref() else {
                return;
            };
            shop.mode = ui.shop.mode;
            shop.selected = ui.shop.selected;
            let total = match ui.shop.mode {
                ShopMode::Select => 2,
                ShopMode::Buy => shop_state.items.len(),
                ShopMode::Sell => s.leader.inventory.len(),
            };
            shop.scroll = scroll_for_selection(shop.selected, total, SHOP_VISIBLE_ITEMS);
        }
        RenderState::Dialog {
            npc_name,
            lines,
            current_line,
            current_text,
            has_next,
            ..
        } => {
            let Some(dialog_state) = ui.dialog.state.as_ref() else {
                return;
            };
            *npc_name = dialog_state.npc_name.clone();
            *lines = dialog_state
                .lines
                .iter()
                .map(|line| line.text.clone())
                .collect();
            *current_line = dialog_state.current_line;
            *current_text = dialog_text_at(lines, *current_line);
            *has_next = *current_line + 1 < lines.len();
        }
        _ => {}
    }
}

pub fn apply_render_tick(render_state: &mut RenderState, render_fx: &RenderFxState) {
    match render_state {
        RenderState::Explore(explore) => {
            if explore.player_hit_flash != render_fx.player_hit_flash {
                explore.player_hit_flash = render_fx.player_hit_flash;
            }
            for enemy in &mut explore.enemies {
                let next = render_fx.enemy_hit_flash(enemy.enemy_id);
                if enemy.hit_flash != next {
                    enemy.hit_flash = next;
                }
            }
        }
        RenderState::Dialog {
            explore: Some(explore),
            ..
        }
        | RenderState::PauseMenu {
            explore: Some(explore),
            ..
        } => {
            if explore.player_hit_flash != render_fx.player_hit_flash {
                explore.player_hit_flash = render_fx.player_hit_flash;
            }
            for enemy in &mut explore.enemies {
                let next = render_fx.enemy_hit_flash(enemy.enemy_id);
                if enemy.hit_flash != next {
                    enemy.hit_flash = next;
                }
            }
        }
        _ => {}
    }
}

fn render_state_from_game_state(
    state: &GameState,
    session: Option<&SessionState>,
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
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            let Some(explore) = build_explore_render(s, ui, data, render_fx) else {
                return RenderState::Error(String::from("Map not found"));
            };
            RenderState::Explore(explore)
        }
        GameState::Inventory => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            let mut items = Vec::with_capacity(s.leader.inventory.len());
            for item in &s.leader.inventory {
                items.push(InventoryItemRender {
                    name: item.name.clone(),
                    kind: item.kind,
                });
            }
            RenderState::Inventory(InventoryRender {
                items,
                equipped_weapon: s.leader.equipped_weapon,
                equipped_armor: s.leader.equipped_armor,
                equipped_accessory: s.leader.equipped_accessory,
                selected: ui.inventory.selected,
                scroll: scroll_for_selection(
                    ui.inventory.selected,
                    s.leader.inventory.len(),
                    INVENTORY_VISIBLE_ITEMS,
                ),
            })
        }
        GameState::Stats => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::Stats(StatsRender {
                hp: as_u32(s.leader.stats.current_hp),
                max_hp: as_u32(s.leader.stats.max_hp),
                mp: as_u32(s.leader.stats.current_mp),
                max_mp: as_u32(s.leader.stats.max_mp),
                level: as_u32(s.leader.stats.level),
                atk: as_u32(s.leader.total_atk()),
                def: as_u32(s.leader.total_def()),
                exp: as_u32(s.leader.stats.exp),
                gold: as_u32(s.leader.stats.gold),
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

            let has_next = dialog_state.current_line + 1 < dialog_state.lines.len();

            RenderState::Dialog {
                explore: session.and_then(|s| build_explore_render(s, ui, data, render_fx)),
                npc_name: dialog_state.npc_name.clone(),
                lines: dialog_state
                    .lines
                    .iter()
                    .map(|line| line.text.clone())
                    .collect(),
                current_line: dialog_state.current_line,
                current_text,
                has_next,
            }
        }
        GameState::Shop => {
            let Some(s) = session else {
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
            let mut player_inventory = Vec::with_capacity(s.leader.inventory.len());
            for item in &s.leader.inventory {
                player_inventory.push(ShopItemRender {
                    name: item.name.clone(),
                    price: item.price / 2,
                });
            }
            RenderState::Shop(ShopRender {
                shop_name: shop_state.shop.name.clone(),
                mode: ui.shop.mode,
                selected: ui.shop.selected,
                scroll: scroll_for_selection(
                    ui.shop.selected,
                    match ui.shop.mode {
                        ShopMode::Select => 2,
                        ShopMode::Buy => shop_state.items.len(),
                        ShopMode::Sell => s.leader.inventory.len(),
                    },
                    SHOP_VISIBLE_ITEMS,
                ),
                buy_items,
                player_gold: s.leader.stats.gold,
                player_inventory,
            })
        }
        GameState::QuestLog => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            let mut quests = Vec::with_capacity(s.quests.len());
            for quest in &s.quests {
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
            RenderState::QuestLog(QuestLogRender { quests })
        }
        GameState::PauseMenu => RenderState::PauseMenu {
            explore: session.and_then(|s| build_explore_render(s, ui, data, render_fx)),
            items: ui.pause_menu.state.items.clone(),
            selected: ui.pause_menu.selected,
        },
        GameState::GameOver => RenderState::GameOver,
        GameState::Error(msg) => RenderState::Error(msg.clone()),
    }
}

pub fn render(state: &RenderState, fb: &mut Framebuffer) {
    match state {
        RenderState::Loading { step } => draw_loading(fb, *step),
        RenderState::Menu {
            title,
            items,
            selected,
        } => draw_menu(fb, title, items, *selected),
        RenderState::Explore(explore) => draw_explore(fb, explore),
        RenderState::Inventory(inventory) => draw_inventory(fb, inventory),
        RenderState::Stats(stats) => draw_stats(fb, stats),
        RenderState::Dialog {
            explore,
            npc_name,
            lines: _,
            current_line: _,
            current_text,
            has_next,
        } => {
            if let Some(explore_state) = explore {
                draw_explore(fb, explore_state);
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
                draw_explore(fb, explore_state);
            }
            draw_pause_menu(fb, items, *selected);
        }
        RenderState::GameOver => {
            clear_screen(fb);
            let w = fb.width() as i32;
            let h = fb.height() as i32;
            fill_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_DARK_GRAY);
            draw_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_RED);
            draw_text(fb, w / 2 - 35, h / 2 - 8, "GAME OVER", COLOR_RED);
            draw_text(fb, w / 2 - 30, h / 2 + 8, "OK:Menu", COLOR_WHITE);
        }
        RenderState::Error(msg) => {
            clear_screen(fb);
            let w = fb.width() as i32;
            let h = fb.height() as i32;
            fill_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_DARK_GRAY);
            draw_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_RED);
            draw_text(fb, 16, h / 2 - 20, "ERROR", COLOR_RED);
            draw_text(fb, 16, h / 2 - 4, msg, COLOR_WHITE);
            draw_text(fb, 16, h / 2 + 16, "OK:Exit", COLOR_WHITE);
        }
        RenderState::NoSession => {
            clear_screen(fb);
            draw_text(fb, 16, 16, "ERR: No session", COLOR_RED);
        }
    }
}

fn draw_loading(fb: &mut Framebuffer, step: usize) {
    clear_screen(fb);

    let w = fb.width() as i32;
    let h = fb.height() as i32;

    draw_text(fb, w / 2 - 30, h / 2 - 30, "Loading...", COLOR_WHITE);

    let clamped = step.min(GameData::LOAD_STEPS - 1);
    let label = GameData::LOAD_LABELS[clamped];
    draw_text(fb, w / 2 - 30, h / 2 - 10, label, COLOR_CYAN);

    let bar_w = 120;
    let bar_h = 8;
    let bar_x = w / 2 - bar_w / 2;
    let bar_y = h / 2 + 10;

    draw_rect(fb, bar_x, bar_y, bar_w, bar_h, COLOR_WHITE);
    let progress = (((clamped + 1) * bar_w as usize / GameData::LOAD_STEPS) as i32).min(bar_w);
    fill_rect(
        fb,
        bar_x + 1,
        bar_y + 1,
        progress - 2,
        bar_h - 2,
        COLOR_GREEN,
    );
}
