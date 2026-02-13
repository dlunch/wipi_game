#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::format;
use alloc::string::String;

use wipi::{app::App, event::KeyCode, framebuffer::Framebuffer, graphics::repaint, wipi_main};

use data::Skill;

const MP_REGEN_INTERVAL: u32 = 60;

use game::{
    COLOR_CYAN, COLOR_DARK_GRAY, COLOR_GREEN, COLOR_RED, COLOR_WHITE, CombatState, DialogIntent,
    GameData, GameState, InventoryIntent, InventoryState, MenuAction, MenuIntent, MenuState,
    MovementState, PauseMenuIntent, Player, PlayerEffect, ShopIntent, ShopMode, TileEvent,
    check_tile_event, clear_screen, draw_dialog, draw_explore, draw_inventory, draw_menu,
    draw_pause_menu, draw_quest_log, draw_rect, draw_shop, draw_stats, draw_text, fill_rect,
    has_save_data, load_game, pause_menu_intent_for_key, save_game,
};

enum AppAction {
    Tick,
    KeyDown(KeyCode),
    KeyUp(KeyCode),
}

enum AppEffect {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    ApplyMenuIntent(MenuIntent),
    ApplyExploreIntent(ExploreIntent),
    ApplyInventoryIntent(InventoryIntent),
    ApplyDialogIntent(DialogIntent),
    ApplyShopIntent(ShopIntent),
    ApplyPauseMenuIntent(PauseMenuIntent),
    ReturnToExplore,
    ReturnToMenuFromGameOver,
    ReleaseMovementKey(KeyCode),
    Exit(i32),
}

#[derive(Clone, Copy)]
enum ExploreIntent {
    MoveDirection(KeyCode),
    TryNpcInteract,
    Attack,
    Skill1,
    Skill2,
    Skill3,
    Pause,
    BackToMenu,
}

fn explore_intents_for_key(key: KeyCode) -> alloc::vec::Vec<ExploreIntent> {
    let mut intents = alloc::vec::Vec::new();
    match key {
        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
            intents.push(ExploreIntent::MoveDirection(key));
        }
        KeyCode::Ok => {
            intents.push(ExploreIntent::TryNpcInteract);
            intents.push(ExploreIntent::Attack);
        }
        KeyCode::Key1 => intents.push(ExploreIntent::Skill1),
        KeyCode::Key2 => intents.push(ExploreIntent::Skill2),
        KeyCode::Key3 => intents.push(ExploreIntent::Skill3),
        KeyCode::Key0 => intents.push(ExploreIntent::Pause),
        KeyCode::Back => intents.push(ExploreIntent::BackToMenu),
        _ => {}
    }

    intents
}

pub struct RpgGame {
    state: GameState,
    player: Player,
    data: GameData,
    inventory_state: InventoryState,
    combat: CombatState,
    movement: MovementState,
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
            combat: CombatState::default(),
            movement: MovementState::default(),
            mp_regen_timer: 0,
        }
    }

    fn collect_effects(&self, action: AppAction) -> alloc::vec::Vec<AppEffect> {
        let mut effects = alloc::vec::Vec::new();

        match action {
            AppAction::Tick => match self.state {
                GameState::Loading(_) => effects.push(AppEffect::UpdateLoading),
                GameState::Explore => {
                    effects.push(AppEffect::UpdateMovement);
                    effects.push(AppEffect::UpdateCombat);
                }
                _ => {}
            },
            AppAction::KeyDown(key) => match self.state {
                GameState::Loading(_) => {}
                GameState::Menu(_) => {
                    if let Some(intent) = MenuState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyMenuIntent(intent));
                    }
                }
                GameState::Explore => {
                    for intent in explore_intents_for_key(key) {
                        effects.push(AppEffect::ApplyExploreIntent(intent));
                    }
                }
                GameState::Inventory => {
                    if let Some(intent) = InventoryState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyInventoryIntent(intent));
                    }
                }
                GameState::Stats | GameState::QuestLog => {
                    if matches!(key, KeyCode::Back | KeyCode::Ok) {
                        effects.push(AppEffect::ReturnToExplore);
                    }
                }
                GameState::Dialog(_) => {
                    if let Some(intent) = game::DialogState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyDialogIntent(intent));
                    }
                }
                GameState::Shop(_) => {
                    if let Some(intent) = game::ShopState::intent_for_key(key) {
                        effects.push(AppEffect::ApplyShopIntent(intent));
                    }
                }
                GameState::PauseMenu(_) => {
                    if let Some(intent) = pause_menu_intent_for_key(key) {
                        effects.push(AppEffect::ApplyPauseMenuIntent(intent));
                    }
                }
                GameState::GameOver => {
                    if matches!(key, KeyCode::Ok) {
                        effects.push(AppEffect::ReturnToMenuFromGameOver);
                    }
                }
                GameState::Error(_) => {
                    if matches!(key, KeyCode::Ok) {
                        effects.push(AppEffect::Exit(1));
                    }
                }
            },
            AppAction::KeyUp(key) => effects.push(AppEffect::ReleaseMovementKey(key)),
        }

        effects
    }

    fn apply_effect(&mut self, effect: AppEffect) {
        match effect {
            AppEffect::UpdateLoading => self.update_loading(),
            AppEffect::UpdateMovement => self.update_movement(),
            AppEffect::UpdateCombat => self.update_combat(),
            AppEffect::ApplyMenuIntent(intent) => self.handle_menu_input(intent),
            AppEffect::ApplyExploreIntent(intent) => self.handle_explore_input(intent),
            AppEffect::ApplyInventoryIntent(intent) => self.handle_inventory_input(intent),
            AppEffect::ApplyDialogIntent(intent) => self.handle_dialog_input(intent),
            AppEffect::ApplyShopIntent(intent) => self.handle_shop_input(intent),
            AppEffect::ApplyPauseMenuIntent(intent) => self.handle_pause_menu_input(intent),
            AppEffect::ReturnToExplore => self.state = GameState::Explore,
            AppEffect::ReturnToMenuFromGameOver => {
                self.state = GameState::Menu(MenuState {
                    selected: 0,
                    has_save: has_save_data(),
                });
            }
            AppEffect::ReleaseMovementKey(key) => {
                game::movement::on_key_released(&mut self.movement, key);
            }
            AppEffect::Exit(code) => wipi::kernel::exit(code),
        }
    }

    fn dispatch(&mut self, action: AppAction) {
        let effects = self.collect_effects(action);
        for effect in effects {
            self.apply_effect(effect);
        }
    }

    fn render(&self, fb: &mut Framebuffer) {
        match &self.state {
            GameState::Loading(_) => {}
            GameState::Menu(menu_state) => draw_menu(fb, menu_state),
            GameState::Explore => {
                if let Some(map) = self.current_map() {
                    draw_explore(fb, map, &self.player, &self.combat, &self.data.npcs);
                }
            }
            GameState::Inventory => {
                draw_inventory(fb, &self.player, &self.inventory_state);
            }
            GameState::Stats => draw_stats(fb, &self.player),
            GameState::Dialog(dialog_state) => {
                if let Some(map) = self.current_map() {
                    draw_explore(fb, map, &self.player, &self.combat, &self.data.npcs);
                }
                draw_dialog(fb, dialog_state);
            }
            GameState::Shop(shop_state) => draw_shop(fb, shop_state, &self.player),
            GameState::QuestLog => draw_quest_log(fb, &self.player, &self.data.quests),
            GameState::PauseMenu(selected) => {
                if let Some(map) = self.current_map() {
                    draw_explore(fb, map, &self.player, &self.combat, &self.data.npcs);
                }
                draw_pause_menu(fb, *selected);
            }
            GameState::GameOver => {
                clear_screen(fb);
                let w = fb.width() as i32;
                let h = fb.height() as i32;
                fill_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_DARK_GRAY);
                draw_rect(fb, w / 2 - 40, h / 2 - 20, 80, 40, COLOR_RED);
                draw_text(fb, w / 2 - 35, h / 2 - 8, "GAME OVER", COLOR_RED);
                draw_text(fb, w / 2 - 30, h / 2 + 8, "OK:Menu", COLOR_WHITE);
            }
            GameState::Error(msg) => {
                clear_screen(fb);
                let w = fb.width() as i32;
                let h = fb.height() as i32;
                fill_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_DARK_GRAY);
                draw_rect(fb, 10, h / 2 - 30, w - 20, 60, COLOR_RED);
                draw_text(fb, 16, h / 2 - 20, "ERROR", COLOR_RED);
                draw_text(fb, 16, h / 2 - 4, msg, COLOR_WHITE);
                draw_text(fb, 16, h / 2 + 16, "OK:Exit", COLOR_WHITE);
            }
        }
    }

    fn current_map(&self) -> Option<&data::Map> {
        self.data.find_map(&self.player.current_map_id)
    }

    fn start_new_game(&mut self) {
        self.player = Player::new(String::from("Hero"), "village");

        if let Some(sword) = self.data.find_item("wooden_sword").cloned() {
            let idx = self.player.inventory.len();
            self.player.add_item(sword);
            self.player.equipped_weapon = Some(idx);
        }
        if let Some(armor) = self.data.find_item("cloth").cloned() {
            let idx = self.player.inventory.len();
            self.player.add_item(armor);
            self.player.equipped_armor = Some(idx);
        }
        if let Some(potion) = self.data.find_item("potion").cloned() {
            self.player.add_item(potion.clone());
            self.player.add_item(potion);
        }

        if let Some(map) = self.data.find_map("village") {
            self.player.spawn_at_map(map);
            let _ = game::combat::reduce(
                &mut self.combat,
                game::CombatIntent::SpawnEnemies {
                    map,
                    enemy_data: &self.data.enemies,
                },
            );
        }

        self.state = GameState::Explore;
    }

    fn continue_game(&mut self) {
        self.player = Player::new(String::from("Hero"), "village");

        match load_game(&mut self.player) {
            Ok(true) => {
                if self.data.find_map(&self.player.current_map_id).is_none() {
                    self.player.current_map_id = String::from("village");
                }
                if let Some(map) = self.data.find_map(&self.player.current_map_id) {
                    if map.get_tile(self.player.x, self.player.y) == data::Tile::Wall
                        || self.player.x >= map.width
                        || self.player.y >= map.height
                    {
                        self.player.spawn_at_map(map);
                    }
                    let _ = game::combat::reduce(
                        &mut self.combat,
                        game::CombatIntent::SpawnEnemies {
                            map,
                            enemy_data: &self.data.enemies,
                        },
                    );
                }
                self.state = GameState::Explore;
            }
            Ok(false) => {
                self.start_new_game();
            }
            Err(_) => {
                self.start_new_game();
            }
        }
    }

    fn update_loading(&mut self) {
        let GameState::Loading(step) = self.state else {
            return;
        };

        self.draw_loading(step);

        match self.data.load_step(step) {
            Ok(true) => {
                self.state = GameState::Menu(MenuState {
                    selected: 0,
                    has_save: has_save_data(),
                });
            }
            Ok(false) => {
                self.state = GameState::Loading(step + 1);
            }
            Err(e) => {
                self.state = GameState::Error(format!("Load error: {}", e));
            }
        }
    }

    fn draw_loading(&self, step: usize) {
        let mut fb = Framebuffer::screen_framebuffer();
        clear_screen(&mut fb);

        let w = fb.width() as i32;
        let h = fb.height() as i32;

        draw_text(&mut fb, w / 2 - 30, h / 2 - 30, "Loading...", COLOR_WHITE);

        let label = GameData::LOAD_LABELS[step];
        draw_text(&mut fb, w / 2 - 30, h / 2 - 10, label, COLOR_CYAN);

        let bar_w = 120;
        let bar_h = 8;
        let bar_x = w / 2 - bar_w / 2;
        let bar_y = h / 2 + 10;

        draw_rect(&mut fb, bar_x, bar_y, bar_w, bar_h, COLOR_WHITE);
        let progress = (((step + 1) * bar_w as usize / GameData::LOAD_STEPS) as i32).min(bar_w);
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

        let moved = game::movement::tick(
            &mut self.movement,
            &mut self.player,
            map,
            &self.combat,
            &self.data.npcs,
        );

        if moved {
            self.check_tile_events();
        }
    }

    fn update_combat(&mut self) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }

        let _ = game::player::reduce(&mut self.player, game::PlayerIntent::UpdateCooldowns);

        self.mp_regen_timer += 1;
        if self.mp_regen_timer >= MP_REGEN_INTERVAL {
            self.mp_regen_timer = 0;
            let _ = game::player::reduce(&mut self.player, game::PlayerIntent::RecoverMp(1));
        }

        let player_x = self.player.x;
        let player_y = self.player.y;
        let player_def = self.player.total_def();
        let map_id = self.player.current_map_id.clone();

        if let Some(map) = self.data.find_map(&map_id) {
            let game::CombatEvent::Tick(result) = game::combat::reduce(
                &mut self.combat,
                game::CombatIntent::Tick {
                    player_x,
                    player_y,
                    player_def,
                    map,
                    enemy_data: &self.data.enemies,
                },
            ) else {
                return;
            };

            if result.damage_taken > 0
                && matches!(
                    game::player::reduce(
                        &mut self.player,
                        game::PlayerIntent::TakeDamage(result.damage_taken),
                    ),
                    game::PlayerEvent::Died
                )
            {
                self.state = GameState::GameOver;
            }
        }
    }

    fn use_skill(&mut self, slot: usize, skill: &Skill) {
        if !game::player::can_use_skill(&self.player, slot, skill.mp_cost) {
            return;
        }

        let game::CombatEvent::Skill(result) = game::combat::reduce(
            &mut self.combat,
            game::CombatIntent::UseSkill {
                skill,
                player_x: self.player.x,
                player_y: self.player.y,
                player_atk: self.player.total_atk(),
                facing: self.player.facing,
            },
        ) else {
            return;
        };

        let _ = game::player::reduce(
            &mut self.player,
            game::PlayerIntent::UseSkill {
                slot,
                mp_cost: skill.mp_cost,
                cooldown: skill.cooldown,
            },
        );

        for effect in &result.player_effects {
            match effect {
                PlayerEffect::Heal(amount) => {
                    let _ =
                        game::player::reduce(&mut self.player, game::PlayerIntent::Heal(*amount));
                }
            }
        }

        for kill in result.kills {
            self.player.stats.add_exp(kill.exp);
            self.player.stats.gold += kill.gold;
            game::quest::on_enemy_killed(&mut self.player, &self.data, &kill.enemy_id);
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
        let Some(map) = self.data.find_map(target_id) else {
            return;
        };

        self.player.current_map_id = map.id.clone();
        if let Some((x, y)) = map.find_player_start() {
            self.player.x = x;
            self.player.y = y;
        }
        let _ = game::combat::reduce(
            &mut self.combat,
            game::CombatIntent::SpawnEnemies {
                map,
                enemy_data: &self.data.enemies,
            },
        );
    }

    fn try_interact_with_npc(&mut self) {
        let facing = self.player.facing;
        if let Some(new_state) = game::npc::reduce(
            &mut self.player,
            &self.data,
            game::NpcIntent::Interact { facing },
        ) {
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

        if let Some(new_state) = game::npc::reduce(
            &mut self.player,
            &self.data,
            game::NpcIntent::ProcessDialogAction { action: &action },
        ) {
            self.state = new_state;
        }
    }

    fn handle_menu_input(&mut self, intent: MenuIntent) {
        let GameState::Menu(ref mut menu) = self.state else {
            return;
        };

        match intent {
            MenuIntent::MoveUp => menu.move_up(),
            MenuIntent::MoveDown => menu.move_down(),
            MenuIntent::Select => {
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
        }
    }

    fn handle_explore_input(&mut self, intent: ExploreIntent) {
        match intent {
            ExploreIntent::MoveDirection(key) => {
                game::movement::on_direction_pressed(&mut self.movement, key);
            }
            ExploreIntent::TryNpcInteract => {
                self.try_interact_with_npc();
            }
            ExploreIntent::Attack => {
                if matches!(self.state, GameState::Dialog(_)) {
                    return;
                }
                if let game::CombatEvent::Attack(Some(reward)) = game::combat::reduce(
                    &mut self.combat,
                    game::CombatIntent::PlayerAttack {
                        player_x: self.player.x,
                        player_y: self.player.y,
                        player_atk: self.player.total_atk(),
                        facing: self.player.facing,
                    },
                ) {
                    self.player.stats.add_exp(reward.exp);
                    self.player.stats.gold += reward.gold;
                    game::quest::on_enemy_killed(&mut self.player, &self.data, &reward.enemy_id);
                }
            }
            ExploreIntent::Skill1 => self.use_skill(0, &Skill::FIREBALL),
            ExploreIntent::Skill2 => self.use_skill(1, &Skill::HEAL),
            ExploreIntent::Skill3 => self.use_skill(2, &Skill::SPIN_ATTACK),
            ExploreIntent::Pause => self.state = GameState::PauseMenu(0),
            ExploreIntent::BackToMenu => {
                let _ = save_game(&self.player);
                self.state = GameState::Menu(MenuState {
                    selected: 0,
                    has_save: has_save_data(),
                });
            }
        }
    }

    fn handle_inventory_input(&mut self, intent: InventoryIntent) {
        match intent {
            InventoryIntent::MoveUp => self.inventory_state.move_up(),
            InventoryIntent::MoveDown => {
                let fb = Framebuffer::screen_framebuffer();
                let visible = ((fb.height() as i32 - 50) / 14).max(1) as usize;
                self.inventory_state
                    .move_down(self.player.inventory.len(), visible);
            }
            InventoryIntent::UseSelected => {
                let _ = game::player::reduce(
                    &mut self.player,
                    game::PlayerIntent::UseItem {
                        index: self.inventory_state.selected,
                    },
                );
            }
            InventoryIntent::Back => self.state = GameState::Explore,
        }
    }

    fn handle_dialog_input(&mut self, intent: DialogIntent) {
        match intent {
            DialogIntent::Confirm => {
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
            DialogIntent::Back => self.state = GameState::Explore,
        }
    }

    fn handle_shop_input(&mut self, intent: ShopIntent) {
        const VISIBLE_ITEMS: usize = 8;

        let GameState::Shop(ref mut state) = self.state else {
            return;
        };

        match state.mode {
            ShopMode::Select => match intent {
                ShopIntent::MoveUp => state.move_up(),
                ShopIntent::MoveDown => state.move_down(2, 2),
                ShopIntent::Confirm => {
                    state.mode = if state.selected == 0 {
                        ShopMode::Buy
                    } else {
                        ShopMode::Sell
                    };
                    state.reset_selection();
                }
                ShopIntent::Back => self.state = GameState::Explore,
            },
            ShopMode::Buy => match intent {
                ShopIntent::MoveUp => state.move_up(),
                ShopIntent::MoveDown => state.move_down(state.items.len(), VISIBLE_ITEMS),
                ShopIntent::Confirm => {
                    if let Some(item) = state.items.get(state.selected).cloned()
                        && self.player.stats.gold >= item.price
                    {
                        self.player.stats.gold -= item.price;
                        self.player.add_item(item);
                    }
                }
                ShopIntent::Back => {
                    state.mode = ShopMode::Select;
                    state.reset_selection();
                }
            },
            ShopMode::Sell => match intent {
                ShopIntent::MoveUp => state.move_up(),
                ShopIntent::MoveDown => state.move_down(self.player.inventory.len(), VISIBLE_ITEMS),
                ShopIntent::Confirm => {
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
                ShopIntent::Back => {
                    state.mode = ShopMode::Select;
                    state.reset_selection();
                }
            },
        }
    }

    fn handle_pause_menu_input(&mut self, intent: PauseMenuIntent) {
        let GameState::PauseMenu(ref mut selected) = self.state else {
            return;
        };

        match intent {
            PauseMenuIntent::MoveUp if *selected > 0 => *selected -= 1,
            PauseMenuIntent::MoveDown if *selected < 3 => *selected += 1,
            PauseMenuIntent::Select => match *selected {
                0 => {
                    self.inventory_state = InventoryState::default();
                    self.state = GameState::Inventory;
                }
                1 => self.state = GameState::Stats,
                2 => self.state = GameState::QuestLog,
                3 => {
                    let _ = save_game(&self.player);
                    self.state = GameState::Explore;
                }
                _ => {}
            },
            PauseMenuIntent::Back => self.state = GameState::Explore,
            _ => {}
        }
    }
}

impl App for RpgGame {
    fn on_paint(&mut self) {
        self.dispatch(AppAction::Tick);

        let mut fb = Framebuffer::screen_framebuffer();
        self.render(&mut fb);

        repaint(0, 0, 0, fb.width() as i32, fb.height() as i32);
    }

    fn on_keydown(&mut self, key: KeyCode) {
        self.dispatch(AppAction::KeyDown(key));
    }

    fn on_keyup(&mut self, key: KeyCode) {
        self.dispatch(AppAction::KeyUp(key));
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}
