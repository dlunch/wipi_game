use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{
    GameData, GameInput, GameState, InputKey, RenderState, RuntimeEvent, SessionEventApplier,
    SessionState, UiInputEventResolver, UiState, build_render_state, domain_appliers,
    domain_resolvers,
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

    pub(crate) fn state(&self) -> &GameState {
        &self.state
    }

    pub(crate) fn data(&self) -> &GameData {
        &self.data
    }

    pub(crate) fn data_rc(&self) -> Rc<GameData> {
        Rc::clone(&self.data)
    }

    pub(crate) fn replace_data(&mut self, data: Rc<GameData>) {
        self.data = data;
    }

    pub(crate) fn session(&self) -> Option<&SessionState> {
        self.session.as_ref()
    }

    pub(crate) fn session_mut(&mut self) -> Option<&mut SessionState> {
        self.session.as_mut()
    }

    pub(crate) fn ui(&self) -> &UiState {
        &self.ui
    }

    pub(crate) fn ui_mut(&mut self) -> &mut UiState {
        &mut self.ui
    }

    pub(crate) fn set_error(&mut self, message: alloc::string::String) {
        self.state = GameState::Error(message);
    }

    pub(crate) fn transition_to(&mut self, next: GameState) {
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

    fn dialog_state_from_intro(
        &self,
        intro: Option<crate::game::lifecycle::IntroDialogSpec>,
    ) -> Option<crate::game::DialogState> {
        let spec = intro?;
        let dialog = self.data.find_dialog(&spec.dialog_id)?;
        Some(crate::game::DialogState::from_dialog(spec.npc_name, dialog))
    }

    pub(crate) fn enter_session(
        &mut self,
        state: GameState,
        session: SessionState,
        intro: Option<crate::game::lifecycle::IntroDialogSpec>,
    ) {
        self.session = Some(session);
        self.transition_to(state);
        if let Err(e) = self.apply_with_handlers(RuntimeEvent::Transition(
            crate::game::TransitionEvent::MapChanged,
        )) {
            self.state = GameState::Error(alloc::format!("{e}"));
            return;
        }
        self.ui = UiState::default();
        self.ui.dialog.set(self.dialog_state_from_intro(intro));
    }

    pub(crate) fn open_shop_by_id(&mut self, shop_id: &str) -> bool {
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

    fn resolve_ui_input_event(&mut self, event: &RuntimeEvent) -> Vec<RuntimeEvent> {
        self.ui
            .resolve_input_event(event, &self.state, self.session.as_ref())
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
}
