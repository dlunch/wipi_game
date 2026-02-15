#![cfg_attr(not(test), no_main)]
#![no_std]
extern crate alloc;

mod data;
mod game;

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::time::Duration;

use wipi::app::App;
use wipi::event::KeyCode;
use wipi::framebuffer::Framebuffer;
use wipi::graphics::repaint;
use wipi::timer::Timer;
use wipi::wipi_main;

use crate::data::{DialogAction, Direction};
use crate::game::{
    AppAction, AppIntent, DialogIntent, ExploreIntent, GameData, GameState, InventoryIntent,
    MenuAction, MenuEvent, MenuIntent, MenuState, PauseMenuIntent, RenderState, SessionState,
    ShopIntent, UiState, build_render_state, has_save_data, render,
};

#[derive(Debug, Clone, Copy)]
enum AppEvent {
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    Menu(MenuIntent),
    Explore(ExploreIntent),
    Inventory(InventoryIntent),
    Dialog(DialogIntent),
    Shop(ShopIntent),
    PauseMenu(PauseMenuIntent),
    ReturnToExplore,
    ReturnToMenuFromGameOver,
    ReleaseMovementKey(KeyCode),
    Exit(i32),
}

struct GameInner {
    state: GameState,
    data: Rc<GameData>,
    session: Option<SessionState>,
    ui: UiState,
}

fn direction_for_key(key: KeyCode) -> Option<Direction> {
    match key {
        KeyCode::Up => Some(Direction::Up),
        KeyCode::Down => Some(Direction::Down),
        KeyCode::Left => Some(Direction::Left),
        KeyCode::Right => Some(Direction::Right),
        _ => None,
    }
}

impl GameInner {
    fn update(&mut self) {
        self.dispatch(AppAction::Tick);
    }

    fn collect_intents(&self, action: AppAction) -> Vec<AppIntent> {
        let mut intents = Vec::new();

        match action {
            AppAction::Tick => match self.state {
                GameState::Loading(_) => intents.push(AppIntent::UpdateLoading),
                GameState::Explore => {
                    intents.push(AppIntent::UpdateMovement);
                    intents.push(AppIntent::UpdateCombat);
                }
                _ => {}
            },
            AppAction::KeyDown(key) => match self.state {
                GameState::Loading(_) => {}
                GameState::Menu => {
                    if let Some(intent) = MenuIntent::intent_for_key(key) {
                        intents.push(AppIntent::Menu(intent));
                    }
                }
                GameState::Explore => {
                    let facing = self
                        .session
                        .as_ref()
                        .map(|s| s.player.facing)
                        .unwrap_or(crate::data::Direction::Down);
                    for intent in ExploreIntent::intent_for_key(
                        key,
                        facing,
                        self.ui.explore.ok_action,
                        self.ui.explore.key_actions,
                    ) {
                        intents.push(AppIntent::Explore(intent));
                    }
                }
                GameState::Inventory => {
                    if let Some(intent) = InventoryIntent::intent_for_key(key) {
                        intents.push(AppIntent::Inventory(intent));
                    }
                }
                GameState::Stats | GameState::QuestLog => {
                    if matches!(key, KeyCode::Back | KeyCode::Ok) {
                        intents.push(AppIntent::ReturnToExplore);
                    }
                }
                GameState::Dialog => {
                    if let Some(intent) = DialogIntent::intent_for_key(key) {
                        intents.push(AppIntent::Dialog(intent));
                    }
                }
                GameState::Shop => {
                    if let Some(intent) = ShopIntent::intent_for_key(key) {
                        intents.push(AppIntent::Shop(intent));
                    }
                }
                GameState::PauseMenu => {
                    if let Some(intent) = PauseMenuIntent::intent_for_key(key) {
                        intents.push(AppIntent::PauseMenu(intent));
                    }
                }
                GameState::GameOver => {
                    if matches!(key, KeyCode::Ok) {
                        intents.push(AppIntent::ReturnToMenuFromGameOver);
                    }
                }
                GameState::Error(_) => {
                    if matches!(key, KeyCode::Ok) {
                        intents.push(AppIntent::Exit(1));
                    }
                }
            },
            AppAction::KeyUp(key) => {
                if matches!(self.state, GameState::Explore)
                    && self.session.is_some()
                    && let Some(direction) = direction_for_key(key)
                {
                    intents.push(AppIntent::ReleaseMovementKey(match direction {
                        Direction::Up => KeyCode::Up,
                        Direction::Down => KeyCode::Down,
                        Direction::Left => KeyCode::Left,
                        Direction::Right => KeyCode::Right,
                    }));
                }
            }
        }

        intents
    }

    fn apply_update_loading(&mut self) {
        let GameState::Loading(step) = self.state else {
            return;
        };

        let load_result = game::lifecycle::load_step(&mut self.data, step);
        match game::lifecycle::reduce_loading(step, load_result) {
            game::LoadingEvent::Advance(step) => self.state = GameState::Loading(step),
            game::LoadingEvent::Loaded => {
                self.state = GameState::Menu;
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            game::LoadingEvent::Error(msg) => self.state = GameState::Error(msg),
        }
    }

    fn apply_update_movement(&mut self) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        if !matches!(self.state, GameState::Explore) {
            return;
        }

        let map = self.data.find_map(&s.player.current_map_id);
        let enemy_positions: Vec<(usize, usize)> = s
            .combat
            .enemies
            .iter()
            .filter(|enemy| !enemy.is_dead())
            .map(|enemy| (enemy.x, enemy.y))
            .collect();
        let npc_positions: Vec<(usize, usize)> = self
            .data
            .npcs
            .iter()
            .filter(|npc| npc.map_id == s.player.current_map_id)
            .map(|npc| (npc.x, npc.y))
            .collect();

        let event = game::movement::reduce_tick(
            &s.movement,
            &s.player,
            map,
            &enemy_positions,
            &npc_positions,
        );
        let moved = game::movement::apply_tick(&mut s.movement, &mut s.player, event);
        if moved && let Some(tile_event) = game::explore::reduce_tile_event(&s.player, &self.data) {
            let apply_event =
                game::explore::apply_tile_event(&mut s.player, &self.data, tile_event);
            if matches!(apply_event, game::explore::TileApplyEvent::MapChanged)
                && let Some(map) = self.data.find_map(&s.player.current_map_id)
            {
                game::combat::spawn_for_map(&mut s.combat, map, &self.data.enemies);
            }
        }
    }

    fn apply_update_combat(&mut self) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        if !matches!(self.state, GameState::Explore) {
            return;
        }

        let Some(map) = self.data.find_map(&s.player.current_map_id) else {
            return;
        };
        let combat_event = game::combat::apply(
            &mut s.combat,
            game::CombatIntent::Tick {
                player_x: s.player.x,
                player_y: s.player.y,
                player_def: s.player.total_def(),
                skill_cooldowns: s.skill_cooldowns,
                mp_regen_timer: s.mp_regen_timer,
                map,
                enemy_data: &self.data.enemies,
            },
        );
        if let game::CombatEvent::Tick(result) = combat_event {
            let damage_taken = game::combat::apply_tick(
                &mut s.player,
                &mut s.skill_cooldowns,
                &mut s.mp_regen_timer,
                result,
            );
            if damage_taken > 0
                && matches!(
                    game::player::apply(
                        &mut s.player,
                        game::PlayerIntent::TakeDamage(damage_taken)
                    ),
                    game::PlayerEvent::Died
                )
            {
                self.state = GameState::GameOver;
            }
        }
    }

    fn spawn_current_map_enemies(&mut self) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        if let Some(map) = self.data.find_map(&s.player.current_map_id) {
            game::combat::spawn_for_map(&mut s.combat, map, &self.data.enemies);
        }
    }

    fn dialog_state_from_intro(
        &self,
        intro: Option<game::lifecycle::IntroDialogSpec>,
    ) -> Option<game::DialogState> {
        let spec = intro?;
        let dialog = self.data.find_dialog(&spec.dialog_id)?;
        Some(game::DialogState::from_dialog(spec.npc_name, dialog))
    }

    fn enter_session(
        &mut self,
        state: GameState,
        session: SessionState,
        intro: Option<game::lifecycle::IntroDialogSpec>,
    ) {
        self.state = state;
        self.session = Some(session);
        self.spawn_current_map_enemies();
        self.ui = UiState::default();
        self.ui.dialog.set(self.dialog_state_from_intro(intro));
    }

    fn open_shop_by_id(&mut self, shop_id: &str) -> bool {
        let Some(shop) = self.data.find_shop(shop_id).cloned() else {
            return false;
        };
        let shop_items = self.data.get_shop_items(&shop);
        self.ui.shop.open(game::ShopState::new(shop, shop_items));
        self.state = GameState::Shop;
        true
    }

    fn apply_explore_action(s: &mut SessionState, data: &GameData, action: game::ExploreAction) {
        if let Some((slot, skill)) = action.skill() {
            if !game::player::can_use_skill(&s.player, &s.skill_cooldowns, slot, skill.mp_cost) {
                return;
            }

            let combat_event = game::combat::apply(
                &mut s.combat,
                game::CombatIntent::UseSkill {
                    skill,
                    player_x: s.player.x,
                    player_y: s.player.y,
                    player_atk: s.player.total_atk(),
                    facing: s.player.facing,
                },
            );
            let game::CombatEvent::Skill(result) = combat_event else {
                return;
            };

            s.skill_cooldowns[slot] = skill.cooldown;
            s.player.stats.current_mp = (s.player.stats.current_mp - skill.mp_cost).max(0);

            for effect in &result.player_effects {
                match effect {
                    game::PlayerEffect::Heal(amount) => {
                        let _ =
                            game::player::apply(&mut s.player, game::PlayerIntent::Heal(*amount));
                    }
                }
            }

            game::reward::apply_kill_rewards(&mut s.player, &result.kills);
            for reward in &result.kills {
                game::quest::apply(
                    &mut s.player,
                    data,
                    game::quest::QuestIntent::EnemyKilled {
                        enemy_id: &reward.enemy_id,
                    },
                );
            }
            return;
        }

        if let game::CombatEvent::Attack(Some(reward)) = game::combat::apply(
            &mut s.combat,
            game::CombatIntent::PlayerAttack {
                player_x: s.player.x,
                player_y: s.player.y,
                player_atk: s.player.total_atk(),
                facing: s.player.facing,
            },
        ) {
            game::reward::apply_kill_reward(&mut s.player, &reward);
            game::quest::apply(
                &mut s.player,
                data,
                game::quest::QuestIntent::EnemyKilled {
                    enemy_id: &reward.enemy_id,
                },
            );
        }
    }

    fn apply_dialog_action(
        s: &mut SessionState,
        data: &GameData,
        ui: &mut UiState,
        state: &mut GameState,
        action: &DialogAction,
    ) -> bool {
        match action {
            DialogAction::GiveQuest(id) => {
                if !s.player.quests.iter().any(|q| q.quest_id == *id) {
                    s.player.quests.push(crate::data::QuestProgress {
                        quest_id: id.clone(),
                        current_count: 0,
                        completed: false,
                        rewarded: false,
                    });
                }
            }
            DialogAction::CompleteQuest(id) => {
                let can_reward = s
                    .player
                    .quests
                    .iter()
                    .any(|q| q.quest_id == *id && q.completed && !q.rewarded);
                if can_reward && let Some(quest) = data.find_quest(id) {
                    s.player.stats.add_exp(quest.reward_exp);
                    let _ = game::player::apply(
                        &mut s.player,
                        game::PlayerIntent::AddGold(quest.reward_gold),
                    );

                    if let Some(item_id) = &quest.reward_item
                        && let Some(item) = data.find_item(item_id).cloned()
                    {
                        let _ =
                            game::player::apply(&mut s.player, game::PlayerIntent::AddItem(item));
                    }

                    if let Some(progress) = s.player.quests.iter_mut().find(|q| q.quest_id == *id) {
                        progress.rewarded = true;
                    }
                }
            }
            DialogAction::GiveItem(id) => {
                if let Some(item) = data.find_item(id).cloned() {
                    let _ = game::player::apply(&mut s.player, game::PlayerIntent::AddItem(item));
                }
            }
            DialogAction::TakeItem(id) => {
                if let Some(index) = s.player.inventory.iter().position(|item| item.id == *id) {
                    let _ =
                        game::player::apply(&mut s.player, game::PlayerIntent::RemoveItemAt(index));
                }
            }
            DialogAction::GiveGold(amount) => {
                let _ = game::player::apply(&mut s.player, game::PlayerIntent::AddGold(*amount));
            }
            DialogAction::TakeGold(amount) => {
                let _ = game::player::apply(&mut s.player, game::PlayerIntent::AddGold(-*amount));
            }
            DialogAction::OpenShop(shop_id) => {
                let Some(shop) = data.find_shop(shop_id).cloned() else {
                    return false;
                };
                let shop_items = data.get_shop_items(&shop);
                ui.shop.open(game::ShopState::new(shop, shop_items));
                *state = GameState::Shop;
                return true;
            }
            DialogAction::Heal => {
                s.player.stats.current_hp = s.player.stats.max_hp;
                s.player.stats.current_mp = s.player.stats.max_mp;
            }
        }

        false
    }

    fn apply_menu_intent(&mut self, intent: MenuIntent) {
        if !matches!(self.state, GameState::Menu) {
            return;
        }
        let event = game::menu::reduce(self.ui.menu.selected, &self.ui.menu.state.items, intent);
        match event {
            MenuEvent::None => {}
            MenuEvent::SetSelected(selected) => self.ui.menu.set_selected(selected),
            MenuEvent::Action(action) => match action {
                MenuAction::NewGame => {
                    let (state, session, intro) = game::lifecycle::start_new_game(&self.data);
                    self.enter_session(state, session, intro);
                }
                MenuAction::Continue => {
                    let (state, session, intro) = game::lifecycle::continue_game(&self.data);
                    self.enter_session(state, session, intro);
                }
                MenuAction::Exit => self.apply_event(AppEvent::Exit(0)),
            },
        }
    }

    fn apply_explore_intent(&mut self, intent: ExploreIntent) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }

        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let is_peaceful = self
            .data
            .find_map(&s.player.current_map_id)
            .is_some_and(|map| map.peaceful);
        let event = game::explore::reduce(is_peaceful, intent);
        match event {
            game::ExploreEvent::None => {}
            game::ExploreEvent::MoveDirection(direction) => {
                game::movement::on_direction_pressed(&mut s.movement, direction);
            }
            game::ExploreEvent::TryNpcInteract {
                facing,
                fallback_action,
            } => {
                if let Some(npc_event) =
                    game::npc::reduce(&s.player, &self.data, game::NpcIntent::Interact { facing })
                {
                    match npc_event {
                        game::NpcEvent::OpenDialog(dialog_spec) => {
                            if dialog_spec.restore {
                                s.player.stats.current_hp = s.player.stats.max_hp;
                                s.player.stats.current_mp = s.player.stats.max_mp;
                            }
                            self.ui.dialog.open(game::DialogState::new(
                                dialog_spec.npc_name,
                                dialog_spec.lines,
                            ));
                            self.state = GameState::Dialog;
                        }
                        game::NpcEvent::OpenShop(shop_id) => {
                            let _ = self.open_shop_by_id(&shop_id);
                        }
                        game::NpcEvent::RestoreStats => {
                            s.player.stats.current_hp = s.player.stats.max_hp;
                            s.player.stats.current_mp = s.player.stats.max_mp;
                        }
                    }
                } else if let Some(action) = fallback_action {
                    Self::apply_explore_action(s, &self.data, action);
                }
            }
            game::ExploreEvent::UseAction(action) => {
                Self::apply_explore_action(s, &self.data, action);
            }
            game::ExploreEvent::EnterPauseMenu => {
                self.ui.pause_menu.reset();
                self.state = GameState::PauseMenu;
            }
            game::ExploreEvent::EnterMenu => {
                let _ = game::save_game(&s.player);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
                self.state = GameState::Menu;
            }
        }
    }

    fn apply_inventory_intent(&mut self, intent: InventoryIntent) {
        if !matches!(self.state, GameState::Inventory) {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event =
            game::inventory::reduce(self.ui.inventory.selected, s.player.inventory.len(), intent);
        match event {
            game::InventoryEvent::None => {}
            game::InventoryEvent::SetSelected(selected) => self.ui.inventory.set_selected(selected),
            game::InventoryEvent::UseSelected(index) => {
                let _ = game::player::apply(&mut s.player, game::PlayerIntent::UseItem { index });
            }
            game::InventoryEvent::CloseToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_dialog_intent(&mut self, intent: DialogIntent) {
        if !matches!(self.state, GameState::Dialog) {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::dialog::reduce(self.ui.dialog.state.as_ref(), intent);
        match event {
            game::DialogEvent::None => {}
            game::DialogEvent::Transition(transition) => match transition {
                game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = self.ui.dialog.state.as_mut() {
                        dialog_state.current_line = line;
                    }
                    self.state = GameState::Dialog;
                }
                game::DialogTransition::CloseToExplore => {
                    self.ui.dialog.close();
                    self.state = GameState::Explore;
                }
            },
            game::DialogEvent::Action(action, transition) => {
                if Self::apply_dialog_action(s, &self.data, &mut self.ui, &mut self.state, &action)
                {
                    return;
                }

                match transition {
                    game::DialogTransition::SetLine(line) => {
                        if let Some(dialog_state) = self.ui.dialog.state.as_mut() {
                            dialog_state.current_line = line;
                        }
                        self.state = GameState::Dialog;
                    }
                    game::DialogTransition::CloseToExplore => {
                        self.ui.dialog.close();
                        self.state = GameState::Explore;
                    }
                }
            }
        }
    }

    fn apply_shop_intent(&mut self, intent: ShopIntent) {
        if !matches!(self.state, GameState::Shop) {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::shop::reduce(
            self.ui.shop.mode,
            self.ui.shop.selected,
            self.ui.shop.state.is_some(),
            s.player.stats.gold,
            s.player.inventory.len(),
            self.ui
                .shop
                .state
                .as_ref()
                .map(|state| state.items.as_slice())
                .unwrap_or(&[]),
            intent,
        );
        match event {
            game::ShopEvent::None => {}
            game::ShopEvent::ErrorNoActiveShop => {
                self.state = GameState::Error(String::from("No active shop state"));
            }
            game::ShopEvent::SetMode(mode) => {
                self.ui.shop.set_mode(mode);
            }
            game::ShopEvent::SetSelected(selected) => self.ui.shop.set_selected(selected),
            game::ShopEvent::BuyItem(item) => {
                let _ =
                    game::player::apply(&mut s.player, game::PlayerIntent::AddGold(-item.price));
                let _ = game::player::apply(&mut s.player, game::PlayerIntent::AddItem(item));
            }
            game::ShopEvent::SellSelected(index) => {
                let event =
                    game::player::apply(&mut s.player, game::PlayerIntent::RemoveItemAt(index));
                if let game::PlayerEvent::ItemRemoved(Some(item)) = event {
                    let _ = game::player::apply(
                        &mut s.player,
                        game::PlayerIntent::AddGold(item.price / 2),
                    );
                    let inv_len = s.player.inventory.len();
                    if self.ui.shop.selected >= inv_len && self.ui.shop.selected > 0 {
                        self.ui.shop.set_selected(self.ui.shop.selected - 1);
                    }
                }
            }
            game::ShopEvent::CloseToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_pause_menu_intent(&mut self, intent: PauseMenuIntent) {
        if !matches!(self.state, GameState::PauseMenu) {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };
        let event = game::menu::reduce_pause(
            self.ui.pause_menu.selected,
            self.ui.pause_menu.state.items.len(),
            intent,
        );
        match event {
            game::PauseMenuEvent::None => {}
            game::PauseMenuEvent::SetSelected(selected) => {
                self.ui.pause_menu.set_selected(selected)
            }
            game::PauseMenuEvent::OpenInventory => {
                self.ui.inventory.reset();
                self.state = GameState::Inventory;
            }
            game::PauseMenuEvent::OpenStats => self.state = GameState::Stats,
            game::PauseMenuEvent::OpenQuestLog => self.state = GameState::QuestLog,
            game::PauseMenuEvent::SaveAndReturnExplore => {
                let _ = game::save_game(&s.player);
                self.ui.shop.reset();
                self.state = GameState::Explore;
            }
            game::PauseMenuEvent::BackToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_release_movement_key(&mut self, key: KeyCode) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            return;
        };
        if let Some(direction) = direction_for_key(key) {
            game::movement::on_direction_released(&mut s.movement, direction);
        }
    }

    fn reduce_intent(&self, intent: AppIntent) -> AppEvent {
        match intent {
            AppIntent::UpdateLoading => AppEvent::UpdateLoading,
            AppIntent::UpdateMovement => AppEvent::UpdateMovement,
            AppIntent::UpdateCombat => AppEvent::UpdateCombat,
            AppIntent::Menu(intent) => AppEvent::Menu(intent),
            AppIntent::Explore(intent) => AppEvent::Explore(intent),
            AppIntent::Inventory(intent) => AppEvent::Inventory(intent),
            AppIntent::Dialog(intent) => AppEvent::Dialog(intent),
            AppIntent::Shop(intent) => AppEvent::Shop(intent),
            AppIntent::PauseMenu(intent) => AppEvent::PauseMenu(intent),
            AppIntent::ReturnToExplore => AppEvent::ReturnToExplore,
            AppIntent::ReturnToMenuFromGameOver => AppEvent::ReturnToMenuFromGameOver,
            AppIntent::ReleaseMovementKey(key) => AppEvent::ReleaseMovementKey(key),
            AppIntent::Exit(code) => AppEvent::Exit(code),
        }
    }

    fn apply_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::UpdateLoading => self.apply_update_loading(),
            AppEvent::UpdateMovement => self.apply_update_movement(),
            AppEvent::UpdateCombat => self.apply_update_combat(),
            AppEvent::Menu(intent) => self.apply_menu_intent(intent),
            AppEvent::Explore(intent) => self.apply_explore_intent(intent),
            AppEvent::Inventory(intent) => self.apply_inventory_intent(intent),
            AppEvent::Dialog(intent) => self.apply_dialog_intent(intent),
            AppEvent::Shop(intent) => self.apply_shop_intent(intent),
            AppEvent::PauseMenu(intent) => self.apply_pause_menu_intent(intent),
            AppEvent::ReturnToExplore => self.state = GameState::Explore,
            AppEvent::ReturnToMenuFromGameOver => {
                self.state = GameState::Menu;
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            AppEvent::ReleaseMovementKey(key) => self.apply_release_movement_key(key),
            AppEvent::Exit(code) => wipi::kernel::exit(code),
        }
    }

    fn dispatch(&mut self, action: AppAction) {
        let intents = self.collect_intents(action);
        for intent in intents {
            let event = self.reduce_intent(intent);
            self.apply_event(event);
        }
    }
}

pub struct RpgGame {
    inner: Rc<RefCell<GameInner>>,
    render_state: Rc<RefCell<RenderState>>,
    _timer: Timer,
}

impl Default for RpgGame {
    fn default() -> Self {
        Self::new()
    }
}

impl RpgGame {
    fn tick(inner: &Rc<RefCell<GameInner>>, render_state: &Rc<RefCell<RenderState>>) {
        let mut inner = inner.borrow_mut();
        inner.update();
        let rs = build_render_state(&inner.state, inner.session.as_ref(), &inner.ui, &inner.data);
        *render_state.borrow_mut() = rs;
        drop(inner);
        repaint(0, 0, 0, 240, 320);
    }

    pub fn new() -> Self {
        let inner = Rc::new(RefCell::new(GameInner {
            state: GameState::Loading(0),
            data: Rc::new(GameData::default()),
            session: None,
            ui: UiState::default(),
        }));

        let render_state = Rc::new(RefCell::new(RenderState::Loading { step: 0 }));

        let timer_inner = Rc::clone(&inner);
        let timer_render_state = Rc::clone(&render_state);
        let timer = Timer::periodic(Duration::from_millis(33), move || {
            Self::tick(&timer_inner, &timer_render_state);
        });

        Self {
            inner,
            render_state,
            _timer: timer,
        }
    }
}

impl App for RpgGame {
    fn on_paint(&mut self) {
        let render_state = self.render_state.borrow();
        let mut fb = Framebuffer::screen_framebuffer();
        render(&render_state, &mut fb);
    }

    fn on_keydown(&mut self, key: KeyCode) {
        self.inner.borrow_mut().dispatch(AppAction::KeyDown(key));
    }

    fn on_keyup(&mut self, key: KeyCode) {
        self.inner.borrow_mut().dispatch(AppAction::KeyUp(key));
    }
}

#[wipi_main]
pub fn main() -> RpgGame {
    RpgGame::new()
}
