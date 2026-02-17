use alloc::collections::VecDeque;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::Result;
use core::mem;

use crate::game::{
    DomainEventResolver, GameData, GameEvent, GameEventKind, GameEventSubscriber, GameInput,
    GameState, InputKey, RenderState, ResolveContext, SessionState, UiEvent, UiEventApplier,
    UiInputEventResolver, UiState, build_render_state, domain_resolvers,
};

pub struct GameEngine {
    state: GameState,
    data: Rc<GameData>,
    session: Option<SessionState>,
    ui: UiState,
    resolver_buckets: Vec<Vec<&'static dyn DomainEventResolver>>,
    event_queue: VecDeque<GameEvent>,
    derived_events: Vec<GameEvent>,
}

impl GameEngine {
    pub fn new() -> Self {
        let resolvers = domain_resolvers();
        let mut resolver_buckets: Vec<Vec<&'static dyn DomainEventResolver>> =
            vec![Vec::new(); GameEventKind::COUNT];
        for resolver in resolvers {
            for kind in resolver.subscribed_kinds() {
                resolver_buckets[kind.as_usize()].push(resolver);
            }
        }

        Self {
            state: GameState::Loading(0),
            data: Rc::new(GameData::default()),
            session: None,
            ui: UiState::default(),
            resolver_buckets,
            event_queue: VecDeque::with_capacity(128),
            derived_events: Vec::with_capacity(32),
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
        let mut out = Vec::with_capacity(ui_events.len() * 2);
        for event in ui_events {
            self.ui
                .apply_ui_event(self.session.as_ref(), event, &mut out);
        }
        out
    }

    fn resolve_with_handlers(&mut self, event: &GameEvent, out: &mut Vec<GameEvent>) -> Result<()> {
        out.clear();
        if matches!(event, GameEvent::UpdateCombat)
            && let Some(session) = self.session.as_ref()
        {
            out.reserve(session.combat.enemies.len() * 4 + 16);
        }
        let bucket = &self.resolver_buckets[event.kind().as_usize()];
        for resolver in bucket {
            let mut ctx = ResolveContext {
                state: &self.state,
                data: &mut self.data,
                session: self.session.as_ref(),
                ui: &self.ui,
            };
            (*resolver).resolve(&mut ctx, event, out)?;
        }
        Ok(())
    }

    fn apply_with_handlers(&mut self, event: GameEvent) -> Result<()> {
        let is_session_event = matches!(event, GameEvent::Session(_));

        if matches!(event, GameEvent::Session(crate::game::SessionEvent::Create)) {
            self.session = Some(SessionState::empty());
        }

        if let GameEvent::Exit(code) = &event {
            wipi::kernel::exit(*code);
        }

        if self.state.subscribes(event.kind()) {
            self.state.apply_event(&event)?;
        }

        if self.state.requires_session() && self.session.is_none() {
            self.state = GameState::Error(format!(
                "Missing session for state transition: {:?}",
                self.state
            ));
            return Ok(());
        }

        if !self.state.requires_session() && !is_session_event {
            self.session = None;
        }

        if let Some(session) = self.session.as_mut() {
            if session.subscribes(event.kind()) {
                session.apply_event(&self.data, &mut self.state, &event)?;
            }
            if session.leader.subscribes(event.kind()) {
                session.leader.apply_event(&self.data, &event)?;
            }
            if session.movement.subscribes(event.kind()) {
                session
                    .movement
                    .apply_event(&self.state, &mut session.leader, &event)?;
            }
            if session.combat.subscribes(event.kind()) {
                session.combat.apply_event(&event)?;
            }

            if matches!(event, GameEvent::SaveSession) {
                let _ = crate::game::save_game(session);
            }
        }

        if !matches!(self.state, GameState::Error(_)) && self.ui.subscribes(event.kind()) {
            self.ui.apply_game_event(self.session.as_ref(), &event)?;
        }
        Ok(())
    }

    fn dispatch_game_events(&mut self, initial_events: Vec<GameEvent>) {
        if initial_events.is_empty() {
            return;
        }
        let mut queue = mem::take(&mut self.event_queue);
        queue.clear();
        for event in initial_events {
            queue.push_back(event);
        }
        let mut derived = mem::take(&mut self.derived_events);
        derived.clear();
        let mut processed = 0usize;
        let mut error_message: Option<String> = None;

        while let Some(event) = queue.pop_front() {
            processed += 1;
            if processed > 256 {
                error_message = Some(format!(
                    "Event queue overflow: processed {} events in one dispatch",
                    processed
                ));
                break;
            }

            if let Err(e) = self.resolve_with_handlers(&event, &mut derived) {
                error_message = Some(format!("{e}"));
                break;
            }

            if let Err(e) = self.apply_with_handlers(event) {
                error_message = Some(format!("{e}"));
                break;
            }

            for derived_event in derived.drain(..) {
                queue.push_back(derived_event);
            }
        }

        self.event_queue = queue;
        self.derived_events = derived;

        if let Some(message) = error_message {
            self.state = GameState::Error(message);
        }
    }
}
