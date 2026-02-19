use alloc::{collections::VecDeque, format, rc::Rc, vec, vec::Vec};

use anyhow::{Error, Result, anyhow, ensure};
use wipi::resource::Resource;

use crate::game::{
    effects::{DomainEventEffect, domain_effects},
    game_data::GameData,
    game_event::{GameEvent, GameEventKind, GameEventSubscriber},
    rendering::{RenderFxState, RenderState, SpriteAtlas},
    state::{GameState, WorldSlot},
    systems::{domain_resolvers, resolver::DomainEventResolver},
    ui::{
        apply::UiEventApplier,
        resolve::UiInputEventResolver,
        state::{GameInput, InputKey, UiState},
    },
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
    effect_buckets: Vec<Vec<&'static dyn DomainEventEffect>>,
    pending_inputs: VecDeque<GameInput>,
}

impl GameEngine {
    pub fn new() -> Self {
        let resolvers = domain_resolvers();
        let effects = domain_effects();
        let mut resolver_buckets = vec![Vec::new(); GameEventKind::COUNT];
        let mut effect_buckets = vec![Vec::new(); GameEventKind::COUNT];
        for resolver in resolvers {
            for kind in resolver.subscribed_kinds() {
                resolver_buckets[kind.as_usize()].push(resolver);
            }
        }
        for effect in effects {
            for kind in effect.subscribed_kinds() {
                effect_buckets[kind.as_usize()].push(effect);
            }
        }

        Self {
            state: GameState::Loading(0),
            data: Rc::new(GameData::new(|path| {
                let resource = Resource::new(path)
                    .map_err(|e| anyhow!("failed to open resource '{}': {:?}", path, e))?;
                Ok(resource.read().to_vec())
            })),
            world: WorldSlot::empty(),
            ui: UiState::default(),
            sprites: SpriteAtlas::load_default(),
            render_fx: RenderFxState::default(),
            render_state: RenderState::Loading { step: 0 },
            resolver_buckets,
            effect_buckets,
            pending_inputs: VecDeque::with_capacity(32),
        }
    }

    pub fn on_keydown(&mut self, key: InputKey) {
        self.pending_inputs.push_back(GameInput::KeyDown(key));
    }

    pub fn on_keyup(&mut self, key: InputKey) {
        self.pending_inputs.push_back(GameInput::KeyUp(key));
    }

    pub fn tick(&mut self) -> bool {
        match self.tick_inner() {
            Ok(needs_repaint) => needs_repaint,
            Err(err) => self.handle_tick_error(err),
        }
    }

    fn tick_inner(&mut self) -> Result<bool> {
        let mut needs_repaint = false;
        let mut input_events = Vec::with_capacity(16);
        while let Some(input) = self.pending_inputs.pop_front() {
            let ui_events = self
                .ui
                .resolve_input(input, &self.state, self.world.as_ref());
            input_events.reserve(ui_events.len() * 2);
            for event in ui_events {
                self.ui
                    .apply_ui_event(self.world.as_ref(), event, &mut input_events)?;
            }
        }
        needs_repaint |= self
            .render_state
            .apply_ui_patch(&self.ui, self.world.as_ref())?;

        self.render_fx.tick();
        needs_repaint |= self.render_state.apply_tick(&self.render_fx);

        if !input_events.is_empty() {
            needs_repaint |= self.dispatch_game_events(input_events)?;
        }

        if matches!(self.state, GameState::Loading(_) | GameState::Explore) {
            needs_repaint |= self.dispatch_game_events(vec![GameEvent::Tick])?;
        }

        Ok(needs_repaint)
    }

    pub fn render_state(&self) -> &RenderState {
        &self.render_state
    }

    pub fn sprite_atlas(&self) -> &SpriteAtlas {
        &self.sprites
    }

    pub fn render_fx(&self) -> &RenderFxState {
        &self.render_fx
    }

    fn resolve_event(&self, event: &GameEvent, out: &mut Vec<GameEvent>) -> Result<()> {
        if matches!(event, GameEvent::Tick) && !self.world.is_active() {
            return Ok(());
        }
        let bucket = &self.resolver_buckets[event.kind().as_usize()];
        for resolver in bucket {
            resolver.resolve(&self.data, self.world.as_ref(), event, out)?;
        }
        Ok(())
    }

    fn apply_event(&mut self, event: &GameEvent) -> Result<()> {
        let kind = event.kind();

        if self.state.subscribes(kind) {
            self.state.apply_event(event)?;
        }

        self.world.apply_event(event);

        ensure!(
            !self.state.requires_world() || self.world.is_active(),
            "Missing world for state transition: {:?}",
            self.state
        );

        if let Some(world) = self.world.as_mut()
            && world.subscribes(kind)
        {
            world.apply_event(&self.data, event)?;
        }

        if !matches!(self.state, GameState::Error(_)) && self.ui.subscribes(kind) {
            self.ui.apply_game_event(event)?;
        }

        Ok(())
    }

    fn dispatch_game_events(&mut self, initial_events: Vec<GameEvent>) -> Result<bool> {
        let mut needs_repaint = false;
        let mut queue = VecDeque::from(initial_events);
        let mut derived = Vec::with_capacity(8);

        while let Some(event) = queue.pop_front() {
            self.resolve_event(&event, &mut derived)?;
            let effect_bucket = &self.effect_buckets[event.kind().as_usize()];
            for effect in effect_bucket {
                effect.apply(
                    &self.state,
                    &mut self.data,
                    self.world.as_ref(),
                    &event,
                    &mut derived,
                )?;
            }

            needs_repaint |= self.apply_and_patch(&event)?;

            queue.extend(derived.drain(..));
        }

        Ok(needs_repaint)
    }

    fn apply_and_patch(&mut self, event: &GameEvent) -> Result<bool> {
        self.apply_event(event)?;

        let render_fx_changed =
            self.render_fx
                .apply_event(&self.state, self.world.as_ref(), event)?;

        let mut needs_repaint = self.render_state.apply_game_event_patch(
            event,
            &self.state,
            self.world.as_ref(),
            &self.ui,
            &self.data,
            &self.render_fx,
        )?;

        if render_fx_changed {
            needs_repaint |= self.render_state.apply_tick(&self.render_fx);
        }

        Ok(needs_repaint)
    }

    fn handle_tick_error(&mut self, err: Error) -> bool {
        let error_event = GameEvent::FatalError(format!("{err}"));
        if let Err(apply_err) = self.apply_and_patch(&error_event) {
            self.state = GameState::Error(format!("{apply_err}"));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use alloc::{rc::Rc, vec, vec::Vec};

    use anyhow::Result;

    use super::GameEngine;
    use crate::game::{
        effects::DomainEventEffect,
        game_data::GameData,
        game_event::{GameEvent, GameEventKind, TransitionEvent, WorldEvent},
        state::GameState,
        systems::resolver::DomainEventResolver,
        world::WorldState,
    };

    struct CreateWorldResolver;

    static CREATE_WORLD_RESOLVER: CreateWorldResolver = CreateWorldResolver;

    impl DomainEventResolver for CreateWorldResolver {
        fn subscribed_kinds(&self) -> &'static [GameEventKind] {
            &[GameEventKind::Exit]
        }

        fn resolve(
            &self,
            _data: &Rc<GameData>,
            _world: Option<&WorldState>,
            _event: &GameEvent,
            out: &mut Vec<GameEvent>,
        ) -> Result<()> {
            out.push(GameEvent::World(WorldEvent::CreateWorld));
            Ok(())
        }
    }

    struct MenuTransitionEffect;

    static MENU_TRANSITION_EFFECT: MenuTransitionEffect = MenuTransitionEffect;

    impl DomainEventEffect for MenuTransitionEffect {
        fn subscribed_kinds(&self) -> &'static [GameEventKind] {
            &[GameEventKind::Exit]
        }

        fn apply(
            &self,
            _state: &GameState,
            _data: &mut Rc<GameData>,
            _world: Option<&WorldState>,
            _event: &GameEvent,
            out: &mut Vec<GameEvent>,
        ) -> Result<()> {
            out.push(GameEvent::Transition(TransitionEvent::ToMenu));
            Ok(())
        }
    }

    fn clear_dispatch_buckets(engine: &mut GameEngine) {
        engine.resolver_buckets = vec![Vec::new(); GameEventKind::COUNT];
        engine.effect_buckets = vec![Vec::new(); GameEventKind::COUNT];
    }

    #[test]
    fn dispatch_applies_resolver_and_effect_events_in_order() -> Result<()> {
        let mut engine = GameEngine::new();
        clear_dispatch_buckets(&mut engine);
        engine.resolver_buckets[GameEventKind::Exit.as_usize()].push(&CREATE_WORLD_RESOLVER);
        engine.effect_buckets[GameEventKind::Exit.as_usize()].push(&MENU_TRANSITION_EFFECT);

        engine.dispatch_game_events(vec![GameEvent::Exit(0)])?;

        assert!(matches!(engine.state, GameState::Menu));
        assert!(!engine.world.is_active());
        Ok(())
    }

    #[test]
    fn tick_transitions_to_error_when_world_is_missing_for_explore() {
        let mut engine = GameEngine::new();
        engine.state = GameState::Explore;
        engine.world = crate::game::state::WorldSlot::empty();

        let repaint = engine.tick();

        assert!(repaint);
        assert!(matches!(engine.state, GameState::Error(_)));
    }

    #[test]
    fn resolve_tick_without_world_is_ignored() -> Result<()> {
        let engine = GameEngine::new();
        let mut derived = Vec::new();

        engine.resolve_event(&GameEvent::Tick, &mut derived)?;

        assert!(derived.is_empty());
        Ok(())
    }
}
