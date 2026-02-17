use alloc::collections::VecDeque;
use alloc::format;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::{
    DomainEventResolver, GameData, GameEvent, GameEventKind, GameEventSubscriber, GameInput,
    GameState, InputKey, RenderFxState, RenderState, ResolveContext, UiEvent, UiEventApplier,
    UiInputEventResolver, UiState, WorldState, domain_resolvers,
};

pub struct GameEngine {
    state: GameState,
    data: Rc<GameData>,
    world: Option<WorldState>,
    ui: UiState,
    render_fx: RenderFxState,
    render_state: RenderState,
    resolver_buckets: Vec<Vec<&'static dyn DomainEventResolver>>,
    pending_inputs: VecDeque<GameInput>,
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
            world: None,
            ui: UiState::default(),
            render_fx: RenderFxState::default(),
            render_state: RenderState::Loading { step: 0 },
            resolver_buckets,
            pending_inputs: VecDeque::with_capacity(32),
        }
    }

    pub fn on_keydown(&mut self, key: InputKey) {
        self.pending_inputs.push_back(GameInput::KeyDown(key));
    }

    pub fn on_keyup(&mut self, key: InputKey) {
        self.pending_inputs.push_back(GameInput::KeyUp(key));
    }

    pub fn tick(&mut self) {
        self.update();
    }

    pub fn render_state(&self) -> &RenderState {
        &self.render_state
    }

    fn update(&mut self) {
        let mut initial_events = Vec::with_capacity(16);
        let mut pending = VecDeque::with_capacity(32);
        core::mem::swap(&mut pending, &mut self.pending_inputs);
        while let Some(input) = pending.pop_front() {
            let ui_events = self
                .ui
                .resolve_input(input, &self.state, self.world.as_ref());
            let initial = self.apply_ui_events(ui_events);
            initial_events.extend(initial);
        }
        self.pending_inputs = pending;
        if self.render_fx.tick() {
            self.render_state.apply_tick(&self.render_fx);
        }
        initial_events.extend(self.resolve_tick_game_events());
        if let Err(e) = self.dispatch_game_events(initial_events) {
            self.state = GameState::Error(format!("{e}"));
        }
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
            self.ui.apply_ui_event(self.world.as_ref(), event, &mut out);
        }
        if out.is_empty() {
            self.render_state
                .apply_ui_patch(&self.ui, self.world.as_ref());
        }
        out
    }

    fn resolve_with_handlers(&mut self, event: &GameEvent, out: &mut Vec<GameEvent>) -> Result<()> {
        out.clear();
        if matches!(event, GameEvent::UpdateCombat)
            && let Some(world) = self.world.as_ref()
        {
            out.reserve(world.combat.enemies.len() * 4 + 16);
        }
        let bucket = &self.resolver_buckets[event.kind().as_usize()];
        for resolver in bucket {
            let ctx = ResolveContext {
                state: &self.state,
                data: &self.data,
                world: self.world.as_ref(),
                ui: &self.ui,
            };
            (*resolver).resolve(&ctx, event, out)?;
        }
        Ok(())
    }

    fn apply_with_handlers(&mut self, event: GameEvent) -> Result<()> {
        let event = self.state.apply_with_data(&mut self.data, event)?;

        let is_session_event = matches!(event, GameEvent::World(_));

        if matches!(event, GameEvent::World(crate::game::WorldEvent::Create)) {
            self.world = Some(WorldState::empty());
        }

        if let GameEvent::Exit(code) = &event {
            wipi::kernel::exit(*code);
        }

        if self.state.requires_world() && self.world.is_none() {
            self.state = GameState::Error(format!(
                "Missing world for state transition: {:?}",
                self.state
            ));
            return Ok(());
        }

        if matches!(
            event,
            GameEvent::Transition(crate::game::TransitionEvent::ToMenu)
                | GameEvent::Transition(crate::game::TransitionEvent::ToMenuFromGameOver)
        ) && !is_session_event
        {
            self.world = None;
        }

        if let Some(world) = self.world.as_mut() {
            world.apply_domain_event(&self.data, &self.state, &event)?;
        }

        if !matches!(self.state, GameState::Error(_)) && self.ui.subscribes(event.kind()) {
            self.ui.apply_game_event(self.world.as_ref(), &event)?;
        }

        if self.render_fx.apply_event(&event) {
            self.render_state.apply_tick(&self.render_fx);
        }
        self.render_state.apply_event(
            &self.state,
            self.world.as_ref(),
            &self.ui,
            &self.data,
            &event,
            &self.render_fx,
        );
        Ok(())
    }

    fn dispatch_game_events(&mut self, initial_events: Vec<GameEvent>) -> Result<()> {
        if initial_events.is_empty() {
            return Ok(());
        }
        let mut queue = VecDeque::with_capacity(128);
        for event in initial_events {
            queue.push_back(event);
        }
        let mut derived = Vec::with_capacity(32);
        let mut processed = 0usize;

        while let Some(event) = queue.pop_front() {
            processed += 1;
            if processed > 256 {
                return Err(anyhow!(
                    "Event queue overflow: processed {} events in one dispatch",
                    processed
                ));
            }

            self.resolve_with_handlers(&event, &mut derived)?;

            self.apply_with_handlers(event)?;

            for derived_event in derived.drain(..) {
                queue.push_back(derived_event);
            }
        }
        Ok(())
    }
}
