use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

mod runtime_flow;

use crate::data::Direction;
use crate::game::{
    GameData, GameInput, GameIntent, GameState, InputKey, MenuAction, MenuEvent, MenuState,
    RenderState, SceneIntent, SessionState, ShopIntent, SystemIntent, UiState, build_render_state,
    has_save_data,
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
    Menu(crate::game::MenuEvent),
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

struct SessionSlot {
    state: Option<SessionState>,
}

impl SessionSlot {
    fn inactive() -> Self {
        Self { state: None }
    }

    fn activate(&mut self, session: SessionState) {
        self.state = Some(session);
    }

    fn deactivate(&mut self) {
        self.state = None;
    }

    fn as_ref(&self) -> Option<&SessionState> {
        self.state.as_ref()
    }

    fn as_mut(&mut self) -> Option<&mut SessionState> {
        self.state.as_mut()
    }

    fn is_active(&self) -> bool {
        self.state.is_some()
    }
}

pub struct GameRuntime {
    state: GameState,
    data: Rc<GameData>,
    session: SessionSlot,
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

fn state_requires_session(state: &GameState) -> bool {
    matches!(
        state,
        GameState::Explore
            | GameState::Inventory
            | GameState::Stats
            | GameState::Dialog
            | GameState::Shop
            | GameState::QuestLog
            | GameState::PauseMenu
            | GameState::GameOver
    )
}

fn state_keeps_session(state: &GameState) -> bool {
    state_requires_session(state)
}

impl GameRuntime {
    pub fn new() -> Self {
        Self {
            state: GameState::Loading(0),
            data: Rc::new(GameData::default()),
            session: SessionSlot::inactive(),
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
            GameState::Loading(_) => vec![GameIntent::System(SystemIntent::UpdateLoading)],
            GameState::Explore => vec![
                GameIntent::System(SystemIntent::UpdateMovement),
                GameIntent::System(SystemIntent::UpdateCombat),
            ],
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
            && self.session.is_active()
            && let Some(direction) = direction_for_key(key)
        {
            vec![GameIntent::System(SystemIntent::ReleaseMovementDirection(
                direction,
            ))]
        } else {
            Vec::new()
        }
    }

    fn collect_menu_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .menu
            .intent_for_key(key)
            .map(SceneIntent::Menu)
            .map(GameIntent::Scene)
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
            .map(SceneIntent::Explore)
            .map(GameIntent::Scene)
            .collect()
    }

    fn collect_inventory_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .inventory
            .intent_for_key(key)
            .map(SceneIntent::Inventory)
            .map(GameIntent::Scene)
            .into_iter()
            .collect()
    }

    fn collect_overlay_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(key, InputKey::Back | InputKey::Ok) {
            vec![GameIntent::System(SystemIntent::ReturnToExplore)]
        } else {
            Vec::new()
        }
    }

    fn collect_dialog_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .dialog
            .intent_for_key(key)
            .map(SceneIntent::Dialog)
            .map(GameIntent::Scene)
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
                    GameIntent::Scene(SceneIntent::Shop(ShopIntent::BuySelected(selected)))
                }
                crate::game::ShopUiIntent::SellSelected(selected) => {
                    GameIntent::Scene(SceneIntent::Shop(ShopIntent::SellSelected(selected)))
                }
                crate::game::ShopUiIntent::Close => {
                    GameIntent::Scene(SceneIntent::Shop(ShopIntent::Close))
                }
            })
            .into_iter()
            .collect()
    }

    fn collect_pause_menu_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        self.ui
            .pause_menu
            .intent_for_key(key)
            .map(SceneIntent::PauseMenu)
            .map(GameIntent::Scene)
            .into_iter()
            .collect()
    }

    fn collect_game_over_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(key, InputKey::Ok) {
            vec![GameIntent::System(SystemIntent::ReturnToMenuFromGameOver)]
        } else {
            Vec::new()
        }
    }

    fn collect_error_keydown_intents(&self, key: InputKey) -> Vec<GameIntent> {
        if matches!(key, InputKey::Ok) {
            vec![GameIntent::System(SystemIntent::Exit(1))]
        } else {
            Vec::new()
        }
    }

    fn transition_to(&mut self, next: GameState) {
        if state_requires_session(&next) && !self.session.is_active() {
            self.state = GameState::Error(alloc::format!(
                "Missing session for state transition: {:?}",
                next
            ));
            return;
        }

        if self.state.can_transition_to(&next) {
            self.state = next;
            if !state_keeps_session(&self.state) {
                self.session.deactivate();
            }
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
        self.session.activate(session);
        self.transition_to(state);
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

    fn resolve_intent(&mut self, intent: GameIntent) -> Vec<RuntimeEvent> {
        match intent {
            GameIntent::System(system_intent) => match system_intent {
                SystemIntent::UpdateLoading => self.resolve_update_loading_intent(),
                SystemIntent::UpdateMovement => self.resolve_update_movement_intent(),
                SystemIntent::UpdateCombat => self.resolve_update_combat_intent(),
                SystemIntent::ReturnToExplore => {
                    vec![RuntimeEvent::Transition(TransitionEvent::ToExplore)]
                }
                SystemIntent::ReturnToMenuFromGameOver => {
                    vec![RuntimeEvent::Transition(
                        TransitionEvent::ToMenuFromGameOver,
                    )]
                }
                SystemIntent::ReleaseMovementDirection(direction) => {
                    vec![RuntimeEvent::Transition(
                        TransitionEvent::ReleaseMovementDirection(direction),
                    )]
                }
                SystemIntent::Exit(code) => vec![RuntimeEvent::Exit(code)],
            },
            GameIntent::Scene(scene_intent) => {
                runtime_flow::resolve_scene_intent(self, scene_intent)
            }
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

    fn apply_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::None => {}
            RuntimeEvent::Domain(domain_event) => match domain_event {
                DomainEvent::Loading(event) => self.apply_update_loading(event),
                DomainEvent::Movement(event) => self.apply_update_movement(event),
                DomainEvent::Combat(event) => self.apply_update_combat(event),
                DomainEvent::Menu(event) => runtime_flow::apply_menu_event(self, event),
                DomainEvent::Explore(event) => runtime_flow::apply_explore_event(self, event),
                DomainEvent::Inventory(event) => runtime_flow::apply_inventory_event(self, event),
                DomainEvent::Dialog(event) => runtime_flow::apply_dialog_event(self, event),
                DomainEvent::Shop(event) => runtime_flow::apply_shop_event(self, event),
                DomainEvent::PauseMenu(event) => runtime_flow::apply_pause_menu_event(self, event),
            },
            RuntimeEvent::Transition(event) => runtime_flow::apply_transition_event(self, event),
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
