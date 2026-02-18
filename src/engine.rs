use alloc::collections::VecDeque;
use alloc::format;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::game::{
    DomainEventResolver, GameData, GameEvent, GameEventKind, GameEventSubscriber, GameInput,
    GameState, InputKey, RenderFxState, RenderState, SpriteAtlas, UiEvent, UiEventApplier,
    UiInputEventResolver, UiState, WorldSlot, apply_effects, domain_resolvers,
};

pub struct GameEngine {
    state: GameState,
    data: Rc<GameData>,
    world: WorldSlot,
    ui: UiState,
    sprites: SpriteAtlas,
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
            world: WorldSlot::empty(),
            ui: UiState::default(),
            sprites: SpriteAtlas::load_default(),
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

    pub fn sprite_atlas(&self) -> &SpriteAtlas {
        &self.sprites
    }

    fn update(&mut self) {
        let mut initial_events = Vec::with_capacity(16);
        let mut pending = VecDeque::with_capacity(32);
        core::mem::swap(&mut pending, &mut self.pending_inputs);
        while let Some(input) = pending.pop_front() {
            let ui_events = self
                .ui
                .resolve_input(input, &self.state, self.world.as_ref());
            self.apply_ui_events(ui_events, &mut initial_events);
        }
        self.render_state
            .apply_ui_patch(&self.ui, self.world.as_ref());
        self.pending_inputs = pending;
        if self.render_fx.tick() {
            self.render_state.apply_tick(&self.render_fx);
        }
        self.resolve_tick_game_events(&mut initial_events);
        if let Err(e) = self.dispatch_game_events(initial_events) {
            self.state = GameState::Error(format!("{e}"));
            self.render_state.apply_state(
                &self.state,
                self.world.as_ref(),
                &self.ui,
                &self.data,
                &self.render_fx,
            );
        }
    }

    fn resolve_tick_game_events(&self, out: &mut Vec<GameEvent>) {
        match self.state {
            GameState::Loading(_) => {
                out.push(GameEvent::Loading(crate::game::LoadingEvent::Tick));
            }
            GameState::Explore => {
                out.push(GameEvent::UpdateMovement);
                out.push(GameEvent::UpdateCombat);
            }
            _ => {}
        }
    }

    fn apply_ui_events(&mut self, ui_events: Vec<UiEvent>, out: &mut Vec<GameEvent>) {
        out.reserve(ui_events.len() * 2);
        for event in ui_events {
            self.ui.apply_ui_event(self.world.as_ref(), event, out);
        }
    }

    fn resolve_with_handlers(&self, event: &GameEvent, out: &mut Vec<GameEvent>) -> Result<()> {
        let bucket = &self.resolver_buckets[event.kind().as_usize()];
        for resolver in bucket {
            resolver.resolve(&self.data, self.world.as_ref(), event, out)?;
        }
        Ok(())
    }

    fn apply_with_handlers(&mut self, event: GameEvent) -> Result<bool> {
        let kind = event.kind();

        if self.state.subscribes(kind) {
            self.state.apply_event(&event)?;
        }

        self.world.apply_event(&event);

        ensure!(
            !self.state.requires_world() || self.world.as_ref().is_some(),
            "Missing world for state transition: {:?}",
            self.state
        );

        if let Some(world) = self.world.as_mut()
            && world.subscribes(kind)
        {
            world.apply_event(&self.data, &event)?;
        }

        if !matches!(self.state, GameState::Error(_)) && self.ui.subscribes(kind) {
            self.ui.apply_game_event(&event)?;
        }

        Ok(self
            .render_fx
            .apply_event(&self.state, self.world.as_ref(), &event))
    }

    fn dispatch_game_events(&mut self, initial_events: Vec<GameEvent>) -> Result<()> {
        let mut queue: VecDeque<GameEvent> = initial_events.into();
        let mut processed = 0usize;
        let mut derived = Vec::with_capacity(8);
        let mut render_fx_changed = false;

        while let Some(event) = queue.pop_front() {
            processed += 1;
            if processed > 256 {
                return Err(anyhow!(
                    "Event queue overflow: processed {} events in one dispatch",
                    processed
                ));
            }

            derived.clear();
            self.resolve_with_handlers(&event, &mut derived)?;
            let effect_events =
                apply_effects(&self.state, &mut self.data, self.world.as_ref(), &event)?;

            render_fx_changed |= self.apply_with_handlers(event)?;

            queue.extend(derived.drain(..));
            queue.extend(effect_events);
        }

        if processed > 0 {
            self.render_state.apply_state(
                &self.state,
                self.world.as_ref(),
                &self.ui,
                &self.data,
                &self.render_fx,
            );
        } else if render_fx_changed {
            self.render_state.apply_tick(&self.render_fx);
        }
        Ok(())
    }
}
