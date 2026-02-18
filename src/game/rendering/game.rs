use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;

use wipi::framebuffer::Framebuffer;

use crate::game::ui::{INVENTORY_VISIBLE_ITEMS, SHOP_VISIBLE_ITEMS, ShopMode, UiState};
use crate::game::{GameData, GameState, SpriteAtlas, WorldState};

use super::dialog::draw_dialog;
use super::explore::draw_explore;
use super::inventory::{draw_inventory, draw_stats};
use super::menu::{draw_menu, draw_pause_menu};
use super::quest::draw_quest_log;
use super::render_fx::RenderFxState;
use super::render_state::{
    ExploreRender, InventoryRender, QuestLogRender, RenderState, ShopRender, SkillEffectRender,
    StatsRender, scroll_for_selection,
};
use super::renderer::{
    COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, clear_screen, draw_rect, draw_text, fill_rect,
};
use super::shop::draw_shop;

impl RenderState {
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
                explore.skill_effects = render_fx
                    .skill_effect_iter()
                    .map(|(x, y, effect_type)| SkillEffectRender { x, y, effect_type })
                    .collect();
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
