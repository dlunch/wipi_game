use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::engine::GameEngine;
use crate::game::{
    AppExploreEvent, AppMovementEvent, ExploreEvent, GameState, MenuAction, MenuEvent,
    RuntimeEvent, ShopIntent, TransitionEvent,
};

pub trait DomainEventResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool;
    fn resolve(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>>;
}

pub trait DomainEventApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool;
    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()>;
}

pub struct CoreResolveHandler;
pub struct CascadeResolveHandler;
pub struct CoreApplyHandler;
pub struct SystemApplyHandler;

static CORE_RESOLVE_HANDLER: CoreResolveHandler = CoreResolveHandler;
static CASCADE_RESOLVE_HANDLER: CascadeResolveHandler = CascadeResolveHandler;
static CORE_APPLY_HANDLER: CoreApplyHandler = CoreApplyHandler;
static SYSTEM_APPLY_HANDLER: SystemApplyHandler = SystemApplyHandler;

pub fn domain_resolvers() -> [&'static dyn DomainEventResolver; 2] {
    [&CORE_RESOLVE_HANDLER, &CASCADE_RESOLVE_HANDLER]
}

pub fn domain_appliers() -> [&'static dyn DomainEventApplier; 2] {
    [&CORE_APPLY_HANDLER, &SYSTEM_APPLY_HANDLER]
}

impl DomainEventResolver for CoreResolveHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::OverlayCloseRequested
                | RuntimeEvent::GameOverConfirmRequested
                | RuntimeEvent::ErrorConfirmRequested
                | RuntimeEvent::UpdateLoading
                | RuntimeEvent::UpdateMovement
                | RuntimeEvent::UpdateCombat
                | RuntimeEvent::MenuInput(_)
                | RuntimeEvent::ExploreInput(_)
                | RuntimeEvent::InventoryInput(_)
                | RuntimeEvent::DialogInput(_)
                | RuntimeEvent::ShopInput(_)
                | RuntimeEvent::PauseMenuInput(_)
        )
    }

    fn resolve(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        match event {
            RuntimeEvent::OverlayCloseRequested => {
                Ok(vec![RuntimeEvent::Transition(TransitionEvent::ToExplore)])
            }
            RuntimeEvent::GameOverConfirmRequested => Ok(vec![RuntimeEvent::Transition(
                TransitionEvent::ToMenuFromGameOver,
            )]),
            RuntimeEvent::ErrorConfirmRequested => Ok(vec![RuntimeEvent::Exit(1)]),
            RuntimeEvent::UpdateLoading => resolve_update_loading_event(engine),
            RuntimeEvent::UpdateMovement => resolve_update_movement_event(engine),
            RuntimeEvent::UpdateCombat => resolve_update_combat_event(engine),
            RuntimeEvent::MenuInput(intent) => resolve_menu_input(engine, *intent),
            RuntimeEvent::ExploreInput(intent) => resolve_explore_input(engine, *intent),
            RuntimeEvent::InventoryInput(intent) => resolve_inventory_input(engine, *intent),
            RuntimeEvent::DialogInput(intent) => resolve_dialog_input(engine, *intent),
            RuntimeEvent::ShopInput(intent) => resolve_shop_input(engine, *intent),
            RuntimeEvent::PauseMenuInput(intent) => resolve_pause_menu_input(engine, *intent),
            _ => Ok(Vec::new()),
        }
    }
}

impl DomainEventResolver for CascadeResolveHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Dialog(_) | RuntimeEvent::Menu(_) | RuntimeEvent::Explore(_)
        )
    }

    fn resolve(&self, _engine: &mut GameEngine, event: &RuntimeEvent) -> Result<Vec<RuntimeEvent>> {
        match event {
            RuntimeEvent::Dialog(dialog_event) => match dialog_event {
                crate::game::DialogEvent::None => Ok(Vec::new()),
                crate::game::DialogEvent::Transition(transition) => {
                    Ok(vec![RuntimeEvent::ApplyDialogTransition(*transition)])
                }
                crate::game::DialogEvent::Action(action, transition) => Ok(vec![
                    RuntimeEvent::ApplyDialogAction(action.clone()),
                    RuntimeEvent::ApplyDialogTransition(*transition),
                ]),
            },
            RuntimeEvent::Menu(MenuEvent::Action(action)) => match action {
                MenuAction::NewGame => Ok(vec![RuntimeEvent::StartNewGame]),
                MenuAction::Continue => Ok(vec![RuntimeEvent::ContinueGame]),
                MenuAction::Exit => Ok(vec![RuntimeEvent::Exit(0)]),
            },
            RuntimeEvent::Explore(AppExploreEvent::Npc(npc_event)) => match npc_event {
                crate::game::NpcEvent::OpenDialog(dialog_spec) => {
                    let mut events = Vec::with_capacity(2);
                    if dialog_spec.restore {
                        events.push(RuntimeEvent::RestoreSessionStats);
                    }
                    events.push(RuntimeEvent::OpenDialogState(
                        crate::game::DialogState::new(
                            dialog_spec.npc_name.clone(),
                            dialog_spec.lines.clone(),
                        ),
                    ));
                    Ok(events)
                }
                crate::game::NpcEvent::OpenShop(shop_id) => {
                    Ok(vec![RuntimeEvent::OpenShopById(shop_id.clone())])
                }
                crate::game::NpcEvent::RestoreStats => Ok(vec![RuntimeEvent::RestoreSessionStats]),
            },
            RuntimeEvent::Explore(AppExploreEvent::UseAction(action)) => {
                Ok(vec![RuntimeEvent::CombatPlayerAction(*action)])
            }
            RuntimeEvent::Explore(AppExploreEvent::EnterPauseMenu) => {
                Ok(vec![RuntimeEvent::OpenPauseMenu])
            }
            RuntimeEvent::Explore(AppExploreEvent::EnterMenu) => {
                Ok(vec![RuntimeEvent::OpenMenuFromExplore])
            }
            _ => Ok(Vec::new()),
        }
    }
}

impl DomainEventApplier for CoreApplyHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Tick
                | RuntimeEvent::KeyDown(_)
                | RuntimeEvent::KeyUp(_)
                | RuntimeEvent::OverlayCloseRequested
                | RuntimeEvent::GameOverConfirmRequested
                | RuntimeEvent::ErrorConfirmRequested
                | RuntimeEvent::UpdateLoading
                | RuntimeEvent::UpdateMovement
                | RuntimeEvent::UpdateCombat
                | RuntimeEvent::MenuInput(_)
                | RuntimeEvent::ExploreInput(_)
                | RuntimeEvent::InventoryInput(_)
                | RuntimeEvent::DialogInput(_)
                | RuntimeEvent::ShopInput(_)
                | RuntimeEvent::PauseMenuInput(_)
                | RuntimeEvent::StartNewGame
                | RuntimeEvent::ContinueGame
                | RuntimeEvent::OpenPauseMenu
                | RuntimeEvent::OpenMenuFromExplore
                | RuntimeEvent::OpenDialogState(_)
                | RuntimeEvent::OpenShopById(_)
                | RuntimeEvent::RestoreSessionStats
                | RuntimeEvent::ApplyDialogAction(_)
                | RuntimeEvent::ApplyDialogTransition(_)
        )
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::Tick
            | RuntimeEvent::KeyDown(_)
            | RuntimeEvent::KeyUp(_)
            | RuntimeEvent::OverlayCloseRequested
            | RuntimeEvent::GameOverConfirmRequested
            | RuntimeEvent::ErrorConfirmRequested
            | RuntimeEvent::UpdateLoading
            | RuntimeEvent::UpdateMovement
            | RuntimeEvent::UpdateCombat
            | RuntimeEvent::MenuInput(_)
            | RuntimeEvent::ExploreInput(_)
            | RuntimeEvent::InventoryInput(_)
            | RuntimeEvent::DialogInput(_)
            | RuntimeEvent::ShopInput(_)
            | RuntimeEvent::PauseMenuInput(_) => {}
            RuntimeEvent::StartNewGame => crate::game::apply::apply_start_new_game(engine),
            RuntimeEvent::ContinueGame => crate::game::apply::apply_continue_game(engine),
            RuntimeEvent::OpenPauseMenu => crate::game::apply::apply_open_pause_menu(engine),
            RuntimeEvent::OpenMenuFromExplore => {
                crate::game::apply::apply_open_menu_from_explore(engine)?
            }
            RuntimeEvent::OpenDialogState(dialog_state) => {
                crate::game::apply::apply_open_dialog_state(engine, dialog_state.clone())
            }
            RuntimeEvent::OpenShopById(shop_id) => {
                crate::game::apply::apply_open_shop_by_id(engine, shop_id)
            }
            RuntimeEvent::RestoreSessionStats => {
                crate::game::apply::apply_restore_session_stats(engine)?
            }
            RuntimeEvent::ApplyDialogAction(action) => {
                crate::game::apply::apply_dialog_action(engine, action)?
            }
            RuntimeEvent::ApplyDialogTransition(transition) => {
                crate::game::apply::apply_dialog_transition(engine, *transition)
            }
            _ => {}
        }

        Ok(())
    }
}

impl DomainEventApplier for SystemApplyHandler {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Loading(_)
                | RuntimeEvent::Movement(_)
                | RuntimeEvent::Menu(_)
                | RuntimeEvent::Explore(_)
                | RuntimeEvent::Inventory(_)
                | RuntimeEvent::Dialog(_)
                | RuntimeEvent::Shop(_)
                | RuntimeEvent::PauseMenu(_)
                | RuntimeEvent::CombatPlayerAction(_)
                | RuntimeEvent::Transition(_)
                | RuntimeEvent::Exit(_)
                | RuntimeEvent::Combat(_)
        )
    }

    fn apply(&self, engine: &mut GameEngine, event: &RuntimeEvent) -> Result<()> {
        crate::game::apply::apply_runtime_event(engine, event)
    }
}

fn resolve_update_loading_event(engine: &mut GameEngine) -> Result<Vec<RuntimeEvent>> {
    let step = if let GameState::Loading(step) = engine.state() {
        *step
    } else {
        return Err(anyhow!("Invalid state: expected Loading"));
    };

    let mut data = engine.data_rc();
    let load_result = crate::game::lifecycle::load_step(&mut data, step);
    engine.replace_data(data);

    Ok(vec![RuntimeEvent::Loading(
        crate::game::lifecycle::resolve_loading(step, load_result),
    )])
}

fn resolve_update_movement_event(engine: &GameEngine) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Explore),
        "Invalid state: expected Explore"
    );
    let s = engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;

    let movement = crate::game::movement::resolve_world_tick(
        &s.movement,
        &s.player,
        &s.combat.enemies,
        engine.data(),
    );

    let mut events = Vec::with_capacity(if movement.map_changed { 2 } else { 1 });
    events.push(RuntimeEvent::Movement(AppMovementEvent::Tick(
        movement.movement_event,
        movement.tile_event,
    )));
    if movement.map_changed {
        events.push(RuntimeEvent::Transition(TransitionEvent::MapChanged));
    }
    Ok(events)
}

fn resolve_update_combat_event(engine: &GameEngine) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Explore),
        "Invalid state: expected Explore"
    );
    let s = engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;
    let Some(map) = engine.data().find_map(&s.player.current_map_id) else {
        return Ok(Vec::new());
    };

    Ok(crate::game::combat::resolve_tick(
        &s.combat,
        s.player.x,
        s.player.y,
        s.player.total_def(),
        (s.skill_cooldowns, s.mp_regen_timer),
        map,
        &engine.data().enemies,
    ))
}

fn resolve_menu_input(
    engine: &GameEngine,
    intent: crate::game::MenuIntent,
) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Menu),
        "Invalid state: expected Menu"
    );

    Ok(crate::game::menu::resolve_many(
        engine.ui().menu.selected,
        &engine.ui().menu.state.items,
        intent,
    )
    .into_iter()
    .map(RuntimeEvent::Menu)
    .collect())
}

fn resolve_explore_input(
    engine: &GameEngine,
    intent: crate::game::ExploreIntent,
) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Explore),
        "Invalid state: expected Explore"
    );
    let s = engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;

    let is_peaceful = engine
        .data()
        .find_map(&s.player.current_map_id)
        .is_some_and(|map| map.peaceful);

    let mut events = Vec::new();
    for explore_event in crate::game::explore::resolve_many(is_peaceful, intent) {
        match explore_event {
            ExploreEvent::None => {}
            ExploreEvent::MoveDirection(direction) => {
                events.push(RuntimeEvent::Explore(AppExploreEvent::MoveDirection(
                    direction,
                )));
            }
            ExploreEvent::TryNpcInteract {
                facing,
                fallback_action,
            } => {
                if let Some(npc_event) = crate::game::npc::resolve(
                    &s.player,
                    engine.data(),
                    crate::game::npc::NpcIntent::Interact { facing },
                ) {
                    events.push(RuntimeEvent::Explore(AppExploreEvent::Npc(npc_event)));
                } else if let Some(action) = fallback_action {
                    events.push(RuntimeEvent::Explore(AppExploreEvent::UseAction(action)));
                }
            }
            ExploreEvent::UseAction(action) => {
                events.push(RuntimeEvent::Explore(AppExploreEvent::UseAction(action)));
            }
            ExploreEvent::EnterPauseMenu => {
                events.push(RuntimeEvent::Explore(AppExploreEvent::EnterPauseMenu));
            }
            ExploreEvent::EnterMenu => {
                events.push(RuntimeEvent::Explore(AppExploreEvent::EnterMenu));
            }
        }
    }
    Ok(events)
}

fn resolve_inventory_input(
    engine: &GameEngine,
    intent: crate::game::InventoryIntent,
) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Inventory),
        "Invalid state: expected Inventory"
    );
    let s = engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;

    Ok(crate::game::inventory::resolve_many(
        engine.ui().inventory.selected,
        s.player.inventory.len(),
        intent,
    )
    .into_iter()
    .map(RuntimeEvent::Inventory)
    .collect())
}

fn resolve_dialog_input(
    engine: &GameEngine,
    intent: crate::game::DialogIntent,
) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Dialog),
        "Invalid state: expected Dialog"
    );
    engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;

    Ok(
        crate::game::dialog::resolve_many(engine.ui().dialog.state.as_ref(), intent)
            .into_iter()
            .map(RuntimeEvent::Dialog)
            .collect(),
    )
}

fn resolve_shop_input(engine: &GameEngine, intent: ShopIntent) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::Shop),
        "Invalid state: expected Shop"
    );

    crate::game::apply::resolve_shop_input(engine, intent)
}

fn resolve_pause_menu_input(
    engine: &GameEngine,
    intent: crate::game::PauseMenuIntent,
) -> Result<Vec<RuntimeEvent>> {
    ensure!(
        matches!(engine.state(), GameState::PauseMenu),
        "Invalid state: expected PauseMenu"
    );
    engine
        .session()
        .ok_or_else(|| anyhow!("No active session"))?;

    Ok(crate::game::menu::resolve_pause_many(
        engine.ui().pause_menu.selected,
        engine.ui().pause_menu.state.items.len(),
        intent,
    )
    .into_iter()
    .map(RuntimeEvent::PauseMenu)
    .collect())
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;

    use super::{
        CASCADE_RESOLVE_HANDLER, CORE_APPLY_HANDLER, CORE_RESOLVE_HANDLER, DomainEventApplier,
        DomainEventResolver, SYSTEM_APPLY_HANDLER,
    };
    use crate::data::DialogLine;
    use crate::engine::GameEngine;
    use crate::game::{DialogState, MenuIntent, RuntimeEvent, SessionState};

    #[test]
    fn core_resolve_menu_continue_maps_to_action_event() {
        let mut engine = GameEngine::new();
        engine.transition_to(crate::game::GameState::Menu);
        engine
            .ui_mut()
            .menu
            .set_menu(crate::game::MenuState::new(true));
        engine.ui_mut().menu.set_selected(1);

        let events = CORE_RESOLVE_HANDLER
            .resolve(&mut engine, &RuntimeEvent::MenuInput(MenuIntent::Select))
            .expect("resolve should succeed");

        assert!(matches!(
            events.as_slice(),
            [RuntimeEvent::Menu(crate::game::MenuEvent::Action(
                crate::game::MenuAction::Continue
            ))]
        ));
    }

    #[test]
    fn cascade_resolve_npc_events() {
        let mut engine = GameEngine::new();

        let event = RuntimeEvent::Explore(crate::game::AppExploreEvent::Npc(
            crate::game::NpcEvent::OpenShop(String::from("shop1")),
        ));

        let events = CASCADE_RESOLVE_HANDLER
            .resolve(&mut engine, &event)
            .expect("resolve should succeed");

        assert!(matches!(
            events.as_slice(),
            [RuntimeEvent::OpenShopById(shop_id)] if shop_id == "shop1"
        ));
    }

    #[test]
    fn system_apply_transition_map_changed() {
        let mut engine = GameEngine::new();
        let session = SessionState {
            player: crate::game::PlayerState::new(String::from("T"), "missing_map"),
            combat: crate::game::CombatState::default(),
            movement: crate::game::MovementState::default(),
            skill_cooldowns: [0; 3],
            mp_regen_timer: 0,
        };
        engine.enter_session(crate::game::GameState::Explore, session, None);

        let result = SYSTEM_APPLY_HANDLER.apply(
            &mut engine,
            &RuntimeEvent::Transition(crate::game::TransitionEvent::MapChanged),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn core_apply_dialog_transition_set_line() {
        let mut engine = GameEngine::new();
        let session = SessionState {
            player: crate::game::PlayerState::new(String::from("T"), "missing_map"),
            combat: crate::game::CombatState::default(),
            movement: crate::game::MovementState::default(),
            skill_cooldowns: [0; 3],
            mp_regen_timer: 0,
        };
        engine.enter_session(crate::game::GameState::Dialog, session, None);
        engine.ui_mut().dialog.open(DialogState::new(
            String::from("NPC"),
            vec![
                DialogLine {
                    text: String::from("a"),
                    condition: None,
                    action: None,
                },
                DialogLine {
                    text: String::from("b"),
                    condition: None,
                    action: None,
                },
            ],
        ));

        CORE_APPLY_HANDLER
            .apply(
                &mut engine,
                &RuntimeEvent::ApplyDialogTransition(crate::game::DialogTransition::SetLine(1)),
            )
            .expect("apply should succeed");

        let current_line = engine
            .ui()
            .dialog
            .state
            .as_ref()
            .map(|d| d.current_line)
            .unwrap_or(999);
        assert_eq!(current_line, 1);
    }
}
