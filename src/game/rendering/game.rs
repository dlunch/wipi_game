use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use wipi::framebuffer::Framebuffer;

use crate::data::{Direction, ItemKind, SkillType};
use crate::game::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, GameData, GameState,
    INVENTORY_VISIBLE_ITEMS, MenuAction, SHOP_VISIBLE_ITEMS, SessionState, ShopMode, UiState,
    clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu, draw_pause_menu,
    draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect,
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
    pub peaceful: bool,
}

pub struct EnemyRender {
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
    pub name: String,
    pub description: String,
    pub current_count: u32,
    pub target_count: u32,
    pub completed: bool,
}

fn as_u32(value: i32) -> u32 {
    value.max(0) as u32
}

fn scroll_for_selection(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }

    let max_scroll = total.saturating_sub(visible);
    selected.saturating_sub(visible - 1).min(max_scroll)
}

fn build_explore_render(session: &SessionState, data: &Rc<GameData>) -> Option<ExploreRender> {
    let map = data.find_map(&session.player.current_map_id)?;

    let first_live_enemy_name = session
        .combat
        .enemies
        .iter()
        .find(|enemy| !enemy.is_dead())
        .map(|enemy| enemy.data.name.clone());

    let enemies = session
        .combat
        .enemies
        .iter()
        .map(|enemy| EnemyRender {
            x: enemy.x,
            y: enemy.y,
            hp: enemy.hp,
            max_hp: enemy.data.hp,
            hit_flash: enemy.hit_flash,
            dead: enemy.is_dead(),
        })
        .collect();

    let skill_effects = session
        .combat
        .skill_effects
        .iter()
        .map(|effect| SkillEffectRender {
            x: effect.x,
            y: effect.y,
            effect_type: effect.effect_type,
        })
        .collect();

    Some(ExploreRender {
        data: Rc::clone(data),
        map_id: session.player.current_map_id.clone(),
        player_x: session.player.x,
        player_y: session.player.y,
        player_facing: session.player.facing,
        hp: as_u32(session.player.stats.current_hp),
        max_hp: as_u32(session.player.stats.max_hp),
        mp: as_u32(session.player.stats.current_mp),
        max_mp: as_u32(session.player.stats.max_mp),
        level: as_u32(session.player.stats.level),
        active_quest_count: session
            .player
            .quests
            .iter()
            .filter(|quest| !quest.rewarded && !quest.completed)
            .count(),
        first_live_enemy_name,
        opened_treasures: session.player.opened_treasures.clone(),
        enemies,
        player_hit_flash: session.combat.player_hit_flash,
        skill_effects,
        skill_cooldowns: session.skill_cooldowns,
        peaceful: map.peaceful,
    })
}

pub fn build_render_state(
    state: &GameState,
    session: Option<&SessionState>,
    ui: &UiState,
    data: &Rc<GameData>,
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
            let Some(explore) = build_explore_render(s, data) else {
                return RenderState::Error(String::from("Map not found"));
            };
            RenderState::Explore(explore)
        }
        GameState::Inventory => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::Inventory(InventoryRender {
                items: s
                    .player
                    .inventory
                    .iter()
                    .map(|item| InventoryItemRender {
                        name: item.name.clone(),
                        kind: item.kind,
                    })
                    .collect(),
                equipped_weapon: s.player.equipped_weapon,
                equipped_armor: s.player.equipped_armor,
                equipped_accessory: s.player.equipped_accessory,
                selected: ui.inventory.selected,
                scroll: scroll_for_selection(
                    ui.inventory.selected,
                    s.player.inventory.len(),
                    INVENTORY_VISIBLE_ITEMS,
                ),
            })
        }
        GameState::Stats => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            RenderState::Stats(StatsRender {
                hp: as_u32(s.player.stats.current_hp),
                max_hp: as_u32(s.player.stats.max_hp),
                mp: as_u32(s.player.stats.current_mp),
                max_mp: as_u32(s.player.stats.max_mp),
                level: as_u32(s.player.stats.level),
                atk: as_u32(s.player.total_atk()),
                def: as_u32(s.player.total_def()),
                exp: as_u32(s.player.stats.exp),
                gold: as_u32(s.player.stats.gold),
            })
        }
        GameState::Dialog => {
            let Some(dialog_state) = ui.dialog.state.as_ref() else {
                return RenderState::Error(String::from("No dialog state"));
            };
            let current_text = data
                .find_dialog(&dialog_state.dialog_id)
                .and_then(|dialog| dialog.lines.get(dialog_state.current_line))
                .map(|line| line.text.clone());

            let has_next = data
                .find_dialog(&dialog_state.dialog_id)
                .map(|dialog| dialog_state.current_line + 1 < dialog.lines.len())
                .unwrap_or(false);

            RenderState::Dialog {
                explore: session.and_then(|s| build_explore_render(s, data)),
                npc_name: dialog_state.npc_name.clone(),
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
            RenderState::Shop(ShopRender {
                shop_name: shop_state.shop.name.clone(),
                mode: ui.shop.mode,
                selected: ui.shop.selected,
                scroll: scroll_for_selection(
                    ui.shop.selected,
                    match ui.shop.mode {
                        ShopMode::Select => 2,
                        ShopMode::Buy => shop_state.items.len(),
                        ShopMode::Sell => s.player.inventory.len(),
                    },
                    SHOP_VISIBLE_ITEMS,
                ),
                buy_items: shop_state
                    .items
                    .iter()
                    .map(|item| ShopItemRender {
                        name: item.name.clone(),
                        price: item.price,
                    })
                    .collect(),
                player_gold: s.player.stats.gold,
                player_inventory: s
                    .player
                    .inventory
                    .iter()
                    .map(|item| ShopItemRender {
                        name: item.name.clone(),
                        price: item.price / 2,
                    })
                    .collect(),
            })
        }
        GameState::QuestLog => {
            let Some(s) = session else {
                return RenderState::NoSession;
            };
            let quests = s
                .player
                .quests
                .iter()
                .filter(|quest| !quest.rewarded)
                .filter_map(|quest| {
                    data.find_quest(&quest.quest_id)
                        .map(|quest_data| QuestEntryRender {
                            name: quest_data.name.clone(),
                            description: quest_data.description.clone(),
                            current_count: as_u32(quest.current_count),
                            target_count: as_u32(quest_data.target_count),
                            completed: quest.completed,
                        })
                })
                .collect();
            RenderState::QuestLog(QuestLogRender { quests })
        }
        GameState::PauseMenu => RenderState::PauseMenu {
            explore: session.and_then(|s| build_explore_render(s, data)),
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
