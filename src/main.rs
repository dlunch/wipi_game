#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::string::String;

use wipi::{app::App, event::KeyCode, framebuffer::Framebuffer, graphics::repaint, wipi_main};

use data::{Skill, SkillType};
use game::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, CombatSystem, GameData,
    GameState, InventoryState, MenuState, MovementController, NpcInteraction, Player, QuestSystem,
    ShopMode, TileEvent, check_tile_event, clear_screen, draw_dialog, draw_explore, draw_inventory,
    draw_menu, draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect,
    has_save_data, load_game, save_game,
};

pub struct RpgGame {
    state: GameState,
    player: Player,
    data: GameData,
    inventory_state: InventoryState,
    combat: CombatSystem,
    movement: MovementController,
    mp_regen_timer: u32,
}

impl Default for RpgGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpgGame {
    pub fn new() -> Self {
        Self {
            state: GameState::Loading(0),
            player: Player::new(String::from("Hero"), "village"),
            data: GameData::default(),
            inventory_state: InventoryState::default(),
            combat: CombatSystem::new(),
            movement: MovementController::default(),
            mp_regen_timer: 0,
        }
    }

    fn current_map(&self) -> Option<&data::Map> {
        self.data.find_map(&self.player.current_map_id)
    }

    fn start_new_game(&mut self) {
        self.player = Player::new(String::from("Hero"), "village");

        if let Some(sword) = self.data.find_item("wooden_sword").cloned() {
            self.player.add_item(sword);
            self.player.equipped_weapon = Some(0);
        }
        if let Some(armor) = self.data.find_item("cloth").cloned() {
            self.player.add_item(armor);
            self.player.equipped_armor = Some(1);
        }
        if let Some(potion) = self.data.find_item("potion").cloned() {
            self.player.add_item(potion.clone());
            self.player.add_item(potion);
        }

        if let Some(map) = self.data.find_map("village") {
            self.player.spawn_at_map(map);
            self.combat.spawn_enemies(map, &self.data.enemies);
        }

        self.state = GameState::Explore;
    }

    fn continue_game(&mut self) {
        self.player = Player::new(String::from("Hero"), "village");

        if load_game(&mut self.player) {
            if let Some(map) = self.data.find_map(&self.player.current_map_id) {
                self.combat.spawn_enemies(map, &self.data.enemies);
            }
            self.state = GameState::Explore;
        } else {
            self.start_new_game();
        }
    }

    fn update_loading(&mut self) {
        let GameState::Loading(step) = self.state else {
            return;
        };

        self.draw_loading(step);

        if self.data.load_step(step) {
            self.state = GameState::Menu(MenuState {
                selected: 0,
                has_save: has_save_data(),
            });
        } else {
            self.state = GameState::Loading(step + 1);
        }
    }

    fn draw_loading(&self, step: usize) {
        let mut fb = Framebuffer::screen_framebuffer();
        clear_screen(&mut fb);

        let w = fb.width() as i32;
        let h = fb.height() as i32;

        draw_text(&mut fb, w / 2 - 30, h / 2 - 30, "Loading...", COLOR_WHITE);

        let label = GameData::LOAD_LABELS.get(step).unwrap_or(&"");
        draw_text(&mut fb, w / 2 - 30, h / 2 - 10, label, COLOR_CYAN);

        let bar_w = 120;
        let bar_h = 8;
        let bar_x = w / 2 - bar_w / 2;
        let bar_y = h / 2 + 10;

        draw_rect(&mut fb, bar_x, bar_y, bar_w, bar_h, COLOR_WHITE);
        let progress = ((step + 1) * bar_w as usize / GameData::LOAD_STEPS) as i32;
        fill_rect(
            &mut fb,
            bar_x + 1,
            bar_y + 1,
            progress - 2,
            bar_h - 2,
            COLOR_GREEN,
        );

        repaint(0, 0, 0, w, h);
    }

    fn update_movement(&mut self) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }

        let map_id = self.player.current_map_id.clone();
        let Some(map) = self.data.find_map(&map_id) else {
            return;
        };

        let moved = self
            .movement
            .update(&mut self.player, map, &self.combat, &self.data.npcs);

        if moved {
            self.check_tile_events();
        }
    }

    fn update_combat(&mut self) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }

        self.player.update_cooldowns();

        self.mp_regen_timer += 1;
        if self.mp_regen_timer >= 60 {
            self.mp_regen_timer = 0;
            self.player.stats.recover_mp(1);
        }

        let player_x = self.player.x;
        let player_y = self.player.y;
        let player_def = self.player.total_def();
        let map_id = self.player.current_map_id.clone();

        if let Some(map) = self.data.find_map(&map_id) {
            let result =
                self.combat
                    .update(player_x, player_y, player_def, map, &self.data.enemies);

            if result.damage_taken > 0 {
                self.player.stats.take_damage(result.damage_taken);

                if self.player.stats.is_dead() {
                    self.state = GameState::GameOver;
                }
            }
        }
    }

    fn use_skill(&mut self, slot: usize, skill: &Skill) {
        if !self.player.can_use_skill(slot, skill.mp_cost) {
            return;
        }

        let result = self.combat.use_skill(
            skill,
            self.player.x,
            self.player.y,
            self.player.total_atk(),
            self.player.facing,
        );

        self.player.use_skill(slot, skill.mp_cost, skill.cooldown);

        if skill.skill_type == SkillType::Heal {
            self.player.stats.heal(result.heal_amount);
        }

        for kill in result.kills {
            self.player.stats.add_exp(kill.exp);
            self.player.stats.gold += kill.gold;
            QuestSystem::on_enemy_killed(&mut self.player, &self.data, &kill.enemy_id);
        }
    }

    fn check_tile_events(&mut self) {
        let event = self
            .current_map()
            .and_then(|map| check_tile_event(map, &self.player));

        let Some(event) = event else { return };

        match event {
            TileEvent::MapExit(target) | TileEvent::DungeonEntrance(target) => {
                if !target.is_empty() {
                    self.change_map(&target);
                }
            }
            TileEvent::Treasure => {
                let map_id = self.player.current_map_id.clone();
                if !self
                    .player
                    .is_treasure_opened(&map_id, self.player.x, self.player.y)
                {
                    if let Some(potion) = self.data.find_item("potion").cloned() {
                        self.player.add_item(potion);
                    }
                    self.player
                        .open_treasure(&map_id, self.player.x, self.player.y);
                }
            }
        }
    }

    fn change_map(&mut self, target_id: &str) {
        let Some(map) = self.data.find_map(target_id).cloned() else {
            return;
        };

        self.player.current_map_id = map.id.clone();
        if let Some((x, y)) = map.find_player_start() {
            self.player.x = x;
            self.player.y = y;
        }
        self.combat.spawn_enemies(&map, &self.data.enemies);
    }

    fn try_interact_with_npc(&mut self) {
        let facing = self.player.facing;
        if let Some(new_state) = NpcInteraction::try_interact(&mut self.player, &self.data, facing)
        {
            self.state = new_state;
        }
    }

    fn process_dialog_action(&mut self) {
        let GameState::Dialog(ref state) = self.state else {
            return;
        };

        let Some(action) = state.current_action().cloned() else {
            return;
        };

        if let Some(new_state) =
            NpcInteraction::process_action(&mut self.player, &self.data, &action)
        {
            self.state = new_state;
        }
    }

    fn draw_pause_menu(&self, fb: &mut Framebuffer, selected: usize) {
        let w = fb.width() as i32;
        let h = fb.height() as i32;
        let menu_w = 100;
        let menu_h = 80;
        let x = (w - menu_w) / 2;
        let y = (h - menu_h) / 2;

        fill_rect(fb, x, y, menu_w, menu_h, COLOR_DARK_GRAY);
        draw_rect(fb, x, y, menu_w, menu_h, COLOR_WHITE);

        let items = ["Inventory", "Stats", "Quests", "Save"];
        for (i, item) in items.iter().enumerate() {
            let is_selected = i == selected;
            let prefix = if is_selected { "> " } else { "  " };
            let y_pos = y + 10 + (i as i32 * 16);
            if is_selected {
                draw_text(fb, x + 10, y_pos, prefix, COLOR_RED);
                draw_text(fb, x + 22, y_pos, item, COLOR_RED);
            } else {
                draw_text(fb, x + 10, y_pos, prefix, COLOR_WHITE);
                draw_text(fb, x + 22, y_pos, item, COLOR_WHITE);
            }
        }
    }

    fn handle_menu_input(&mut self, key: KeyCode) {
        let GameState::Menu(ref mut menu) = self.state else {
            return;
        };

        match key {
            KeyCode::Up => menu.move_up(),
            KeyCode::Down => menu.move_down(),
            KeyCode::Ok => {
                let action = if menu.has_save {
                    match menu.selected {
                        0 => MenuAction::NewGame,
                        1 => MenuAction::Continue,
                        _ => MenuAction::Exit,
                    }
                } else {
                    match menu.selected {
                        0 => MenuAction::NewGame,
                        _ => MenuAction::Exit,
                    }
                };

                match action {
                    MenuAction::NewGame => self.start_new_game(),
                    MenuAction::Continue => self.continue_game(),
                    MenuAction::Exit => wipi::kernel::exit(0),
                }
            }
            _ => {}
        }
    }

    fn handle_explore_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                self.movement.on_direction_pressed(key);
            }
            KeyCode::Ok => {
                self.try_interact_with_npc();
                if matches!(self.state, GameState::Dialog(_)) {
                    return;
                }

                if let Some(reward) = self.combat.player_attack(
                    self.player.x,
                    self.player.y,
                    self.player.total_atk(),
                    self.player.facing,
                ) {
                    self.player.stats.add_exp(reward.exp);
                    self.player.stats.gold += reward.gold;
                    QuestSystem::on_enemy_killed(&mut self.player, &self.data, &reward.enemy_id);
                }
            }
            KeyCode::Key1 => self.use_skill(0, &Skill::FIREBALL),
            KeyCode::Key2 => self.use_skill(1, &Skill::HEAL),
            KeyCode::Key3 => self.use_skill(2, &Skill::SPIN_ATTACK),
            KeyCode::Key0 => self.state = GameState::PauseMenu(0),
            KeyCode::Back => {
                save_game(&self.player);
                self.state = GameState::Menu(MenuState {
                    selected: 0,
                    has_save: has_save_data(),
                });
            }
            _ => {}
        }
    }

    fn handle_inventory_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => self.inventory_state.move_up(),
            KeyCode::Down => self.inventory_state.move_down(self.player.inventory.len()),
            KeyCode::Ok => {
                self.player.use_item(self.inventory_state.selected);
            }
            KeyCode::Back => self.state = GameState::Explore,
            _ => {}
        }
    }

    fn handle_dialog_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Ok => {
                self.process_dialog_action();

                if matches!(self.state, GameState::Shop(_)) {
                    return;
                }

                if let GameState::Dialog(ref mut state) = self.state
                    && !state.advance()
                {
                    self.state = GameState::Explore;
                }
            }
            KeyCode::Back => self.state = GameState::Explore,
            _ => {}
        }
    }

    fn handle_shop_input(&mut self, key: KeyCode) {
        const VISIBLE_ITEMS: usize = 8;

        let GameState::Shop(ref mut state) = self.state else {
            return;
        };

        match state.mode {
            ShopMode::Select => match key {
                KeyCode::Up => state.move_up(),
                KeyCode::Down => state.move_down(2, 2),
                KeyCode::Ok => {
                    state.mode = if state.selected == 0 {
                        ShopMode::Buy
                    } else {
                        ShopMode::Sell
                    };
                    state.reset_selection();
                }
                KeyCode::Back => self.state = GameState::Explore,
                _ => {}
            },
            ShopMode::Buy => match key {
                KeyCode::Up => state.move_up(),
                KeyCode::Down => state.move_down(state.items.len(), VISIBLE_ITEMS),
                KeyCode::Ok => {
                    if let Some(item) = state.items.get(state.selected).cloned()
                        && self.player.stats.gold >= item.price
                    {
                        self.player.stats.gold -= item.price;
                        self.player.add_item(item);
                    }
                }
                KeyCode::Back => {
                    state.mode = ShopMode::Select;
                    state.reset_selection();
                }
                _ => {}
            },
            ShopMode::Sell => match key {
                KeyCode::Up => state.move_up(),
                KeyCode::Down => state.move_down(self.player.inventory.len(), VISIBLE_ITEMS),
                KeyCode::Ok => {
                    if let Some(item) = self.player.remove_item_at(state.selected) {
                        self.player.stats.gold += item.price / 2;

                        let inv_len = self.player.inventory.len();
                        if state.selected >= inv_len && state.selected > 0 {
                            state.selected -= 1;
                        }
                        if state.scroll > 0
                            && state.scroll >= inv_len.saturating_sub(VISIBLE_ITEMS - 1)
                        {
                            state.scroll = inv_len.saturating_sub(VISIBLE_ITEMS);
                        }
                    }
                }
                KeyCode::Back => {
                    state.mode = ShopMode::Select;
                    state.reset_selection();
                }
                _ => {}
            },
        }
    }

    fn handle_pause_menu_input(&mut self, key: KeyCode) {
        let GameState::PauseMenu(ref mut selected) = self.state else {
            return;
        };

        match key {
            KeyCode::Up if *selected > 0 => *selected -= 1,
            KeyCode::Down if *selected < 3 => *selected += 1,
            KeyCode::Ok => match *selected {
                0 => {
                    self.inventory_state = InventoryState::default();
                    self.state = GameState::Inventory;
                }
                1 => self.state = GameState::Stats,
                2 => self.state = GameState::QuestLog,
                3 => {
                    save_game(&self.player);
                    self.state = GameState::Explore;
                }
                _ => {}
            },
            KeyCode::Back | KeyCode::Key0 => self.state = GameState::Explore,
            _ => {}
        }
    }
}

enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

impl App for RpgGame {
    fn on_paint(&mut self) {
        if matches!(self.state, GameState::Loading(_)) {
            self.update_loading();
            return;
        }

        self.update_movement();
        self.update_combat();

        let mut fb = Framebuffer::screen_framebuffer();

        match &self.state {
            GameState::Loading(_) => {}
            GameState::Menu(menu_state) => draw_menu(&mut fb, menu_state),
            GameState::Explore => {
                if let Some(map) = self.current_map() {
                    draw_explore(&mut fb, map, &self.player, &self.combat, &self.data.npcs);
                }
            }
            GameState::Inventory => {
                draw_inventory(&mut fb, &self.player, &self.inventory_state);
            }
            GameState::Stats => draw_stats(&mut fb, &self.player),
            GameState::Dialog(dialog_state) => {
                if let Some(map) = self.current_map() {
                    draw_explore(&mut fb, map, &self.player, &self.combat, &self.data.npcs);
                }
                draw_dialog(&mut fb, dialog_state);
            }
            GameState::Shop(shop_state) => draw_shop(&mut fb, shop_state, &self.player),
            GameState::QuestLog => draw_quest_log(&mut fb, &self.player, &self.data.quests),
            GameState::PauseMenu(selected) => {
                if let Some(map) = self.current_map() {
                    draw_explore(&mut fb, map, &self.player, &self.combat, &self.data.npcs);
                }
                self.draw_pause_menu(&mut fb, *selected);
            }
            GameState::GameOver => {
                clear_screen(&mut fb);
                let w = fb.width() as i32;
                let h = fb.height() as i32;
                fill_rect(&mut fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_DARK_GRAY);
                draw_rect(&mut fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_RED);
                draw_text(&mut fb, w / 2 - 35, h / 2 - 8, "GAME OVER", COLOR_RED);
                draw_text(&mut fb, w / 2 - 30, h / 2 + 8, "OK:Menu", COLOR_WHITE);
            }
        }

        repaint(0, 0, 0, fb.width() as i32, fb.height() as i32);
    }

    fn on_keydown(&mut self, key: KeyCode) {
        match &self.state {
            GameState::Loading(_) => {}
            GameState::Menu(_) => self.handle_menu_input(key),
            GameState::Explore => self.handle_explore_input(key),
            GameState::Inventory => self.handle_inventory_input(key),
            GameState::Stats | GameState::QuestLog => {
                if matches!(key, KeyCode::Back | KeyCode::Ok) {
                    self.state = GameState::Explore;
                }
            }
            GameState::Dialog(_) => self.handle_dialog_input(key),
            GameState::Shop(_) => self.handle_shop_input(key),
            GameState::PauseMenu(_) => self.handle_pause_menu_input(key),
            GameState::GameOver => {
                if matches!(key, KeyCode::Ok) {
                    self.state = GameState::Menu(MenuState {
                        selected: 0,
                        has_save: has_save_data(),
                    });
                }
            }
        }
    }

    fn on_keyup(&mut self, key: KeyCode) {
        self.movement.on_key_released(key);
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}
