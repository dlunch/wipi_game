use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameData, GameState, RuntimeEvent};

#[derive(Clone)]
pub enum LoadingEvent {
    Advance(usize),
    Loaded,
    Error(String),
}

pub fn resolve_loading(step: usize, load_result: Result<bool, String>) -> LoadingEvent {
    match load_result {
        Ok(true) => LoadingEvent::Loaded,
        Ok(false) => LoadingEvent::Advance(step + 1),
        Err(e) => LoadingEvent::Error(e),
    }
}

pub fn load_step(data: &mut Rc<GameData>, step: usize) -> Result<bool, String> {
    let Some(data_mut) = Rc::get_mut(data) else {
        return Err(String::from("Load error: data is shared"));
    };

    data_mut
        .load_step(step)
        .map_err(|e| format!("Load error: {}", e))
}

struct UpdateLoadingResolver;

static UPDATE_LOADING_RESOLVER: UpdateLoadingResolver = UpdateLoadingResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&UPDATE_LOADING_RESOLVER]
}

impl DomainEventResolver for UpdateLoadingResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::UpdateLoading)
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<Vec<RuntimeEvent>> {
        let step = if let GameState::Loading(step) = ctx.state {
            *step
        } else {
            return Err(anyhow!("Invalid state: expected Loading"));
        };

        let load_result = load_step(ctx.data, step);

        Ok(alloc::vec![RuntimeEvent::Loading(resolve_loading(
            step,
            load_result,
        ))])
    }
}
