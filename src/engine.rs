use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::Direction;
use crate::game::{
    AppExploreEvent, AppMovementEvent, GameData, GameInput, GameIntent, GameState, InputKey,
    MenuAction, MenuEvent, MenuState, RenderState, RuntimeEvent, SceneIntent, SessionState,
    ShopIntent, SystemIntent, TransitionEvent, UiState, build_render_state, has_save_data,
};

pub struct GameEngine {
    state: GameState,
    data: Rc<GameData>,
    session: Option<SessionState>,
    ui: UiState,
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
            && self.session.is_some()
            && let Some(direction) = key.direction()
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

    fn apply_combat_event(&mut self, event: crate::game::CombatRuntimeEvent) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

        if s.apply_event(RuntimeEvent::Combat(event)) {
            self.transition_to(GameState::GameOver);
        }
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
        if let Err(e) = self.apply_event(RuntimeEvent::Transition(TransitionEvent::MapChanged)) {
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

    fn resolve_intent(&mut self, intent: GameIntent) -> Result<Vec<RuntimeEvent>> {
        match intent {
            GameIntent::System(system_intent) => match system_intent {
                SystemIntent::UpdateLoading => self.resolve_update_loading_intent(),
                SystemIntent::UpdateMovement => self.resolve_update_movement_intent(),
                SystemIntent::UpdateCombat => self.resolve_update_combat_intent(),
                SystemIntent::ReturnToExplore => {
                    Ok(vec![RuntimeEvent::Transition(TransitionEvent::ToExplore)])
                }
                SystemIntent::ReturnToMenuFromGameOver => Ok(vec![RuntimeEvent::Transition(
                    TransitionEvent::ToMenuFromGameOver,
                )]),
                SystemIntent::ReleaseMovementDirection(direction) => {
                    Ok(vec![RuntimeEvent::Transition(
                        TransitionEvent::ReleaseMovementDirection(direction),
                    )])
                }
                SystemIntent::Exit(code) => Ok(vec![RuntimeEvent::Exit(code)]),
            },
            GameIntent::Scene(scene_intent) => self.resolve_scene_intent(scene_intent),
        }
    }

    fn resolve_update_loading_intent(&mut self) -> Result<Vec<RuntimeEvent>> {
        let GameState::Loading(step) = self.state else {
            return Err(anyhow!("Invalid state: expected Loading"));
        };

        let load_result = crate::game::lifecycle::load_step(&mut self.data, step);
        Ok(vec![RuntimeEvent::Loading(
            crate::game::lifecycle::resolve_loading(step, load_result),
        )])
    }

    fn resolve_update_movement_intent(&self) -> Result<Vec<RuntimeEvent>> {
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

    fn resolve_update_combat_intent(&self) -> Result<Vec<RuntimeEvent>> {
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

    fn apply_event(&mut self, event: RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::Loading(event) => self.apply_update_loading(event),
            RuntimeEvent::Movement(event) => self.apply_update_movement(event)?,
            RuntimeEvent::Menu(event) => self.apply_menu_event(event)?,
            RuntimeEvent::Explore(event) => self.apply_explore_event(event)?,
            RuntimeEvent::Inventory(event) => self.apply_inventory_event(event)?,
            RuntimeEvent::Dialog(event) => self.apply_dialog_event(event)?,
            RuntimeEvent::Shop(event) => self.apply_shop_event(event)?,
            RuntimeEvent::PauseMenu(event) => self.apply_pause_menu_event(event)?,
            RuntimeEvent::Combat(event) => self.apply_combat_event(event)?,
            RuntimeEvent::Transition(event) => self.apply_transition_event(event)?,
            RuntimeEvent::Exit(code) => wipi::kernel::exit(code),
        }
        Ok(())
    }

    fn dispatch(&mut self, action: GameInput) {
        let intents = self.collect_intents(action);
        for intent in intents {
            let events = match self.resolve_intent(intent) {
                Ok(events) => events,
                Err(e) => {
                    self.state = GameState::Error(alloc::format!("{e}"));
                    return;
                }
            };
            for event in events {
                if let Err(e) = self.apply_event(event) {
                    self.state = GameState::Error(alloc::format!("{e}"));
                    return;
                }
            }
        }
    }

    fn resolve_scene_intent(&self, scene_intent: SceneIntent) -> Result<Vec<RuntimeEvent>> {
        match scene_intent {
            SceneIntent::Menu(intent) => self.resolve_menu_intent(intent),
            SceneIntent::Explore(intent) => self.resolve_explore_intent(intent),
            SceneIntent::Inventory(intent) => self.resolve_inventory_intent(intent),
            SceneIntent::Dialog(intent) => self.resolve_dialog_intent(intent),
            SceneIntent::Shop(intent) => self.resolve_shop_intent(intent),
            SceneIntent::PauseMenu(intent) => self.resolve_pause_menu_intent(intent),
        }
    }

    fn resolve_menu_intent(&self, intent: crate::game::MenuIntent) -> Result<Vec<RuntimeEvent>> {
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

    fn resolve_explore_intent(
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

    fn resolve_inventory_intent(
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

    fn resolve_dialog_intent(
        &self,
        intent: crate::game::DialogIntent,
    ) -> Result<Vec<RuntimeEvent>> {
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

    fn resolve_shop_intent(&self, intent: ShopIntent) -> Result<Vec<RuntimeEvent>> {
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

    fn resolve_pause_menu_intent(
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
                MenuAction::Exit => self.apply_event(RuntimeEvent::Exit(0))?,
            },
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
            AppExploreEvent::Npc(npc_event) => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                match npc_event {
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
                }
            }
            AppExploreEvent::UseAction(action) => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.apply_explore_action(&self.data, action);
            }
            AppExploreEvent::EnterPauseMenu => {
                self.ui.pause_menu.reset();
                self.transition_to(GameState::PauseMenu);
            }
            AppExploreEvent::EnterMenu => {
                let s = self
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow!("No active session"))?;
                let _ = crate::game::save_game(&s.player);
                self.ui.menu.set_menu(MenuState::new(has_save_data()));
                self.transition_to(GameState::Menu);
            }
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
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow!("No active session"))?;

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
                            return Ok(());
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
