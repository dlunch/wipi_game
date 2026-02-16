use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::Direction;
use crate::game::{
    AppExploreEvent, AppMovementEvent, GameData, GameInput, GameState, InputKey, MenuAction,
    MenuEvent, MenuState, RenderState, RuntimeEvent, SessionEventApplier, SessionState, ShopIntent,
    TransitionEvent, UiInputEventResolver, UiState, build_render_state, has_save_data,
};

pub struct GameEngine {
    state: GameState,
    data: Rc<GameData>,
    session: Option<SessionState>,
    ui: UiState,
}

trait DomainEventResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool;
    fn resolve(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>>;
}

trait DomainEventApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool;
    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()>;
}

struct CoreResolveHandler;
struct CascadeResolveHandler;
struct CoreApplyHandler;
struct SystemApplyHandler;

static CORE_RESOLVE_HANDLER: CoreResolveHandler = CoreResolveHandler;
static CASCADE_RESOLVE_HANDLER: CascadeResolveHandler = CascadeResolveHandler;
static CORE_APPLY_HANDLER: CoreApplyHandler = CoreApplyHandler;
static SYSTEM_APPLY_HANDLER: SystemApplyHandler = SystemApplyHandler;

impl DomainEventResolver for CoreResolveHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::OverlayCloseRequested
                | RuntimeEvent::GameOverConfirmRequested
                | RuntimeEvent::ErrorConfirmRequested
                | RuntimeEvent::UpdateLoading
                | RuntimeEvent::UpdateMovement
                | RuntimeEvent::UpdateCombat
                | RuntimeEvent::MenuInput(_)
                | RuntimeEvent::ExploreInput(_)
                | RuntimeEvent::InventoryInput(_)
                | RuntimeEvent::DialogInput(_)
                | RuntimeEvent::ShopInput(_)
                | RuntimeEvent::PauseMenuInput(_)
        )
    }

    fn resolve(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        match event {
            RuntimeEvent::OverlayCloseRequested => {
                Ok(vec![RuntimeEvent::Transition(TransitionEvent::ToExplore)])
            }
            RuntimeEvent::GameOverConfirmRequested => Ok(vec![RuntimeEvent::Transition(
                TransitionEvent::ToMenuFromGameOver,
            )]),
            RuntimeEvent::ErrorConfirmRequested => Ok(vec![RuntimeEvent::Exit(1)]),
            RuntimeEvent::UpdateLoading => engine.resolve_update_loading_event(),
            RuntimeEvent::UpdateMovement => engine.resolve_update_movement_event(),
            RuntimeEvent::UpdateCombat => engine.resolve_update_combat_event(),
            RuntimeEvent::MenuInput(intent) => engine.resolve_menu_input(*intent),
            RuntimeEvent::ExploreInput(intent) => engine.resolve_explore_input(*intent),
            RuntimeEvent::InventoryInput(intent) => engine.resolve_inventory_input(*intent),
            RuntimeEvent::DialogInput(intent) => engine.resolve_dialog_input(*intent),
            RuntimeEvent::ShopInput(intent) => engine.resolve_shop_input(*intent),
            RuntimeEvent::PauseMenuInput(intent) => engine.resolve_pause_menu_input(*intent),
            _ => Ok(Vec::new()),
        }
    }
}

impl DomainEventResolver for CascadeResolveHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Dialog(_) | RuntimeEvent::Menu(_) | RuntimeEvent::Explore(_)
        )
    }

    fn resolve(&self, _engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        match event {
            RuntimeEvent::Dialog(dialog_event) => match dialog_event {
                crate::game::DialogEvent::None => Ok(Vec::new()),
                crate::game::DialogEvent::Transition(transition) => {
                    Ok(vec![RuntimeEvent::ApplyDialogTransition(*transition)])
                }
                crate::game::DialogEvent::Action(action, transition) => Ok(vec![
                    RuntimeEvent::ApplyDialogAction(action.clone()),
                    RuntimeEvent::ApplyDialogTransition(*transition),
                ]),
            },
            RuntimeEvent::Menu(MenuEvent::Action(action)) => match action {
                MenuAction::NewGame => Ok(vec![RuntimeEvent::StartNewGame]),
                MenuAction::Continue => Ok(vec![RuntimeEvent::ContinueGame]),
                MenuAction::Exit => Ok(vec![RuntimeEvent::Exit(0)]),
            },
            RuntimeEvent::Explore(AppExploreEvent::Npc(npc_event)) => match npc_event {
                crate::game::NpcEvent::OpenDialog(dialog_spec) => {
                    let mut events = Vec::with_capacity(2);
                    if dialog_spec.restore {
                        events.push(RuntimeEvent::RestoreSessionStats);
                    }
                    events.push(RuntimeEvent::OpenDialogState(
                        crate::game::DialogState::new(
                            dialog_spec.npc_name.clone(),
                            dialog_spec.lines.clone(),
                        ),
                    ));
                    Ok(events)
                }
                crate::game::NpcEvent::OpenShop(shop_id) => {
                    Ok(vec![RuntimeEvent::OpenShopById(shop_id.clone())])
                }
                crate::game::NpcEvent::RestoreStats => Ok(vec![RuntimeEvent::RestoreSessionStats]),
            },
            RuntimeEvent::Explore(AppExploreEvent::UseAction(action)) => {
                Ok(vec![RuntimeEvent::CombatPlayerAction(*action)])
            }
            RuntimeEvent::Explore(AppExploreEvent::EnterPauseMenu) => {
                Ok(vec![RuntimeEvent::OpenPauseMenu])
            }
            RuntimeEvent::Explore(AppExploreEvent::EnterMenu) => {
                Ok(vec![RuntimeEvent::OpenMenuFromExplore])
            }
            _ => Ok(Vec::new()),
        }
    }
}

impl DomainEventApplier for CoreApplyHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Tick
                | RuntimeEvent::KeyDown(_)
                | RuntimeEvent::KeyUp(_)
                | RuntimeEvent::OverlayCloseRequested
                | RuntimeEvent::GameOverConfirmRequested
                | RuntimeEvent::ErrorConfirmRequested
                | RuntimeEvent::UpdateLoading
                | RuntimeEvent::UpdateMovement
                | RuntimeEvent::UpdateCombat
                | RuntimeEvent::MenuInput(_)
                | RuntimeEvent::ExploreInput(_)
                | RuntimeEvent::InventoryInput(_)
                | RuntimeEvent::DialogInput(_)
                | RuntimeEvent::ShopInput(_)
                | RuntimeEvent::PauseMenuInput(_)
                | RuntimeEvent::StartNewGame
                | RuntimeEvent::ContinueGame
                | RuntimeEvent::OpenPauseMenu
                | RuntimeEvent::OpenMenuFromExplore
                | RuntimeEvent::OpenDialogState(_)
                | RuntimeEvent::OpenShopById(_)
                | RuntimeEvent::RestoreSessionStats
                | RuntimeEvent::ApplyDialogAction(_)
                | RuntimeEvent::ApplyDialogTransition(_)
        )
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::Tick
            | RuntimeEvent::KeyDown(_)
            | RuntimeEvent::KeyUp(_)
            | RuntimeEvent::OverlayCloseRequested
            | RuntimeEvent::GameOverConfirmRequested
            | RuntimeEvent::ErrorConfirmRequested
            | RuntimeEvent::UpdateLoading
            | RuntimeEvent::UpdateMovement
            | RuntimeEvent::UpdateCombat
            | RuntimeEvent::MenuInput(_)
            | RuntimeEvent::ExploreInput(_)
            | RuntimeEvent::InventoryInput(_)
            | RuntimeEvent::DialogInput(_)
            | RuntimeEvent::ShopInput(_)
            | RuntimeEvent::PauseMenuInput(_) => {}
            RuntimeEvent::StartNewGame => {
                let (state, session, intro) = crate::game::lifecycle::start_new_game(&engine.data);
                engine.enter_session(state, session, intro);
            }
            RuntimeEvent::ContinueGame => {
                let (state, session, intro) = crate::game::lifecycle::continue_game(&engine.data);
                engine.enter_session(state, session, intro);
            }
            RuntimeEvent::OpenPauseMenu => {
                engine.ui.pause_menu.reset();
                engine.transition_to(GameState::PauseMenu);
            }
            RuntimeEvent::OpenMenuFromExplore => {
                let s = engine
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow!("No active session"))?;
                let _ = crate::game::save_game(&s.player);
                engine.ui.menu.set_menu(MenuState::new(has_save_data()));
                engine.transition_to(GameState::Menu);
            }
            RuntimeEvent::OpenDialogState(dialog_state) => {
                engine.ui.dialog.open(dialog_state.clone());
                engine.transition_to(GameState::Dialog);
            }
            RuntimeEvent::OpenShopById(shop_id) => {
                let _ = engine.open_shop_by_id(shop_id);
            }
            RuntimeEvent::RestoreSessionStats => {
                let s = engine
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.restore_stats();
            }
            RuntimeEvent::ApplyDialogAction(action) => {
                let s = engine
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                if let crate::game::DialogActionResult::OpenShop(shop_id) =
                    s.apply_dialog_action(&engine.data, action)
                {
                    let _ = engine.open_shop_by_id(&shop_id);
                }
            }
            RuntimeEvent::ApplyDialogTransition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = engine.ui.dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                    engine.transition_to(GameState::Dialog);
                }
                crate::game::DialogTransition::CloseToExplore => {
                    engine.ui.dialog.close();
                    engine.transition_to(GameState::Explore);
                }
            },
            _ => {}
        }

        Ok(())
    }
}

impl DomainEventApplier for SystemApplyHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Loading(_)
                | RuntimeEvent::Movement(_)
                | RuntimeEvent::Menu(_)
                | RuntimeEvent::Explore(_)
                | RuntimeEvent::Inventory(_)
                | RuntimeEvent::Dialog(_)
                | RuntimeEvent::Shop(_)
                | RuntimeEvent::PauseMenu(_)
                | RuntimeEvent::CombatPlayerAction(_)
                | RuntimeEvent::Transition(_)
                | RuntimeEvent::Exit(_)
                | RuntimeEvent::Combat(_)
        )
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::Loading(event) => engine.apply_update_loading(event.clone()),
            RuntimeEvent::Movement(event) => engine.apply_update_movement(event.clone())?,
            RuntimeEvent::Menu(event) => engine.apply_menu_event(*event)?,
            RuntimeEvent::Explore(event) => engine.apply_explore_event(event.clone())?,
            RuntimeEvent::Inventory(event) => engine.apply_inventory_event(event.clone())?,
            RuntimeEvent::Dialog(event) => engine.apply_dialog_event(event.clone())?,
            RuntimeEvent::Shop(event) => engine.apply_shop_event(event.clone())?,
            RuntimeEvent::PauseMenu(event) => engine.apply_pause_menu_event(*event)?,
            RuntimeEvent::Combat(_) => {}
            RuntimeEvent::CombatPlayerAction(action) => {
                let s = engine
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.apply_explore_action(&engine.data, *action);
            }
            RuntimeEvent::Transition(event) => engine.apply_transition_event(*event)?,
            RuntimeEvent::Exit(code) => wipi::kernel::exit(*code),
            _ => {}
        }

        Ok(())
    }
}

fn domain_resolvers() -> [&'static dyn DomainEventResolver; 2] {
    [&CORE_RESOLVE_HANDLER, &CASCADE_RESOLVE_HANDLER]
}

fn domain_appliers() -> [&'static dyn DomainEventApplier; 2] {
    [&CORE_APPLY_HANDLER, &SYSTEM_APPLY_HANDLER]
}

impl GameEngine {
    pub fn new() -> Self {
        Self {
            state: GameState::Loading(0),
            data: Rc::new(GameData::default()),
            session: None,
            ui: UiState::default(),
        }
    }

    pub fn on_keydown(&mut self, key: InputKey) {
        self.dispatch(RuntimeEvent::from(GameInput::KeyDown(key)));
    }

    pub fn on_keyup(&mut self, key: InputKey) {
        self.dispatch(RuntimeEvent::from(GameInput::KeyUp(key)));
    }

    pub fn tick_and_build_render_state(&mut self) -> RenderState {
        self.update();
        build_render_state(&self.state, self.session.as_ref(), &self.ui, &self.data)
    }

    fn update(&mut self) {
        self.dispatch(RuntimeEvent::from(GameInput::Tick));
    }

    fn resolve_ui_input_event(&mut self, event: &RuntimeEvent) -> Vec<RuntimeEvent> {
        self.ui
            .resolve_input_event(event, &self.state, self.session.as_ref())
    }

    fn transition_to(&mut self, next: GameState) {
        if next.requires_session() && self.session.is_none() {
            self.state = GameState::Error(alloc::format!(
                "Missing session for state transition: {:?}",
                next
            ));
            return;
        }

        if self.state.can_transition_to(&next) {
            self.state = next;
            if !self.state.requires_session() {
                self.session = None;
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

    fn apply_update_movement(&mut self, event: AppMovementEvent) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

        let AppMovementEvent::Tick(movement_event, tile_event) = event;
        s.apply_movement_tick(&self.data, movement_event, tile_event);
        Ok(())
    }

    fn apply_map_changed(&mut self) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.spawn_current_map_enemies(&self.data);
        Ok(())
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
        self.session = Some(session);
        self.transition_to(state);
        if let Err(e) =
            self.apply_with_handlers(RuntimeEvent::Transition(TransitionEvent::MapChanged))
        {
            self.state = GameState::Error(alloc::format!("{e}"));
            return;
        }
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

    fn resolve_update_loading_event(&mut self) -> Result<Vec<RuntimeEvent>> {
        let GameState::Loading(step) = self.state else {
            return Err(anyhow!("Invalid state: expected Loading"));
        };

        let load_result = crate::game::lifecycle::load_step(&mut self.data, step);
        Ok(vec![RuntimeEvent::Loading(
            crate::game::lifecycle::resolve_loading(step, load_result),
        )])
    }

    fn resolve_update_movement_event(&self) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;

        let movement = crate::game::movement::resolve_world_tick(
            &s.movement,
            &s.player,
            &s.combat.enemies,
            &self.data,
        );

        let mut events = Vec::with_capacity(if movement.map_changed { 2 } else { 1 });
        events.push(RuntimeEvent::Movement(AppMovementEvent::Tick(
            movement.movement_event,
            movement.tile_event,
        )));
        if movement.map_changed {
            events.push(RuntimeEvent::Transition(TransitionEvent::MapChanged));
        }
        Ok(events)
    }

    fn resolve_update_combat_event(&self) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;
        let Some(map) = self.data.find_map(&s.player.current_map_id) else {
            return Ok(Vec::new());
        };

        Ok(crate::game::combat::resolve_tick(
            &s.combat,
            s.player.x,
            s.player.y,
            s.player.total_def(),
            (s.skill_cooldowns, s.mp_regen_timer),
            map,
            &self.data.enemies,
        ))
    }

    fn resolve_with_handlers(&mut self, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        let mut derived = Vec::new();
        derived.extend(self.resolve_ui_input_event(event));
        for resolver in domain_resolvers() {
            if resolver.handles(event) {
                derived.extend(resolver.resolve(self, event)?);
            }
        }
        Ok(derived)
    }

    fn apply_with_handlers(&mut self, event: RuntimeEvent) -> Result<()> {
        for applier in domain_appliers() {
            if applier.handles(&event) {
                applier.apply(self, &event)?;
            }
        }
        if let Some(s) = self.session.as_mut()
            && s.handles_event(&event)
            && s.apply_runtime_event(&event)
        {
            self.transition_to(GameState::GameOver);
        }
        Ok(())
    }

    fn dispatch(&mut self, initial: RuntimeEvent) {
        let mut queue = VecDeque::from([initial]);
        let mut processed = 0usize;

        while let Some(event) = queue.pop_front() {
            processed += 1;
            if processed > 256 {
                self.state = GameState::Error(alloc::format!(
                    "Event queue overflow: processed {} events in one dispatch",
                    processed
                ));
                return;
            }

            let derived = match self.resolve_with_handlers(&event) {
                Ok(events) => events,
                Err(e) => {
                    self.state = GameState::Error(alloc::format!("{e}"));
                    return;
                }
            };

            if let Err(e) = self.apply_with_handlers(event) {
                self.state = GameState::Error(alloc::format!("{e}"));
                return;
            }

            for derived_event in derived {
                queue.push_back(derived_event);
            }
        }
    }

    fn resolve_menu_input(&self, intent: crate::game::MenuIntent) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Menu),
            "Invalid state: expected Menu"
        );

        Ok(crate::game::menu::resolve_many(
            self.ui.menu.selected,
            &self.ui.menu.state.items,
            intent,
        )
        .into_iter()
        .map(RuntimeEvent::Menu)
        .collect())
    }

    fn resolve_explore_input(
        &self,
        intent: crate::game::ExploreIntent,
    ) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;

        let is_peaceful = self
            .data
            .find_map(&s.player.current_map_id)
            .is_some_and(|map| map.peaceful);

        let mut events = Vec::new();
        for explore_event in crate::game::explore::resolve_many(is_peaceful, intent) {
            match explore_event {
                crate::game::ExploreEvent::None => {}
                crate::game::ExploreEvent::MoveDirection(direction) => {
                    events.push(RuntimeEvent::Explore(AppExploreEvent::MoveDirection(
                        direction,
                    )));
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
                        events.push(RuntimeEvent::Explore(AppExploreEvent::Npc(npc_event)));
                    } else if let Some(action) = fallback_action {
                        events.push(RuntimeEvent::Explore(AppExploreEvent::UseAction(action)));
                    }
                }
                crate::game::ExploreEvent::UseAction(action) => {
                    events.push(RuntimeEvent::Explore(AppExploreEvent::UseAction(action)));
                }
                crate::game::ExploreEvent::EnterPauseMenu => {
                    events.push(RuntimeEvent::Explore(AppExploreEvent::EnterPauseMenu));
                }
                crate::game::ExploreEvent::EnterMenu => {
                    events.push(RuntimeEvent::Explore(AppExploreEvent::EnterMenu));
                }
            }
        }
        Ok(events)
    }

    fn resolve_inventory_input(
        &self,
        intent: crate::game::InventoryIntent,
    ) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Inventory),
            "Invalid state: expected Inventory"
        );
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;

        Ok(crate::game::inventory::resolve_many(
            self.ui.inventory.selected,
            s.player.inventory.len(),
            intent,
        )
        .into_iter()
        .map(RuntimeEvent::Inventory)
        .collect())
    }

    fn resolve_dialog_input(&self, intent: crate::game::DialogIntent) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Dialog),
            "Invalid state: expected Dialog"
        );
        self.session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;

        Ok(
            crate::game::dialog::resolve_many(self.ui.dialog.state.as_ref(), intent)
                .into_iter()
                .map(RuntimeEvent::Dialog)
                .collect(),
        )
    }

    fn resolve_shop_input(&self, intent: ShopIntent) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::Shop),
            "Invalid state: expected Shop"
        );
        let s = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;
        let shop_items = self
            .ui
            .shop
            .state
            .as_ref()
            .map(|state| state.items.as_slice())
            .unwrap_or(&[]);

        Ok(
            crate::game::shop::resolve_many(intent, s.player.stats.gold, shop_items)
                .into_iter()
                .map(RuntimeEvent::Shop)
                .collect(),
        )
    }

    fn resolve_pause_menu_input(
        &self,
        intent: crate::game::PauseMenuIntent,
    ) -> Result<Vec<RuntimeEvent>> {
        ensure!(
            matches!(self.state, GameState::PauseMenu),
            "Invalid state: expected PauseMenu"
        );
        self.session
            .as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;

        Ok(crate::game::menu::resolve_pause_many(
            self.ui.pause_menu.selected,
            self.ui.pause_menu.state.items.len(),
            intent,
        )
        .into_iter()
        .map(RuntimeEvent::PauseMenu)
        .collect())
    }

    fn apply_menu_event(&mut self, event: MenuEvent) -> Result<()> {
        match event {
            MenuEvent::None => {}
            MenuEvent::SetSelected(selected) => self.ui.menu.set_selected(selected),
            MenuEvent::Action(_) => {}
        }
        Ok(())
    }

    fn apply_explore_event(&mut self, event: AppExploreEvent) -> Result<()> {
        match event {
            AppExploreEvent::MoveDirection(direction) => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.on_direction_pressed(direction);
            }
            AppExploreEvent::Npc(_)
            | AppExploreEvent::UseAction(_)
            | AppExploreEvent::EnterPauseMenu
            | AppExploreEvent::EnterMenu => {}
        }
        Ok(())
    }

    fn apply_inventory_event(&mut self, event: crate::game::InventoryEvent) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

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
        Ok(())
    }

    fn apply_dialog_event(&mut self, event: crate::game::DialogEvent) -> Result<()> {
        match event {
            crate::game::DialogEvent::None => {}
            crate::game::DialogEvent::Transition(_) | crate::game::DialogEvent::Action(_, _) => {}
        }
        Ok(())
    }

    fn apply_shop_event(&mut self, event: crate::game::ShopEvent) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

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
        Ok(())
    }

    fn apply_pause_menu_event(&mut self, event: crate::game::PauseMenuEvent) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

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
        Ok(())
    }

    fn apply_transition_event(&mut self, event: TransitionEvent) -> Result<()> {
        match event {
            TransitionEvent::MapChanged => self.apply_map_changed()?,
            TransitionEvent::ToExplore => self.transition_to(GameState::Explore),
            TransitionEvent::ToMenuFromGameOver => {
                self.transition_to(GameState::Menu);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            TransitionEvent::ReleaseMovementDirection(direction) => {
                self.apply_release_movement_direction(direction)?
            }
        }
        Ok(())
    }

    fn apply_release_movement_direction(&mut self, direction: Direction) -> Result<()> {
        ensure!(
            matches!(self.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.on_direction_released(direction);
        Ok(())
    }
}
