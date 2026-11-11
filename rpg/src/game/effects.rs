use alloc::{vec, vec::Vec};

use anyhow::{Result, anyhow};

use super::save::{has_save_data, save_game};
use crate::game::{
    game_event::{GameEvent, GameEventKind, TransitionEvent},
    systems::lifecycle::LifecycleEvent,
    world::WorldState,
};

pub trait DomainEventEffect {
    fn subscribed_kinds(&self) -> &'static [GameEventKind];
    fn apply(
        &self,
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
            GameEventKind::SaveWorld,
            GameEventKind::Transition,
            GameEventKind::Exit,
        ]
    }

    fn apply(
        &self,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
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
