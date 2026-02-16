use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use wipi::event::KeyCode;

use crate::data::Direction;
use crate::game::{
    DialogIntent, ExploreIntent, GameData, GameInput, GameIntent, GameState, InventoryIntent,
    MenuAction, MenuEvent, MenuIntent, MenuState, PauseMenuIntent, RenderState, SessionState,
    ShopIntent, UiState, build_render_state, has_save_data,
};

enum GameEvent {
    None,
    UpdateLoading(crate::game::LoadingEvent),
    UpdateMovement(AppMovementEvent),
    UpdateCombat(crate::game::combat::CombatTickEvent),
    Menu(MenuEvent),
    Explore(AppExploreEvent),
    Inventory(crate::game::InventoryEvent),
    Dialog(crate::game::DialogEvent),
    Shop(crate::game::ShopEvent),
    PauseMenu(crate::game::PauseMenuEvent),
    MapChanged,
    ReturnToExplore,
    ReturnToMenuFromGameOver,
    ReleaseMovementKey(KeyCode),
    Exit(i32),
    Error(String),
}

enum AppExploreEvent {
    MoveDirection(Direction),
    Npc(crate::game::NpcEvent),
    UseAction(crate::game::ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

enum AppMovementEvent {
    Tick(
        crate::game::MovementTickEvent,
        Option<crate::game::TileEvent>,
        bool,
    ),
}

pub struct GameRuntime {
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

impl GameRuntime {
    pub fn new() -> Self {
        Self {
            state: GameState::Loading(0),
            data: Rc::new(GameData::default()),
            session: None,
            ui: UiState::default(),
        }
    }

    pub fn on_keydown(&mut self, key: KeyCode) {
        self.dispatch(GameInput::KeyDown(key));
    }

    pub fn on_keyup(&mut self, key: KeyCode) {
        self.dispatch(GameInput::KeyUp(key));
    }

    pub fn tick_and_build_render_state(&mut self) -> RenderState {
        self.update();
        build_render_state(&self.state, self.session.as_ref(), &self.ui, &self.data)
    }

    fn update(&mut self) {
        self.dispatch(GameInput::Tick);
    }

    fn collect_intents(&self, action: GameInput) -> Vec<GameIntent> {
        let mut intents = Vec::new();

        match action {
            GameInput::Tick => match self.state {
                GameState::Loading(_) => intents.push(GameIntent::UpdateLoading),
                GameState::Explore => {
                    intents.push(GameIntent::UpdateMovement);
                    intents.push(GameIntent::UpdateCombat);
                }
                _ => {}
            },
            GameInput::KeyDown(key) => match self.state {
                GameState::Loading(_) => {}
                GameState::Menu => {
                    if let Some(intent) = MenuIntent::intent_for_key(key) {
                        intents.push(GameIntent::Menu(intent));
                    }
                }
                GameState::Explore => {
                    let facing = self
                        .session
                        .as_ref()
                        .map(|s| s.player.facing)
                        .unwrap_or(Direction::Down);
                    for intent in ExploreIntent::intent_for_key(
                        key,
                        facing,
                        self.ui.explore.ok_action,
                        self.ui.explore.key_actions,
                    ) {
                        intents.push(GameIntent::Explore(intent));
                    }
                }
                GameState::Inventory => {
                    if let Some(intent) = InventoryIntent::intent_for_key(key) {
                        intents.push(GameIntent::Inventory(intent));
                    }
                }
                GameState::Stats | GameState::QuestLog => {
                    if matches!(key, KeyCode::Back | KeyCode::Ok) {
                        intents.push(GameIntent::ReturnToExplore);
                    }
                }
                GameState::Dialog => {
                    if let Some(intent) = DialogIntent::intent_for_key(key) {
                        intents.push(GameIntent::Dialog(intent));
                    }
                }
                GameState::Shop => {
                    if let Some(intent) = ShopIntent::intent_for_key(key) {
                        intents.push(GameIntent::Shop(intent));
                    }
                }
                GameState::PauseMenu => {
                    if let Some(intent) = PauseMenuIntent::intent_for_key(key) {
                        intents.push(GameIntent::PauseMenu(intent));
                    }
                }
                GameState::GameOver => {
                    if matches!(key, KeyCode::Ok) {
                        intents.push(GameIntent::ReturnToMenuFromGameOver);
                    }
                }
                GameState::Error(_) => {
                    if matches!(key, KeyCode::Ok) {
                        intents.push(GameIntent::Exit(1));
                    }
                }
            },
            GameInput::KeyUp(key) => {
                if matches!(self.state, GameState::Explore)
                    && self.session.is_some()
                    && let Some(direction) = direction_for_key(key)
                {
                    intents.push(GameIntent::ReleaseMovementKey(match direction {
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

    fn apply_update_loading(&mut self, event: crate::game::LoadingEvent) {
        match event {
            crate::game::LoadingEvent::Advance(step) => self.state = GameState::Loading(step),
            crate::game::LoadingEvent::Loaded => {
                self.state = GameState::Menu;
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            crate::game::LoadingEvent::Error(msg) => self.state = GameState::Error(msg),
        }
    }

    fn apply_update_movement(&mut self, event: AppMovementEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        let AppMovementEvent::Tick(movement_event, tile_event, _) = event;
        s.apply_movement_tick(&self.data, movement_event, tile_event);
    }

    fn apply_update_combat(&mut self, result: crate::game::combat::CombatTickEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        if s.apply_combat_tick(result) {
            self.state = GameState::GameOver;
        }
    }

    fn apply_map_changed(&mut self) {
        let Some(s) = self.session.as_mut() else {
            return;
        };
        s.spawn_current_map_enemies(&self.data);
    }

    fn dialog_state_from_intro(
        &self,
        intro: Option<crate::game::lifecycle::IntroDialogSpec>,
    ) -> Option<crate::game::DialogState> {
        let spec = intro?;
        let dialog = self.data.find_dialog(&spec.dialog_id)?;
        Some(crate::game::DialogState::from_dialog(spec.npc_name, dialog))
    }

    fn enter_session(
        &mut self,
        state: GameState,
        session: SessionState,
        intro: Option<crate::game::lifecycle::IntroDialogSpec>,
    ) {
        self.state = state;
        self.session = Some(session);
        self.apply_event(GameEvent::MapChanged);
        self.ui = UiState::default();
        self.ui.dialog.set(self.dialog_state_from_intro(intro));
    }

    fn open_shop_by_id(&mut self, shop_id: &str) -> bool {
        let Some(shop) = self.data.find_shop(shop_id).cloned() else {
            return false;
        };
        let shop_items = self.data.get_shop_items(&shop);
        self.ui
            .shop
            .open(crate::game::ShopState::new(shop, shop_items));
        self.state = GameState::Shop;
        true
    }

    fn apply_menu_event(&mut self, event: MenuEvent) {
        match event {
            MenuEvent::None => {}
            MenuEvent::SetSelected(selected) => self.ui.menu.set_selected(selected),
            MenuEvent::Action(action) => match action {
                MenuAction::NewGame => {
                    let (state, session, intro) =
                        crate::game::lifecycle::start_new_game(&self.data);
                    self.enter_session(state, session, intro);
                }
                MenuAction::Continue => {
                    let (state, session, intro) = crate::game::lifecycle::continue_game(&self.data);
                    self.enter_session(state, session, intro);
                }
                MenuAction::Exit => self.apply_event(GameEvent::Exit(0)),
            },
        }
    }

    fn apply_explore_event(&mut self, event: AppExploreEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        match event {
            AppExploreEvent::MoveDirection(direction) => {
                s.on_direction_pressed(direction);
            }
            AppExploreEvent::Npc(npc_event) => match npc_event {
                crate::game::NpcEvent::OpenDialog(dialog_spec) => {
                    if dialog_spec.restore {
                        s.restore_stats();
                    }
                    self.ui.dialog.open(crate::game::DialogState::new(
                        dialog_spec.npc_name,
                        dialog_spec.lines,
                    ));
                    self.state = GameState::Dialog;
                }
                crate::game::NpcEvent::OpenShop(shop_id) => {
                    let _ = self.open_shop_by_id(&shop_id);
                }
                crate::game::NpcEvent::RestoreStats => {
                    s.restore_stats();
                }
            },
            AppExploreEvent::UseAction(action) => {
                s.apply_explore_action(&self.data, action);
            }
            AppExploreEvent::EnterPauseMenu => {
                self.ui.pause_menu.reset();
                self.state = GameState::PauseMenu;
            }
            AppExploreEvent::EnterMenu => {
                let _ = crate::game::save_game(&s.player);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
                self.state = GameState::Menu;
            }
        }
    }

    fn apply_inventory_event(&mut self, event: crate::game::InventoryEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        match event {
            crate::game::InventoryEvent::None => {}
            crate::game::InventoryEvent::SetSelected(selected) => {
                self.ui.inventory.set_selected(selected)
            }
            crate::game::InventoryEvent::UseSelected(index) => {
                s.use_inventory_item(index);
            }
            crate::game::InventoryEvent::CloseToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_dialog_event(&mut self, event: crate::game::DialogEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        match event {
            crate::game::DialogEvent::None => {}
            crate::game::DialogEvent::Transition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = self.ui.dialog.state.as_mut() {
                        dialog_state.current_line = line;
                    }
                    self.state = GameState::Dialog;
                }
                crate::game::DialogTransition::CloseToExplore => {
                    self.ui.dialog.close();
                    self.state = GameState::Explore;
                }
            },
            crate::game::DialogEvent::Action(action, transition) => {
                match s.apply_dialog_action(&self.data, &action) {
                    crate::game::DialogActionResult::None => {}
                    crate::game::DialogActionResult::OpenShop(shop_id) => {
                        if self.open_shop_by_id(&shop_id) {
                            return;
                        }
                    }
                }

                match transition {
                    crate::game::DialogTransition::SetLine(line) => {
                        if let Some(dialog_state) = self.ui.dialog.state.as_mut() {
                            dialog_state.current_line = line;
                        }
                        self.state = GameState::Dialog;
                    }
                    crate::game::DialogTransition::CloseToExplore => {
                        self.ui.dialog.close();
                        self.state = GameState::Explore;
                    }
                }
            }
        }
    }

    fn apply_shop_event(&mut self, event: crate::game::ShopEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        match event {
            crate::game::ShopEvent::None => {}
            crate::game::ShopEvent::ErrorNoActiveShop => {
                self.state = GameState::Error(String::from("No active shop state"));
            }
            crate::game::ShopEvent::SetMode(mode) => {
                self.ui.shop.set_mode(mode);
            }
            crate::game::ShopEvent::SetSelected(selected) => self.ui.shop.set_selected(selected),
            crate::game::ShopEvent::BuyItem(item) => {
                s.buy_shop_item(item);
            }
            crate::game::ShopEvent::SellSelected(index) => {
                if s.sell_inventory_item(index).is_some() {
                    let inv_len = s.player.inventory.len();
                    if self.ui.shop.selected >= inv_len && self.ui.shop.selected > 0 {
                        self.ui.shop.set_selected(self.ui.shop.selected - 1);
                    }
                }
            }
            crate::game::ShopEvent::CloseToExplore => self.state = GameState::Explore,
        }
    }

    fn apply_pause_menu_event(&mut self, event: crate::game::PauseMenuEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        match event {
            crate::game::PauseMenuEvent::None => {}
            crate::game::PauseMenuEvent::SetSelected(selected) => {
                self.ui.pause_menu.set_selected(selected)
            }
            crate::game::PauseMenuEvent::OpenInventory => {
                self.ui.inventory.reset();
                self.state = GameState::Inventory;
            }
            crate::game::PauseMenuEvent::OpenStats => self.state = GameState::Stats,
            crate::game::PauseMenuEvent::OpenQuestLog => self.state = GameState::QuestLog,
            crate::game::PauseMenuEvent::SaveAndReturnExplore => {
                let _ = crate::game::save_game(&s.player);
                self.ui.shop.reset();
                self.state = GameState::Explore;
            }
            crate::game::PauseMenuEvent::BackToExplore => self.state = GameState::Explore,
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
            s.on_direction_released(direction);
        }
    }

    fn resolve_intent(&mut self, intent: GameIntent) -> Vec<GameEvent> {
        let event = match intent {
            GameIntent::UpdateLoading => {
                let GameState::Loading(step) = self.state else {
                    return vec![GameEvent::None];
                };

                let load_result = crate::game::lifecycle::load_step(&mut self.data, step);
                GameEvent::UpdateLoading(crate::game::lifecycle::resolve_loading(step, load_result))
            }
            GameIntent::UpdateMovement => {
                if !matches!(self.state, GameState::Explore) {
                    return vec![GameEvent::None];
                }
                let Some(s) = self.session.as_ref() else {
                    return vec![GameEvent::Error(String::from("No active session"))];
                };

                let movement = crate::game::movement::resolve_world_tick(
                    &s.movement,
                    &s.player,
                    &s.combat.enemies,
                    &self.data,
                );

                GameEvent::UpdateMovement(AppMovementEvent::Tick(
                    movement.movement_event,
                    movement.tile_event,
                    movement.map_changed,
                ))
            }
            GameIntent::UpdateCombat => {
                if !matches!(self.state, GameState::Explore) {
                    return vec![GameEvent::None];
                }
                let Some(s) = self.session.as_mut() else {
                    return vec![GameEvent::Error(String::from("No active session"))];
                };
                let Some(map) = self.data.find_map(&s.player.current_map_id) else {
                    return vec![GameEvent::None];
                };

                GameEvent::UpdateCombat(crate::game::combat::resolve_tick(
                    &s.combat,
                    crate::game::combat::CombatTickInput {
                        player_x: s.player.x,
                        player_y: s.player.y,
                        player_def: s.player.total_def(),
                        skill_cooldowns: s.skill_cooldowns,
                        mp_regen_timer: s.mp_regen_timer,
                        map,
                        enemy_data: &self.data.enemies,
                    },
                ))
            }
            GameIntent::Menu(intent) => {
                if !matches!(self.state, GameState::Menu) {
                    GameEvent::None
                } else {
                    GameEvent::Menu(crate::game::menu::resolve(
                        self.ui.menu.selected,
                        &self.ui.menu.state.items,
                        intent,
                    ))
                }
            }
            GameIntent::Explore(intent) => {
                if !matches!(self.state, GameState::Explore) {
                    return vec![GameEvent::None];
                }
                let Some(s) = self.session.as_ref() else {
                    return vec![GameEvent::Error(String::from("No active session"))];
                };
                let is_peaceful = self
                    .data
                    .find_map(&s.player.current_map_id)
                    .is_some_and(|map| map.peaceful);
                match crate::game::explore::resolve(is_peaceful, intent) {
                    crate::game::ExploreEvent::None => GameEvent::None,
                    crate::game::ExploreEvent::MoveDirection(direction) => {
                        GameEvent::Explore(AppExploreEvent::MoveDirection(direction))
                    }
                    crate::game::ExploreEvent::TryNpcInteract {
                        facing,
                        fallback_action,
                    } => {
                        if let Some(npc_event) = crate::game::npc::resolve(
                            &s.player,
                            &self.data,
                            crate::game::npc::NpcIntent::Interact { facing },
                        ) {
                            GameEvent::Explore(AppExploreEvent::Npc(npc_event))
                        } else if let Some(action) = fallback_action {
                            GameEvent::Explore(AppExploreEvent::UseAction(action))
                        } else {
                            GameEvent::None
                        }
                    }
                    crate::game::ExploreEvent::UseAction(action) => {
                        GameEvent::Explore(AppExploreEvent::UseAction(action))
                    }
                    crate::game::ExploreEvent::EnterPauseMenu => {
                        GameEvent::Explore(AppExploreEvent::EnterPauseMenu)
                    }
                    crate::game::ExploreEvent::EnterMenu => {
                        GameEvent::Explore(AppExploreEvent::EnterMenu)
                    }
                }
            }
            GameIntent::Inventory(intent) => {
                if !matches!(self.state, GameState::Inventory) {
                    return vec![GameEvent::None];
                }
                let Some(s) = self.session.as_ref() else {
                    return vec![GameEvent::Error(String::from("No active session"))];
                };
                GameEvent::Inventory(crate::game::inventory::resolve(
                    self.ui.inventory.selected,
                    s.player.inventory.len(),
                    intent,
                ))
            }
            GameIntent::Dialog(intent) => {
                if !matches!(self.state, GameState::Dialog) {
                    return vec![GameEvent::None];
                }
                if self.session.is_none() {
                    return vec![GameEvent::Error(String::from("No active session"))];
                }
                GameEvent::Dialog(crate::game::dialog::resolve(
                    self.ui.dialog.state.as_ref(),
                    intent,
                ))
            }
            GameIntent::Shop(intent) => {
                if !matches!(self.state, GameState::Shop) {
                    return vec![GameEvent::None];
                }
                let Some(s) = self.session.as_ref() else {
                    return vec![GameEvent::Error(String::from("No active session"))];
                };
                let ui_event = crate::game::shop::resolve_ui(
                    self.ui.shop.mode,
                    self.ui.shop.selected,
                    self.ui.shop.state.is_some(),
                    s.player.inventory.len(),
                    self.ui
                        .shop
                        .state
                        .as_ref()
                        .map(|state| state.items.len())
                        .unwrap_or(0),
                    intent,
                );

                let shop_items = self
                    .ui
                    .shop
                    .state
                    .as_ref()
                    .map(|state| state.items.as_slice())
                    .unwrap_or(&[]);

                let event = match ui_event {
                    crate::game::shop::ShopUiEvent::None => crate::game::ShopEvent::None,
                    crate::game::shop::ShopUiEvent::ErrorNoActiveShop => {
                        crate::game::ShopEvent::ErrorNoActiveShop
                    }
                    crate::game::shop::ShopUiEvent::SetMode(mode) => {
                        crate::game::ShopEvent::SetMode(mode)
                    }
                    crate::game::shop::ShopUiEvent::SetSelected(selected) => {
                        crate::game::ShopEvent::SetSelected(selected)
                    }
                    crate::game::shop::ShopUiEvent::RequestBuy(selected) => {
                        if let Some(item) = crate::game::shop::resolve_buy(
                            selected,
                            s.player.stats.gold,
                            shop_items,
                        ) {
                            crate::game::ShopEvent::BuyItem(item)
                        } else {
                            crate::game::ShopEvent::None
                        }
                    }
                    crate::game::shop::ShopUiEvent::RequestSell(selected) => {
                        crate::game::ShopEvent::SellSelected(selected)
                    }
                    crate::game::shop::ShopUiEvent::CloseToExplore => {
                        crate::game::ShopEvent::CloseToExplore
                    }
                };

                GameEvent::Shop(event)
            }
            GameIntent::PauseMenu(intent) => {
                if !matches!(self.state, GameState::PauseMenu) {
                    return vec![GameEvent::None];
                }
                if self.session.is_none() {
                    return vec![GameEvent::Error(String::from("No active session"))];
                }
                GameEvent::PauseMenu(crate::game::menu::resolve_pause(
                    self.ui.pause_menu.selected,
                    self.ui.pause_menu.state.items.len(),
                    intent,
                ))
            }
            GameIntent::ReturnToExplore => GameEvent::ReturnToExplore,
            GameIntent::ReturnToMenuFromGameOver => GameEvent::ReturnToMenuFromGameOver,
            GameIntent::ReleaseMovementKey(key) => GameEvent::ReleaseMovementKey(key),
            GameIntent::Exit(code) => GameEvent::Exit(code),
        };

        match event {
            GameEvent::UpdateMovement(AppMovementEvent::Tick(
                movement_event,
                tile_event,
                map_changed,
            )) => {
                let mut events = Vec::with_capacity(if map_changed { 2 } else { 1 });
                events.push(GameEvent::UpdateMovement(AppMovementEvent::Tick(
                    movement_event,
                    tile_event,
                    map_changed,
                )));
                if map_changed {
                    events.push(GameEvent::MapChanged);
                }
                events
            }
            other => vec![other],
        }
    }

    fn apply_event(&mut self, event: GameEvent) {
        match event {
            GameEvent::None => {}
            GameEvent::UpdateLoading(event) => self.apply_update_loading(event),
            GameEvent::UpdateMovement(event) => self.apply_update_movement(event),
            GameEvent::UpdateCombat(result) => self.apply_update_combat(result),
            GameEvent::Menu(event) => self.apply_menu_event(event),
            GameEvent::Explore(event) => self.apply_explore_event(event),
            GameEvent::Inventory(event) => self.apply_inventory_event(event),
            GameEvent::Dialog(event) => self.apply_dialog_event(event),
            GameEvent::Shop(event) => self.apply_shop_event(event),
            GameEvent::PauseMenu(event) => self.apply_pause_menu_event(event),
            GameEvent::MapChanged => self.apply_map_changed(),
            GameEvent::ReturnToExplore => self.state = GameState::Explore,
            GameEvent::ReturnToMenuFromGameOver => {
                self.state = GameState::Menu;
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            GameEvent::ReleaseMovementKey(key) => self.apply_release_movement_key(key),
            GameEvent::Exit(code) => wipi::kernel::exit(code),
            GameEvent::Error(message) => self.state = GameState::Error(message),
        }
    }

    fn dispatch(&mut self, action: GameInput) {
        let intents = self.collect_intents(action);
        for intent in intents {
            let events = self.resolve_intent(intent);
            for event in events {
                self.apply_event(event);
            }
        }
    }
}
