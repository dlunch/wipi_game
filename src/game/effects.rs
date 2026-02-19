use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use super::save::{has_save_data, save_game};
use crate::game::game_data::GameData;
use crate::game::game_event::{GameEvent, GameEventKind, TransitionEvent};
use crate::game::state::GameState;
use crate::game::systems::lifecycle::{LifecycleEvent, LoadingEvent};
use crate::game::world::WorldState;

pub trait DomainEventEffect {
    fn subscribed_kinds(&self) -> &'static [GameEventKind];
    fn apply(
        &self,
        state: &GameState,
        data: &mut Rc<GameData>,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()>;
}

struct CoreEffects;

static CORE_EFFECTS: CoreEffects = CoreEffects;

pub fn domain_effects() -> Vec<&'static dyn DomainEventEffect> {
    vec![&CORE_EFFECTS]
}

impl DomainEventEffect for CoreEffects {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[
            GameEventKind::Tick,
            GameEventKind::SaveWorld,
            GameEventKind::Transition,
            GameEventKind::Exit,
        ]
    }

    fn apply(
        &self,
        state: &GameState,
        data: &mut Rc<GameData>,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::Tick => {
                let GameState::Loading(step) = state else {
                    return Ok(());
                };

                let data_mut =
                    Rc::get_mut(data).ok_or_else(|| anyhow!("Load error: data is shared"))?;
                let loaded = data_mut
                    .load_step(*step)
                    .map_err(|e| anyhow!("Load error: {}", e))?;
                let loading = if loaded {
                    out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                        has_save_data()?,
                    )));
                    LoadingEvent::Loaded
                } else {
                    LoadingEvent::Advance(*step + 1)
                };
                out.push(GameEvent::Loading(loading));
            }
            GameEvent::SaveWorld => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                save_game(world)?;
                out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                    true,
                )));
            }
            GameEvent::Transition(TransitionEvent::ToMenu) => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::SetMenuHasSaveData(
                    has_save_data()?,
                )));
            }
            GameEvent::Exit(code) => {
                wipi::kernel::exit(*code);
            }
            _ => {}
        }
        Ok(())
    }
}
