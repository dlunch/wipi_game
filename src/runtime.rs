use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::data::Direction;
use crate::game::{
    GameData, GameInput, GameIntent, GameState, InputKey, MenuAction, MenuEvent, MenuState,
    RenderState, SessionState, ShopIntent, UiState, build_render_state, has_save_data,
};

enum RuntimeEvent {
    None,
    Domain(DomainEvent),
    Transition(TransitionEvent),
    Exit(i32),
    Error(String),
}

enum DomainEvent {
    Loading(crate::game::LoadingEvent),
    Movement(AppMovementEvent),
    Combat(crate::game::combat::CombatTickEvent),
    Menu(MenuEvent),
    Explore(AppExploreEvent),
    Inventory(crate::game::InventoryEvent),
    Dialog(crate::game::DialogEvent),
    Shop(crate::game::ShopEvent),
    PauseMenu(crate::game::PauseMenuEvent),
}

enum TransitionEvent {
    MapChanged,
    ToExplore,
    ToMenuFromGameOver,
    ReleaseMovementDirection(Direction),
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
    ),
}

pub struct GameRuntime {
    state: GameState,
    data: Rc<GameData>,
    session: Option<SessionState>,
    ui: UiState,
}

fn direction_for_key(key: InputKey) -> Option<Direction> {
    match key {
        InputKey::Up => Some(Direction::Up),
        InputKey::Down => Some(Direction::Down),
        InputKey::Left => Some(Direction::Left),
        InputKey::Right => Some(Direction::Right),
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

    pub fn on_keydown(&mut self, key: InputKey) {
        self.dispatch(GameInput::KeyDown(key));
    }

    pub fn on_keyup(&mut self, key: InputKey) {
        self.dispatch(GameInput::KeyUp(key));
    }

    pub fn tick_and_build_render_state(&mut self) -> RenderState {
        self.update();
        build_render_state(&self.state, self.session.as_ref(), &self.ui, &self.data)
    }

    fn update(&mut self) {
        self.dispatch(GameInput::Tick);
    }

    fn collect_intents(&mut self, action: GameInput) -> Vec<GameIntent> {
        match action {
            GameInput::Tick => self.collect_tick_intents(),
            GameInput::KeyDown(key) => self.collect_keydown_intents(key),
            GameInput::KeyUp(key) => self.collect_keyup_intents(key),
        }
    }

    fn collect_tick_intents(&self) -> Vec<GameIntent> {
        match self.state {
            GameState::Loading(_) => vec![GameIntent::UpdateLoading],
            GameState::Explore => vec![GameIntent::UpdateMovement, GameIntent::UpdateCombat],
            _ => Vec::new(),
        }
    }

    fn collect_keydown_intents(&mut self, key: InputKey) -> Vec<GameIntent> {
        match self.state {
            GameState::Loading(_) => Vec::new(),
            GameState::Menu => self.collect_menu_keydown_intents(key),
            GameState::Explore => self.collect_explore_keydown_intents(key),
            GameState::Inventory => self.collect_inventory_keydown_intents(key),
            GameState::Stats | GameState::QuestLog => self.collect_overlay_keydown_intents(key),
            GameState::Dialog => self.collect_dialog_keydown_intents(key),
            GameState::Shop => self.collect_shop_keydown_intents(key),
            GameState::PauseMenu => self.collect_pause_menu_keydown_intents(key),
            GameState::GameOver => self.collect_game_over_keydown_intents(key),
            GameState::Error(_) => self.collect_error_keydown_intents(key),
        }
    }

    fn collect_keyup_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(self.state, GameState::Explore)
            && self.session.is_some()
            && let Some(direction) = direction_for_key(key)
        {
            vec![GameIntent::ReleaseMovementDirection(direction)]
        } else {
            Vec::new()
        }
    }

    fn collect_menu_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .menu
            .intent_for_key(key)
            .map(GameIntent::Menu)
            .into_iter()
            .collect()
    }

    fn collect_explore_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        let facing = self
            .session
            .as_ref()
            .map(|s| s.player.facing)
            .unwrap_or(Direction::Down);
        self.ui
            .explore
            .intents_for_key(key, facing)
            .into_iter()
            .map(GameIntent::Explore)
            .collect()
    }

    fn collect_inventory_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .inventory
            .intent_for_key(key)
            .map(GameIntent::Inventory)
            .into_iter()
            .collect()
    }

    fn collect_overlay_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(key, InputKey::Back | InputKey::Ok) {
            vec![GameIntent::ReturnToExplore]
        } else {
            Vec::new()
        }
    }

    fn collect_dialog_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .dialog
            .intent_for_key(key)
            .map(GameIntent::Dialog)
            .into_iter()
            .collect()
    }

    fn collect_shop_keydown_intents(&mut self, key: InputKey) -> Vec<GameIntent> {
        let inventory_len = self
            .session
            .as_ref()
            .map(|s| s.player.inventory.len())
            .unwrap_or(0);

        self.ui
            .shop
            .handle_key(key, inventory_len)
            .map(|ui_intent| match ui_intent {
                crate::game::ShopUiIntent::BuySelected(selected) => {
                    GameIntent::Shop(ShopIntent::BuySelected(selected))
                }
                crate::game::ShopUiIntent::SellSelected(selected) => {
                    GameIntent::Shop(ShopIntent::SellSelected(selected))
                }
                crate::game::ShopUiIntent::Close => GameIntent::Shop(ShopIntent::Close),
            })
            .into_iter()
            .collect()
    }

    fn collect_pause_menu_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .pause_menu
            .intent_for_key(key)
            .map(GameIntent::PauseMenu)
            .into_iter()
            .collect()
    }

    fn collect_game_over_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(key, InputKey::Ok) {
            vec![GameIntent::ReturnToMenuFromGameOver]
        } else {
            Vec::new()
        }
    }

    fn collect_error_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(key, InputKey::Ok) {
            vec![GameIntent::Exit(1)]
        } else {
            Vec::new()
        }
    }

    fn transition_to(&mut self, next: GameState) {
        if self.state.can_transition_to(&next) {
            self.state = next;
            return;
        }

        self.state = GameState::Error(alloc::format!(
            "Invalid state transition: {:?} -> {:?}",
            self.state,
            next
        ));
    }

    fn apply_update_loading(&mut self, event: crate::game::LoadingEvent) {
        match event {
            crate::game::LoadingEvent::Advance(step) => {
                self.transition_to(GameState::Loading(step))
            }
            crate::game::LoadingEvent::Loaded => {
                self.transition_to(GameState::Menu);
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

        let AppMovementEvent::Tick(movement_event, tile_event) = event;
        s.apply_movement_tick(&self.data, movement_event, tile_event);
    }

    fn apply_update_combat(&mut self, result: crate::game::combat::CombatTickEvent) {
        let Some(s) = self.session.as_mut() else {
            self.state = GameState::Error(String::from("No active session"));
            return;
        };

        if s.apply_combat_tick(result) {
            self.transition_to(GameState::GameOver);
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
        self.transition_to(state);
        self.session = Some(session);
        self.apply_event(RuntimeEvent::Transition(TransitionEvent::MapChanged));
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
        self.transition_to(GameState::Shop);
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
                MenuAction::Exit => self.apply_event(RuntimeEvent::Exit(0)),
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
                    self.transition_to(GameState::Dialog);
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
                self.transition_to(GameState::PauseMenu);
            }
            AppExploreEvent::EnterMenu => {
                let _ = crate::game::save_game(&s.player);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
                self.transition_to(GameState::Menu);
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
            crate::game::InventoryEvent::CloseToExplore => self.transition_to(GameState::Explore),
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
                    self.transition_to(GameState::Dialog);
                }
                crate::game::DialogTransition::CloseToExplore => {
                    self.ui.dialog.close();
                    self.transition_to(GameState::Explore);
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
                        self.transition_to(GameState::Dialog);
                    }
                    crate::game::DialogTransition::CloseToExplore => {
                        self.ui.dialog.close();
                        self.transition_to(GameState::Explore);
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
            crate::game::ShopEvent::CloseToExplore => self.transition_to(GameState::Explore),
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
                self.transition_to(GameState::Inventory);
            }
            crate::game::PauseMenuEvent::OpenStats => self.transition_to(GameState::Stats),
            crate::game::PauseMenuEvent::OpenQuestLog => self.transition_to(GameState::QuestLog),
            crate::game::PauseMenuEvent::SaveAndReturnExplore => {
                let _ = crate::game::save_game(&s.player);
                self.ui.shop.reset();
                self.transition_to(GameState::Explore);
            }
            crate::game::PauseMenuEvent::BackToExplore => self.transition_to(GameState::Explore),
        }
    }

    fn apply_transition_event(&mut self, event: TransitionEvent) {
        match event {
            TransitionEvent::MapChanged => self.apply_map_changed(),
            TransitionEvent::ToExplore => self.transition_to(GameState::Explore),
            TransitionEvent::ToMenuFromGameOver => {
                self.transition_to(GameState::Menu);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            TransitionEvent::ReleaseMovementDirection(direction) => {
                self.apply_release_movement_direction(direction)
            }
        }
    }

    fn apply_release_movement_direction(&mut self, direction: Direction) {
        if !matches!(self.state, GameState::Explore) {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            return;
        };
        s.on_direction_released(direction);
    }

    fn resolve_intent(&mut self, intent: GameIntent) -> Vec<RuntimeEvent> {
        match intent {
            GameIntent::UpdateLoading => self.resolve_update_loading_intent(),
            GameIntent::UpdateMovement => self.resolve_update_movement_intent(),
            GameIntent::UpdateCombat => self.resolve_update_combat_intent(),
            GameIntent::Menu(intent) => self.resolve_menu_intent(intent),
            GameIntent::Explore(intent) => self.resolve_explore_intent(intent),
            GameIntent::Inventory(intent) => self.resolve_inventory_intent(intent),
            GameIntent::Dialog(intent) => self.resolve_dialog_intent(intent),
            GameIntent::Shop(intent) => self.resolve_shop_intent(intent),
            GameIntent::PauseMenu(intent) => self.resolve_pause_menu_intent(intent),
            GameIntent::ReturnToExplore => {
                vec![RuntimeEvent::Transition(TransitionEvent::ToExplore)]
            }
            GameIntent::ReturnToMenuFromGameOver => {
                vec![RuntimeEvent::Transition(
                    TransitionEvent::ToMenuFromGameOver,
                )]
            }
            GameIntent::ReleaseMovementDirection(direction) => vec![RuntimeEvent::Transition(
                TransitionEvent::ReleaseMovementDirection(direction),
            )],
            GameIntent::Exit(code) => vec![RuntimeEvent::Exit(code)],
        }
    }

    fn resolve_update_loading_intent(&mut self) -> Vec<RuntimeEvent> {
        let GameState::Loading(step) = self.state else {
            return vec![RuntimeEvent::None];
        };

        let load_result = crate::game::lifecycle::load_step(&mut self.data, step);
        vec![RuntimeEvent::Domain(DomainEvent::Loading(
            crate::game::lifecycle::resolve_loading(step, load_result),
        ))]
    }

    fn resolve_update_movement_intent(&self) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Explore) {
            return vec![RuntimeEvent::None];
        }
        let Some(s) = self.session.as_ref() else {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        };

        let movement = crate::game::movement::resolve_world_tick(
            &s.movement,
            &s.player,
            &s.combat.enemies,
            &self.data,
        );

        let mut events = Vec::with_capacity(if movement.map_changed { 2 } else { 1 });
        events.push(RuntimeEvent::Domain(DomainEvent::Movement(
            AppMovementEvent::Tick(movement.movement_event, movement.tile_event),
        )));
        if movement.map_changed {
            events.push(RuntimeEvent::Transition(TransitionEvent::MapChanged));
        }
        events
    }

    fn resolve_update_combat_intent(&self) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Explore) {
            return vec![RuntimeEvent::None];
        }
        let Some(s) = self.session.as_ref() else {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        };
        let Some(map) = self.data.find_map(&s.player.current_map_id) else {
            return vec![RuntimeEvent::None];
        };

        vec![RuntimeEvent::Domain(DomainEvent::Combat(
            crate::game::combat::resolve_tick(
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
            ),
        ))]
    }

    fn resolve_menu_intent(&self, intent: crate::game::MenuIntent) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Menu) {
            return vec![RuntimeEvent::None];
        }

        vec![RuntimeEvent::Domain(DomainEvent::Menu(
            crate::game::menu::resolve(self.ui.menu.selected, &self.ui.menu.state.items, intent),
        ))]
    }

    fn resolve_explore_intent(&self, intent: crate::game::ExploreIntent) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Explore) {
            return vec![RuntimeEvent::None];
        }
        let Some(s) = self.session.as_ref() else {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        };

        let is_peaceful = self
            .data
            .find_map(&s.player.current_map_id)
            .is_some_and(|map| map.peaceful);

        let event = match crate::game::explore::resolve(is_peaceful, intent) {
            crate::game::ExploreEvent::None => RuntimeEvent::None,
            crate::game::ExploreEvent::MoveDirection(direction) => RuntimeEvent::Domain(
                DomainEvent::Explore(AppExploreEvent::MoveDirection(direction)),
            ),
            crate::game::ExploreEvent::TryNpcInteract {
                facing,
                fallback_action,
            } => {
                if let Some(npc_event) = crate::game::npc::resolve(
                    &s.player,
                    &self.data,
                    crate::game::npc::NpcIntent::Interact { facing },
                ) {
                    RuntimeEvent::Domain(DomainEvent::Explore(AppExploreEvent::Npc(npc_event)))
                } else if let Some(action) = fallback_action {
                    RuntimeEvent::Domain(DomainEvent::Explore(AppExploreEvent::UseAction(action)))
                } else {
                    RuntimeEvent::None
                }
            }
            crate::game::ExploreEvent::UseAction(action) => {
                RuntimeEvent::Domain(DomainEvent::Explore(AppExploreEvent::UseAction(action)))
            }
            crate::game::ExploreEvent::EnterPauseMenu => {
                RuntimeEvent::Domain(DomainEvent::Explore(AppExploreEvent::EnterPauseMenu))
            }
            crate::game::ExploreEvent::EnterMenu => {
                RuntimeEvent::Domain(DomainEvent::Explore(AppExploreEvent::EnterMenu))
            }
        };
        vec![event]
    }

    fn resolve_inventory_intent(&self, intent: crate::game::InventoryIntent) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Inventory) {
            return vec![RuntimeEvent::None];
        }
        let Some(s) = self.session.as_ref() else {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        };

        vec![RuntimeEvent::Domain(DomainEvent::Inventory(
            crate::game::inventory::resolve(
                self.ui.inventory.selected,
                s.player.inventory.len(),
                intent,
            ),
        ))]
    }

    fn resolve_dialog_intent(&self, intent: crate::game::DialogIntent) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Dialog) {
            return vec![RuntimeEvent::None];
        }
        if self.session.is_none() {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        }

        vec![RuntimeEvent::Domain(DomainEvent::Dialog(
            crate::game::dialog::resolve(self.ui.dialog.state.as_ref(), intent),
        ))]
    }

    fn resolve_shop_intent(&self, intent: ShopIntent) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::Shop) {
            return vec![RuntimeEvent::None];
        }
        let Some(s) = self.session.as_ref() else {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        };
        let shop_items = self
            .ui
            .shop
            .state
            .as_ref()
            .map(|state| state.items.as_slice())
            .unwrap_or(&[]);

        vec![RuntimeEvent::Domain(DomainEvent::Shop(
            crate::game::shop::resolve(intent, s.player.stats.gold, shop_items),
        ))]
    }

    fn resolve_pause_menu_intent(&self, intent: crate::game::PauseMenuIntent) -> Vec<RuntimeEvent> {
        if !matches!(self.state, GameState::PauseMenu) {
            return vec![RuntimeEvent::None];
        }
        if self.session.is_none() {
            return vec![RuntimeEvent::Error(String::from("No active session"))];
        }

        vec![RuntimeEvent::Domain(DomainEvent::PauseMenu(
            crate::game::menu::resolve_pause(
                self.ui.pause_menu.selected,
                self.ui.pause_menu.state.items.len(),
                intent,
            ),
        ))]
    }

    fn apply_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::None => {}
            RuntimeEvent::Domain(domain_event) => match domain_event {
                DomainEvent::Loading(event) => self.apply_update_loading(event),
                DomainEvent::Movement(event) => self.apply_update_movement(event),
                DomainEvent::Combat(event) => self.apply_update_combat(event),
                DomainEvent::Menu(event) => self.apply_menu_event(event),
                DomainEvent::Explore(event) => self.apply_explore_event(event),
                DomainEvent::Inventory(event) => self.apply_inventory_event(event),
                DomainEvent::Dialog(event) => self.apply_dialog_event(event),
                DomainEvent::Shop(event) => self.apply_shop_event(event),
                DomainEvent::PauseMenu(event) => self.apply_pause_menu_event(event),
            },
            RuntimeEvent::Transition(event) => self.apply_transition_event(event),
            RuntimeEvent::Exit(code) => wipi::kernel::exit(code),
            RuntimeEvent::Error(message) => self.state = GameState::Error(message),
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
