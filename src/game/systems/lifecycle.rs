use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{DialogState, GameData, GameEvent, GameState};

#[derive(Clone)]
pub enum LoadingEvent {
    Advance(usize),
    Loaded,
    Error(String),
}

#[derive(Clone, Copy)]
pub enum LifecycleEvent {
    ResetUi,
    SetupNewGame,
    SetupContinue,
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
struct StartContinueResolver;

static UPDATE_LOADING_RESOLVER: UpdateLoadingResolver = UpdateLoadingResolver;
static START_CONTINUE_RESOLVER: StartContinueResolver = StartContinueResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![&UPDATE_LOADING_RESOLVER, &START_CONTINUE_RESOLVER]
}

impl DomainEventResolver for UpdateLoadingResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::UpdateLoading)
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, _event: &GameEvent) -> Result<Vec<GameEvent>> {
        let step = if let GameState::Loading(step) = ctx.state {
            *step
        } else {
            return Err(anyhow!("Invalid state: expected Loading"));
        };

        let load_result = load_step(ctx.data, step);

        Ok(alloc::vec![GameEvent::Loading(resolve_loading(
            step,
            load_result,
        ))])
    }
}

impl DomainEventResolver for StartContinueResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::StartNewGame | GameEvent::ContinueGame)
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let mut out = Vec::new();
        out.push(GameEvent::Lifecycle(LifecycleEvent::ResetUi));

        match event {
            GameEvent::StartNewGame => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::SetupNewGame));
                if let Some(dialog_state) = intro_dialog_state(ctx.data()) {
                    out.push(GameEvent::OpenDialogState(dialog_state));
                }
            }
            GameEvent::ContinueGame => {
                out.push(GameEvent::Lifecycle(LifecycleEvent::SetupContinue));
            }
            _ => {}
        }

        Ok(out)
    }
}

fn intro_dialog_state(data: &GameData) -> Option<DialogState> {
    let (dialog_id, npc_name) = data.newgame.intro_dialog.as_ref()?;
    let dialog = data.find_dialog(dialog_id)?;
    Some(DialogState::from_dialog(npc_name.clone(), dialog))
}
