use alloc::collections::VecDeque;
use alloc::format;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Error, Result, ensure};

use crate::game::effects::{DomainEventEffect, domain_effects};
use crate::game::game_data::GameData;
use crate::game::game_event::{GameEvent, GameEventKind, GameEventSubscriber};
use crate::game::rendering::{RenderFxState, RenderState, SpriteAtlas};
use crate::game::state::{GameState, WorldSlot};
use crate::game::systems::domain_resolvers;
use crate::game::systems::resolver::DomainEventResolver;
use crate::game::ui::apply::UiEventApplier;
use crate::game::ui::resolve::UiInputEventResolver;
use crate::game::ui::state::{GameInput, InputKey, UiState};

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
        let mut resolver_buckets: Vec<Vec<&'static dyn DomainEventResolver>> =
            vec![Vec::new(); GameEventKind::COUNT];
        let mut effect_buckets: Vec<Vec<&'static dyn DomainEventEffect>> =
            vec![Vec::new(); GameEventKind::COUNT];
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
            data: Rc::new(GameData::default()),
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

    fn resolve_with_handlers(&self, event: &GameEvent, out: &mut Vec<GameEvent>) -> Result<()> {
        if matches!(event, GameEvent::Tick) && !self.world.is_active() {
            return Ok(());
        }
        let bucket = &self.resolver_buckets[event.kind().as_usize()];
        for resolver in bucket {
            resolver.resolve(&self.data, self.world.as_ref(), event, out)?;
        }
        Ok(())
    }

    fn apply_with_handlers(&mut self, event: &GameEvent) -> Result<()> {
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
        let mut queue: VecDeque<GameEvent> = initial_events.into();
        let mut derived = Vec::with_capacity(8);

        while let Some(event) = queue.pop_front() {
            self.resolve_with_handlers(&event, &mut derived)?;
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

            needs_repaint |= self.apply_and_patch_event(&event)?;

            queue.extend(derived.drain(..));
        }

        Ok(needs_repaint)
    }

    fn apply_and_patch_event(&mut self, event: &GameEvent) -> Result<bool> {
        self.apply_with_handlers(event)?;

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
        if let Err(apply_err) = self.apply_and_patch_event(&error_event) {
            self.state = GameState::Error(format!("{apply_err}"));
        }
        true
    }
}
