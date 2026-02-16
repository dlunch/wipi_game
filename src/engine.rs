use alloc::collections::VecDeque;
use alloc::format;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;

use crate::game::{
    GameData, GameEvent, GameInput, GameState, InputKey, RenderState, ResolveContext, SessionState,
    UiEvent, UiEventApplier, UiInputEventResolver, UiState, build_render_state, domain_resolvers,
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
        let ui_events = self.resolve_ui_input_event(GameInput::KeyDown(key));
        let initial = self.apply_ui_events(ui_events);
        self.dispatch_game_events(initial);
    }

    pub fn on_keyup(&mut self, key: InputKey) {
        let ui_events = self.resolve_ui_input_event(GameInput::KeyUp(key));
        let initial = self.apply_ui_events(ui_events);
        self.dispatch_game_events(initial);
    }

    pub fn tick_and_build_render_state(&mut self) -> RenderState {
        self.update();
        build_render_state(&self.state, self.session.as_ref(), &self.ui, &self.data)
    }

    fn update(&mut self) {
        let initial = self.resolve_tick_game_events();
        self.dispatch_game_events(initial);
    }

    fn resolve_ui_input_event(&mut self, input: GameInput) -> Vec<UiEvent> {
        self.ui
            .resolve_input(input, &self.state, self.session.as_ref())
    }

    fn resolve_tick_game_events(&self) -> Vec<GameEvent> {
        match self.state {
            GameState::Loading(_) => vec![GameEvent::UpdateLoading],
            GameState::Explore => vec![GameEvent::UpdateMovement, GameEvent::UpdateCombat],
            _ => Vec::new(),
        }
    }

    fn apply_ui_events(&mut self, ui_events: Vec<UiEvent>) -> Vec<GameEvent> {
        let mut out = Vec::new();
        for event in ui_events {
            out.extend(self.ui.apply_ui_event(self.session.as_ref(), event));
        }
        out
    }

    fn resolve_with_handlers(&mut self, event: &GameEvent) -> Result<Vec<GameEvent>> {
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

    fn apply_with_handlers(&mut self, event: GameEvent) -> Result<()> {
        if matches!(event, GameEvent::Session(crate::game::SessionEvent::Create)) {
            self.session = Some(SessionState::empty());
        }

        if let GameEvent::Exit(code) = &event {
            wipi::kernel::exit(*code);
        }

        self.state.apply_event(&event)?;

        if self.state.requires_session() && self.session.is_none() {
            self.state = GameState::Error(format!(
                "Missing session for state transition: {:?}",
                self.state
            ));
            return Ok(());
        }

        if !self.state.requires_session() {
            self.session = None;
        }

        if let Some(session) = self.session.as_mut() {
            session.apply_event(&self.data, &mut self.state, &event)?;
            session.leader.apply_event(&self.data, &event)?;
            session
                .movement
                .apply_event(&self.state, &mut session.leader, &event)?;
            session.combat.apply_event(&event)?;

            if matches!(
                event,
                GameEvent::PauseMenu(crate::game::PauseMenuEvent::SaveAndReturnExplore)
                    | GameEvent::OpenMenuFromExplore
            ) {
                let _ = crate::game::save_game(session);
            }
        }

        if !matches!(self.state, GameState::Error(_)) {
            self.ui.apply_game_event(self.session.as_ref(), &event)?;
        }
        Ok(())
    }

    fn dispatch_game_events(&mut self, initial_events: Vec<GameEvent>) {
        if initial_events.is_empty() {
            return;
        }

        let mut queue: VecDeque<GameEvent> = initial_events.into();
        let mut processed = 0usize;

        while let Some(event) = queue.pop_front() {
            processed += 1;
            if processed > 256 {
                self.state = GameState::Error(format!(
                    "Event queue overflow: processed {} events in one dispatch",
                    processed
                ));
                return;
            }

            let derived = match self.resolve_with_handlers(&event) {
                Ok(events) => events,
                Err(e) => {
                    self.state = GameState::Error(format!("{e}"));
                    return;
                }
            };

            if let Err(e) = self.apply_with_handlers(event) {
                self.state = GameState::Error(format!("{e}"));
                return;
            }

            for derived_event in derived {
                queue.push_back(derived_event);
            }
        }
    }
}
