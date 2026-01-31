#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::string::String;
use alloc::vec::Vec;
use core::str;

use wipi::{
    app::App, event::KeyCode, framebuffer::Framebuffer, graphics::repaint, resource::Resource,
    wipi_main,
};

use data::{
    Dialog, Enemy, Item, Map, Npc, Quest, Shop, parse_dialogs, parse_enemies, parse_items,
    parse_maps, parse_npcs, parse_quests, parse_shops,
};
use game::{
    COLOR_DARK_GRAY, COLOR_RED, COLOR_WHITE, CombatSystem, DialogState, GameState, InventoryState,
    MenuState, Player, ShopMode, ShopState, TileEvent, check_tile_event, clear_screen, draw_dialog,
    draw_explore, draw_inventory, draw_menu, draw_quest_log, draw_rect, draw_shop, draw_stats,
    draw_text, fill_rect, has_save_data, load_game, save_game,
};

pub struct RpgGame {
    state: GameState,
    player: Player,
    items: Vec<Item>,
    enemies: Vec<Enemy>,
    maps: Vec<Map>,
    npcs: Vec<Npc>,
    dialogs: Vec<Dialog>,
    quests: Vec<Quest>,
    shops: Vec<Shop>,
    inventory_state: InventoryState,
    combat: CombatSystem,
    pressed_direction: Option<KeyCode>,
    move_cooldown: u32,
}

use wipi::WIPICError;

#[derive(Debug)]
pub enum GameError {
    WIPIC(WIPICError),
    Utf8(core::str::Utf8Error),
}

impl From<WIPICError> for GameError {
    fn from(e: WIPICError) -> Self {
        GameError::WIPIC(e)
    }
}

impl From<core::str::Utf8Error> for GameError {
    fn from(e: core::str::Utf8Error) -> Self {
        GameError::Utf8(e)
    }
}

impl RpgGame {
    pub fn new() -> Result<Self, GameError> {
        let items = Self::load_items()?;
        let enemies = Self::load_enemies()?;
        let maps = Self::load_maps()?;
        let npcs = Self::load_npcs()?;
        let dialogs = Self::load_dialogs()?;
        let quests = Self::load_quests()?;
        let shops = Self::load_shops()?;

        let menu_state = MenuState {
            selected: 0,
            has_save: has_save_data(),
        };

        Ok(Self {
            state: GameState::Menu(menu_state),
            player: Player::new(String::from("Hero"), "village"),
            items,
            enemies,
            maps,
            npcs,
            dialogs,
            quests,
            shops,
            inventory_state: InventoryState::default(),
            combat: CombatSystem::new(),
            pressed_direction: None,
            move_cooldown: 0,
        })
    }

    fn load_resource<T>(path: &str, parser: fn(&str) -> T) -> Result<T, GameError> {
        let resource = Resource::new(path)?;
        let text = str::from_utf8(resource.read())?;
        Ok(parser(text))
    }

    fn load_items() -> Result<Vec<Item>, GameError> {
        Self::load_resource("data/items.dat", parse_items)
    }

    fn load_enemies() -> Result<Vec<Enemy>, GameError> {
        Self::load_resource("data/enemies.dat", parse_enemies)
    }

    fn load_maps() -> Result<Vec<Map>, GameError> {
        Self::load_resource("data/maps.dat", parse_maps)
    }

    fn load_npcs() -> Result<Vec<Npc>, GameError> {
        Self::load_resource("data/npcs.dat", parse_npcs)
    }

    fn load_dialogs() -> Result<Vec<Dialog>, GameError> {
        Self::load_resource("data/dialogs.dat", parse_dialogs)
    }

    fn load_quests() -> Result<Vec<Quest>, GameError> {
        Self::load_resource("data/quests.dat", parse_quests)
    }

    fn load_shops() -> Result<Vec<Shop>, GameError> {
        Self::load_resource("data/shops.dat", parse_shops)
    }

    fn current_map(&self) -> Option<&Map> {
        self.maps
            .iter()
            .find(|m| m.id == self.player.current_map_id)
    }

    fn start_new_game(&mut self) {
        self.player = Player::new(String::from("Hero"), "village");

        if let Some(sword) = self.items.iter().find(|i| i.id == "wooden_sword").cloned() {
            self.player.add_item(sword);
            self.player.equipped_weapon = Some(0);
        }
        if let Some(armor) = self.items.iter().find(|i| i.id == "cloth").cloned() {
            self.player.add_item(armor);
            self.player.equipped_armor = Some(1);
        }
        if let Some(potion) = self.items.iter().find(|i| i.id == "potion").cloned() {
            self.player.add_item(potion.clone());
            self.player.add_item(potion);
        }

        if let Some(map) = self.maps.iter().find(|m| m.id == "village") {
            self.player.spawn_at_map(map);
            self.combat.spawn_enemies(map, &self.enemies);
        }

        self.state = GameState::Explore;
    }

    fn continue_game(&mut self) {
        self.player = Player::new(String::from("Hero"), "village");

        if load_game(&mut self.player) {
            if let Some(map) = self
                .maps
                .iter()
                .find(|m| m.id == self.player.current_map_id)
            {
                self.combat.spawn_enemies(map, &self.enemies);
            }
            self.state = GameState::Explore;
        } else {
            self.start_new_game();
        }
    }

    fn handle_menu_input(&mut self, key: KeyCode) {
        if let GameState::Menu(ref mut menu) = self.state {
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
                        MenuAction::Exit => {
                            wipi::kernel::exit(0);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_explore_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                self.pressed_direction = Some(key);
                self.move_cooldown = 0;
            }
            KeyCode::Ok => {
                self.try_interact_with_npc();
                if matches!(self.state, GameState::Dialog(_)) {
                    return;
                }

                let reward = self.combat.player_attack(
                    self.player.x,
                    self.player.y,
                    self.player.total_atk(),
                    self.player.facing,
                );
                if let Some(reward) = reward {
                    self.player.stats.add_exp(reward.exp);
                    self.player.stats.gold += reward.gold;
                    self.update_kill_quest(&reward.enemy_id);
                }
            }
            KeyCode::Key1 => {
                self.inventory_state = InventoryState::default();
                self.state = GameState::Inventory;
            }
            KeyCode::Key2 => {
                self.state = GameState::Stats;
            }
            KeyCode::Key3 => {
                self.state = GameState::QuestLog;
            }
            KeyCode::Key0 => {
                save_game(&self.player);
            }
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

    fn try_move(&mut self, key: KeyCode) {
        let (dx, dy) = match key {
            KeyCode::Up => (0, -1),
            KeyCode::Down => (0, 1),
            KeyCode::Left => (-1, 0),
            KeyCode::Right => (1, 0),
            _ => return,
        };

        self.player.set_facing(dx, dy);

        if let Some(map) = self.current_map()
            && self.player.can_move(map, dx, dy)
        {
            let new_x = self.player.x.wrapping_add_signed(dx as isize);
            let new_y = self.player.y.wrapping_add_signed(dy as isize);
            if !self.combat.enemy_at(new_x, new_y) {
                self.player.move_by(dx, dy);
                self.check_tile_events();
            }
        }
    }

    fn update_movement(&mut self) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }

        if self.move_cooldown > 0 {
            self.move_cooldown -= 1;
            return;
        }

        if let Some(key) = self.pressed_direction {
            self.try_move(key);
            self.move_cooldown = 5;
        }
    }

    fn update_combat(&mut self) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }

        if let Some(map) = self.current_map().cloned() {
            let result = self.combat.update(
                self.player.x,
                self.player.y,
                self.player.total_def(),
                &map,
                &self.enemies,
            );

            if result.damage_taken > 0 {
                self.player.stats.take_damage(result.damage_taken);

                if self.player.stats.is_dead() {
                    self.state = GameState::GameOver;
                }
            }
        }
    }

    fn check_tile_events(&mut self) {
        let event = if let Some(map) = self.current_map() {
            check_tile_event(map, &self.player)
        } else {
            None
        };

        if let Some(event) = event {
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
                        if let Some(potion) = self.items.iter().find(|i| i.id == "potion").cloned()
                        {
                            self.player.add_item(potion);
                        }
                        self.player
                            .open_treasure(&map_id, self.player.x, self.player.y);
                    }
                }
            }
        }
    }

    fn change_map(&mut self, target_id: &str) {
        let map = self.maps.iter().find(|m| m.id == target_id).cloned();
        if let Some(map) = map {
            self.player.current_map_id = map.id.clone();
            if let Some((x, y)) = map.find_player_start() {
                self.player.x = x;
                self.player.y = y;
            }
            self.combat.spawn_enemies(&map, &self.enemies);
        }
    }

    fn find_npc_at(&self, x: usize, y: usize) -> Option<&Npc> {
        self.npcs
            .iter()
            .find(|npc| npc.map_id == self.player.current_map_id && npc.x == x && npc.y == y)
    }

    fn try_interact_with_npc(&mut self) {
        use data::NpcType;

        let (target_x, target_y) = match self.player.facing {
            game::Direction::Up => (self.player.x, self.player.y.saturating_sub(1)),
            game::Direction::Down => (self.player.x, self.player.y.saturating_add(1)),
            game::Direction::Left => (self.player.x.saturating_sub(1), self.player.y),
            game::Direction::Right => (self.player.x.saturating_add(1), self.player.y),
        };

        let Some(npc) = self.find_npc_at(target_x, target_y).cloned() else {
            return;
        };

        match npc.npc_type {
            NpcType::Healer => {
                self.player.stats.current_hp = self.player.stats.max_hp;
                self.player.stats.current_mp = self.player.stats.max_mp;

                if let Some(dialog) = self.dialogs.iter().find(|d| d.id == npc.dialog_id).cloned() {
                    let filtered_lines = self.filter_dialog_lines(&dialog);
                    if !filtered_lines.lines.is_empty() {
                        self.state =
                            GameState::Dialog(DialogState::new(npc.name.clone(), &filtered_lines));
                        return;
                    }
                }
            }
            NpcType::ShopKeeper => {
                let shop = npc
                    .shop_id
                    .as_ref()
                    .and_then(|sid| self.shops.iter().find(|s| s.id == *sid))
                    .or_else(|| self.shops.first())
                    .cloned();
                if let Some(shop) = shop {
                    let shop_items: Vec<_> = shop
                        .items
                        .iter()
                        .filter_map(|item_id| self.items.iter().find(|i| i.id == *item_id).cloned())
                        .collect();
                    self.state = GameState::Shop(ShopState::new(shop, shop_items));
                    return;
                }
            }
            NpcType::QuestGiver | NpcType::Villager => {}
        }

        if let Some(dialog) = self.dialogs.iter().find(|d| d.id == npc.dialog_id).cloned() {
            let filtered_lines = self.filter_dialog_lines(&dialog);
            if !filtered_lines.lines.is_empty() {
                self.state = GameState::Dialog(DialogState::new(npc.name.clone(), &filtered_lines));
            }
        }
    }

    fn filter_dialog_lines(&self, dialog: &Dialog) -> Dialog {
        use data::DialogCondition;

        let mut filtered = Vec::new();

        for line in &dialog.lines {
            let should_show = match &line.condition {
                None => true,
                Some(DialogCondition::HasQuest(id)) => self.player.has_quest(id),
                Some(DialogCondition::QuestComplete(id)) => self.player.is_quest_complete(id),
                Some(DialogCondition::HasItem(id)) => self.player.has_item(id),
                Some(DialogCondition::HasGold(amount)) => self.player.stats.gold >= *amount,
            };

            if should_show {
                filtered.push(line.clone());
            }
        }

        Dialog {
            id: dialog.id.clone(),
            lines: filtered,
        }
    }

    fn process_dialog_action(&mut self) {
        if let GameState::Dialog(ref state) = self.state
            && let Some(action) = state.current_action().cloned()
        {
            use data::DialogAction;
            match action {
                DialogAction::GiveQuest(id) => {
                    self.player.add_quest(&id);
                }
                DialogAction::CompleteQuest(id) => {
                    if let Some(quest) = self.quests.iter().find(|q| q.id == id).cloned() {
                        self.player.stats.add_exp(quest.reward_exp);
                        self.player.stats.gold += quest.reward_gold;
                        if let Some(item_id) = &quest.reward_item
                            && let Some(item) =
                                self.items.iter().find(|i| i.id == *item_id).cloned()
                        {
                            self.player.add_item(item);
                        }
                        self.player.complete_quest(&id);
                    }
                }
                DialogAction::GiveItem(id) => {
                    if let Some(item) = self.items.iter().find(|i| i.id == id).cloned() {
                        self.player.add_item(item);
                    }
                }
                DialogAction::TakeItem(id) => {
                    self.player.remove_item(&id);
                }
                DialogAction::GiveGold(amount) => {
                    self.player.stats.gold += amount;
                }
                DialogAction::TakeGold(amount) => {
                    self.player.stats.gold = (self.player.stats.gold - amount).max(0);
                }
                DialogAction::OpenShop(id) => {
                    if let Some(shop) = self.shops.iter().find(|s| s.id == id).cloned() {
                        let shop_items: Vec<_> = shop
                            .items
                            .iter()
                            .filter_map(|item_id| {
                                self.items.iter().find(|i| i.id == *item_id).cloned()
                            })
                            .collect();
                        self.state = GameState::Shop(ShopState::new(shop, shop_items));
                    }
                }
                DialogAction::Heal => {
                    self.player.stats.current_hp = self.player.stats.max_hp;
                    self.player.stats.current_mp = self.player.stats.max_mp;
                }
            }
        }
    }

    fn handle_inventory_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => self.inventory_state.move_up(),
            KeyCode::Down => self.inventory_state.move_down(self.player.inventory.len()),
            KeyCode::Ok => {
                let idx = self.inventory_state.selected;
                self.player.use_item(idx);
            }
            KeyCode::Back => {
                self.state = GameState::Explore;
            }
            _ => {}
        }
    }

    fn handle_stats_input(&mut self, key: KeyCode) {
        if matches!(key, KeyCode::Back | KeyCode::Ok) {
            self.state = GameState::Explore;
        }
    }

    fn handle_gameover_input(&mut self, key: KeyCode) {
        if matches!(key, KeyCode::Ok) {
            self.state = GameState::Menu(MenuState {
                selected: 0,
                has_save: has_save_data(),
            });
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
            KeyCode::Back => {
                self.state = GameState::Explore;
            }
            _ => {}
        }
    }

    fn handle_shop_input(&mut self, key: KeyCode) {
        const SHOP_VISIBLE_ITEMS: usize = 8;

        if let GameState::Shop(ref mut state) = self.state {
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
                    KeyCode::Back => {
                        self.state = GameState::Explore;
                    }
                    _ => {}
                },
                ShopMode::Buy => match key {
                    KeyCode::Up => state.move_up(),
                    KeyCode::Down => state.move_down(state.items.len(), SHOP_VISIBLE_ITEMS),
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
                    KeyCode::Down => {
                        state.move_down(self.player.inventory.len(), SHOP_VISIBLE_ITEMS)
                    }
                    KeyCode::Ok => {
                        if let Some(item) = self.player.remove_item_at(state.selected) {
                            let sell_price = item.price / 2;
                            self.player.stats.gold += sell_price;
                            let inv_len = self.player.inventory.len();
                            if state.selected >= inv_len && state.selected > 0 {
                                state.selected -= 1;
                            }
                            if state.scroll > 0
                                && state.scroll >= inv_len.saturating_sub(SHOP_VISIBLE_ITEMS - 1)
                            {
                                state.scroll = inv_len.saturating_sub(SHOP_VISIBLE_ITEMS);
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
    }

    fn handle_quest_input(&mut self, key: KeyCode) {
        if matches!(key, KeyCode::Back | KeyCode::Ok) {
            self.state = GameState::Explore;
        }
    }

    fn update_kill_quest(&mut self, killed_enemy_id: &str) {
        for progress in &mut self.player.quests {
            if progress.completed || progress.rewarded {
                continue;
            }
            if let Some(quest) = self.quests.iter().find(|q| q.id == progress.quest_id)
                && quest.quest_type == data::QuestType::Kill
                && quest.target_id == killed_enemy_id
            {
                progress.current_count += 1;
                if progress.current_count >= quest.target_count {
                    progress.completed = true;
                }
            }
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
        self.update_movement();
        self.update_combat();

        let mut fb = Framebuffer::screen_framebuffer();

        match &self.state {
            GameState::Menu(menu_state) => {
                draw_menu(&mut fb, menu_state);
            }
            GameState::Explore => {
                if let Some(map) = self.current_map() {
                    draw_explore(&mut fb, map, &self.player, &self.combat, &self.npcs);
                }
            }
            GameState::Inventory => {
                draw_inventory(&mut fb, &self.player, &self.inventory_state);
            }
            GameState::Stats => {
                draw_stats(&mut fb, &self.player);
            }
            GameState::Dialog(dialog_state) => {
                if let Some(map) = self.current_map() {
                    draw_explore(&mut fb, map, &self.player, &self.combat, &self.npcs);
                }
                draw_dialog(&mut fb, dialog_state);
            }
            GameState::Shop(shop_state) => {
                draw_shop(&mut fb, shop_state, &self.player);
            }
            GameState::QuestLog => {
                draw_quest_log(&mut fb, &self.player, &self.quests);
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
            GameState::Menu(_) => self.handle_menu_input(key),
            GameState::Explore => self.handle_explore_input(key),
            GameState::Inventory => self.handle_inventory_input(key),
            GameState::Stats => self.handle_stats_input(key),
            GameState::Dialog(_) => self.handle_dialog_input(key),
            GameState::Shop(_) => self.handle_shop_input(key),
            GameState::QuestLog => self.handle_quest_input(key),
            GameState::GameOver => self.handle_gameover_input(key),
        }
    }

    fn on_keyup(&mut self, key: KeyCode) {
        if self.pressed_direction == Some(key) {
            self.pressed_direction = None;
        }
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    match RpgGame::new() {
        Ok(game) => game,
        Err(e) => panic!("Failed to load game: {:?}", e),
    }
}
