use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{
    ApplyContext, GameData, GameInput, GameState, InputKey, RenderState, ResolveContext,
    RuntimeEvent, SessionState, UiInputEventResolver, UiState, build_render_state, domain_appliers,
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
        let initial = self.resolve_ui_input_event(GameInput::KeyDown(key));
        self.dispatch_all(initial);
    }

    pub fn on_keyup(&mut self, key: InputKey) {
        let initial = self.resolve_ui_input_event(GameInput::KeyUp(key));
        self.dispatch_all(initial);
    }

    pub fn tick_and_build_render_state(&mut self) -> RenderState {
        self.update();
        build_render_state(&self.state, self.session.as_ref(), &self.ui, &self.data)
    }

    fn update(&mut self) {
        let initial = self.resolve_ui_input_event(GameInput::Tick);
        self.dispatch_all(initial);
    }

    fn resolve_ui_input_event(&mut self, input: GameInput) -> Vec<RuntimeEvent> {
        self.ui
            .resolve_input(input, &self.state, self.session.as_ref())
    }

    fn resolve_with_handlers(&mut self, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        let mut derived = Vec::new();
        for resolver in domain_resolvers() {
            if resolver.handles(event) {
                let mut ctx = ResolveContext {
                    state: &self.state,
                    data: &mut self.data,
                    session: self.session.as_ref(),
                    ui: &self.ui,
                };
                derived.extend(resolver.resolve(&mut ctx, event)?);
            }
        }
        Ok(derived)
    }

    fn apply_with_handlers(&mut self, event: RuntimeEvent) -> Result<()> {
        let mut ctx = ApplyContext {
            state: &mut self.state,
            data: &self.data,
            session: &mut self.session,
            ui: &mut self.ui,
        };
        for applier in domain_appliers() {
            if applier.handles(&event) {
                applier.apply(&mut ctx, &event)?;
            }
        }
        Ok(())
    }

    fn dispatch_all(&mut self, initial_events: Vec<RuntimeEvent>) {
        if initial_events.is_empty() {
            return;
        }

        let mut queue: VecDeque<RuntimeEvent> = initial_events.into();
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
