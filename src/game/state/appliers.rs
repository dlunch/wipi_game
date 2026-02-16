use anyhow::{Result, anyhow, ensure};

use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{
    AppExploreEvent, AppMovementEvent, DialogActionResult, DialogState, DialogTransition, GameData,
    GameState, LoadingEvent, MenuEvent, MenuState, PauseMenuEvent, RuntimeEvent, SessionState,
    TransitionEvent, has_save_data,
};

struct LoadingApplier;
struct StartNewGameApplier;
struct ContinueGameApplier;
struct MovementApplier;
struct CombatPlayerActionApplier;
struct MenuApplier;
struct PauseMenuApplier;
struct OpenPauseMenuApplier;
struct OpenMenuFromExploreApplier;
struct ExploreApplier;
struct InventoryApplier;
struct DialogEventApplier;
struct ApplyDialogActionApplier;
struct ApplyDialogTransitionApplier;
struct ShopApplier;
struct OpenDialogStateApplier;
struct OpenShopByIdApplier;
struct RestoreSessionStatsApplier;
struct TransitionApplier;
struct ExitApplier;

static LOADING_APPLIER: LoadingApplier = LoadingApplier;
static START_NEW_GAME_APPLIER: StartNewGameApplier = StartNewGameApplier;
static CONTINUE_GAME_APPLIER: ContinueGameApplier = ContinueGameApplier;
static MOVEMENT_APPLIER: MovementApplier = MovementApplier;
static COMBAT_PLAYER_ACTION_APPLIER: CombatPlayerActionApplier = CombatPlayerActionApplier;
static MENU_APPLIER: MenuApplier = MenuApplier;
static PAUSE_MENU_APPLIER: PauseMenuApplier = PauseMenuApplier;
static OPEN_PAUSE_MENU_APPLIER: OpenPauseMenuApplier = OpenPauseMenuApplier;
static OPEN_MENU_FROM_EXPLORE_APPLIER: OpenMenuFromExploreApplier = OpenMenuFromExploreApplier;
static EXPLORE_APPLIER: ExploreApplier = ExploreApplier;
static INVENTORY_APPLIER: InventoryApplier = InventoryApplier;
static DIALOG_EVENT_APPLIER: DialogEventApplier = DialogEventApplier;
static APPLY_DIALOG_ACTION_APPLIER: ApplyDialogActionApplier = ApplyDialogActionApplier;
static APPLY_DIALOG_TRANSITION_APPLIER: ApplyDialogTransitionApplier = ApplyDialogTransitionApplier;
static SHOP_APPLIER: ShopApplier = ShopApplier;
static OPEN_DIALOG_STATE_APPLIER: OpenDialogStateApplier = OpenDialogStateApplier;
static OPEN_SHOP_BY_ID_APPLIER: OpenShopByIdApplier = OpenShopByIdApplier;
static RESTORE_SESSION_STATS_APPLIER: RestoreSessionStatsApplier = RestoreSessionStatsApplier;
static TRANSITION_APPLIER: TransitionApplier = TransitionApplier;
static EXIT_APPLIER: ExitApplier = ExitApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![
        &LOADING_APPLIER,
        &START_NEW_GAME_APPLIER,
        &CONTINUE_GAME_APPLIER,
        &MOVEMENT_APPLIER,
        &COMBAT_PLAYER_ACTION_APPLIER,
        &MENU_APPLIER,
        &PAUSE_MENU_APPLIER,
        &OPEN_PAUSE_MENU_APPLIER,
        &OPEN_MENU_FROM_EXPLORE_APPLIER,
        &EXPLORE_APPLIER,
        &INVENTORY_APPLIER,
        &DIALOG_EVENT_APPLIER,
        &APPLY_DIALOG_ACTION_APPLIER,
        &APPLY_DIALOG_TRANSITION_APPLIER,
        &SHOP_APPLIER,
        &OPEN_DIALOG_STATE_APPLIER,
        &OPEN_SHOP_BY_ID_APPLIER,
        &RESTORE_SESSION_STATS_APPLIER,
        &TRANSITION_APPLIER,
        &EXIT_APPLIER,
    ]
}

impl DomainEventApplier for LoadingApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Loading(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Loading(event) = event else {
            return Ok(());
        };
        match event {
            LoadingEvent::Advance(step) => ctx.transition_to(GameState::Loading(*step)),
            LoadingEvent::Loaded => {
                ctx.transition_to(GameState::Menu);
                ctx.ui.menu.set_menu(MenuState::new(has_save_data()));
            }
            LoadingEvent::Error(msg) => ctx.set_error(msg.clone()),
        }
        Ok(())
    }
}

impl DomainEventApplier for StartNewGameApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::StartNewGame)
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        let (state, session, intro) = crate::game::systems::lifecycle::start_new_game(ctx.data);
        enter_session(ctx, state, session, intro);
        Ok(())
    }
}

impl DomainEventApplier for ContinueGameApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::ContinueGame)
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        let (state, session, intro) = crate::game::systems::lifecycle::continue_game(ctx.data);
        enter_session(ctx, state, session, intro);
        Ok(())
    }
}

impl DomainEventApplier for MovementApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Movement(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Movement(AppMovementEvent::Tick(movement_event, tile_event)) = event
        else {
            return Ok(());
        };
        let data = alloc::rc::Rc::clone(ctx.data);
        let s = ctx
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.apply_movement_tick(&data, *movement_event, tile_event.clone());
        Ok(())
    }
}

impl DomainEventApplier for CombatPlayerActionApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::CombatPlayerAction(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::CombatPlayerAction(action) = event else {
            return Ok(());
        };
        let data = alloc::rc::Rc::clone(ctx.data);
        let s = ctx
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.apply_explore_action(&data, *action);
        Ok(())
    }
}

impl DomainEventApplier for MenuApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Menu(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Menu(event) = event else {
            return Ok(());
        };
        match event {
            MenuEvent::None => {}
            MenuEvent::SetSelected(selected) => ctx.ui_mut().menu.set_selected(*selected),
            MenuEvent::Action(_) => {}
        }
        Ok(())
    }
}

impl DomainEventApplier for PauseMenuApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::PauseMenu(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::PauseMenu(event) = event else {
            return Ok(());
        };
        match event {
            PauseMenuEvent::None => {}
            PauseMenuEvent::SetSelected(selected) => {
                ctx.ui_mut().pause_menu.set_selected(*selected)
            }
            PauseMenuEvent::OpenInventory => {
                ctx.ui_mut().inventory.reset();
                ctx.transition_to(GameState::Inventory);
            }
            PauseMenuEvent::OpenStats => ctx.transition_to(GameState::Stats),
            PauseMenuEvent::OpenQuestLog => ctx.transition_to(GameState::QuestLog),
            PauseMenuEvent::SaveAndReturnExplore => {
                {
                    let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
                    let _ = crate::game::save_game(&s.player);
                }
                ctx.ui_mut().shop.reset();
                ctx.transition_to(GameState::Explore);
            }
            PauseMenuEvent::BackToExplore => ctx.transition_to(GameState::Explore),
        }
        Ok(())
    }
}

impl DomainEventApplier for OpenPauseMenuApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::OpenPauseMenu)
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        ctx.ui_mut().pause_menu.reset();
        ctx.transition_to(GameState::PauseMenu);
        Ok(())
    }
}

impl DomainEventApplier for OpenMenuFromExploreApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::OpenMenuFromExplore)
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        {
            let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
            let _ = crate::game::save_game(&s.player);
        }
        ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
        ctx.transition_to(GameState::Menu);
        Ok(())
    }
}

impl DomainEventApplier for ExploreApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Explore(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Explore(event) = event else {
            return Ok(());
        };
        match event {
            AppExploreEvent::MoveDirection(direction) => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.on_direction_pressed(*direction);
            }
            AppExploreEvent::Npc(_)
            | AppExploreEvent::UseAction(_)
            | AppExploreEvent::EnterPauseMenu
            | AppExploreEvent::EnterMenu => {}
        }
        Ok(())
    }
}

impl DomainEventApplier for InventoryApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Inventory(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Inventory(event) = event else {
            return Ok(());
        };
        match event {
            crate::game::InventoryEvent::None => {}
            crate::game::InventoryEvent::SetSelected(selected) => {
                ctx.ui_mut().inventory.set_selected(*selected)
            }
            crate::game::InventoryEvent::UseSelected(index) => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.use_inventory_item(*index);
            }
            crate::game::InventoryEvent::CloseToExplore => ctx.transition_to(GameState::Explore),
        }
        Ok(())
    }
}

impl DomainEventApplier for DialogEventApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Dialog(_))
    }

    fn apply(&self, _ctx: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        Ok(())
    }
}

impl DomainEventApplier for ApplyDialogActionApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::ApplyDialogAction(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::ApplyDialogAction(action) = event else {
            return Ok(());
        };
        let open_shop = {
            let data = ctx.data_rc();
            let s = ctx
                .session_mut()
                .ok_or_else(|| anyhow!("No active session"))?;
            if let DialogActionResult::OpenShop(shop_id) = s.apply_dialog_action(&data, action) {
                Some(shop_id)
            } else {
                None
            }
        };
        if let Some(shop_id) = open_shop {
            let _ = ctx.open_shop_by_id(&shop_id);
        }
        Ok(())
    }
}

impl DomainEventApplier for ApplyDialogTransitionApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::ApplyDialogTransition(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::ApplyDialogTransition(transition) = event else {
            return Ok(());
        };
        match transition {
            DialogTransition::SetLine(line) => {
                if let Some(dialog_state) = ctx.ui_mut().dialog.state.as_mut() {
                    dialog_state.current_line = *line;
                }
                ctx.transition_to(GameState::Dialog);
            }
            DialogTransition::CloseToExplore => {
                ctx.ui_mut().dialog.close();
                ctx.transition_to(GameState::Explore);
            }
        }
        Ok(())
    }
}

impl DomainEventApplier for ShopApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Shop(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Shop(event) = event else {
            return Ok(());
        };
        match event {
            crate::game::ShopEvent::None => {}
            crate::game::ShopEvent::BuyItem(item) => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.buy_shop_item(item.clone());
            }
            crate::game::ShopEvent::SellSelected(index) => {
                let (sold, len_after) = {
                    let s = ctx
                        .session_mut()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let sold = s.sell_inventory_item(*index).is_some();
                    (sold, s.player.inventory.len())
                };
                if sold {
                    let current_selected = ctx.ui.shop.selected;
                    if current_selected >= len_after && current_selected > 0 {
                        ctx.ui_mut().shop.set_selected(current_selected - 1);
                    }
                }
            }
            crate::game::ShopEvent::CloseToExplore => ctx.transition_to(GameState::Explore),
        }
        Ok(())
    }
}

impl DomainEventApplier for OpenDialogStateApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::OpenDialogState(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::OpenDialogState(dialog_state) = event else {
            return Ok(());
        };
        ctx.ui_mut().dialog.open(dialog_state.clone());
        ctx.transition_to(GameState::Dialog);
        Ok(())
    }
}

impl DomainEventApplier for OpenShopByIdApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::OpenShopById(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::OpenShopById(shop_id) = event else {
            return Ok(());
        };
        let _ = ctx.open_shop_by_id(shop_id);
        Ok(())
    }
}

impl DomainEventApplier for RestoreSessionStatsApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::RestoreSessionStats)
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, _event: &RuntimeEvent) -> Result<()> {
        let s = ctx
            .session_mut()
            .ok_or_else(|| anyhow!("No active session"))?;
        s.restore_stats();
        Ok(())
    }
}

impl DomainEventApplier for TransitionApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Transition(_))
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Transition(transition) = event else {
            return Ok(());
        };
        apply_transition(ctx, *transition)
    }
}

impl DomainEventApplier for ExitApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Exit(_))
    }

    fn apply(&self, _ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        let RuntimeEvent::Exit(code) = event else {
            return Ok(());
        };
        wipi::kernel::exit(*code);
        Ok(())
    }
}

fn dialog_state_from_intro(
    data: &GameData,
    intro: Option<crate::game::systems::lifecycle::IntroDialogSpec>,
) -> Option<DialogState> {
    let spec = intro?;
    let dialog = data.find_dialog(&spec.dialog_id)?;
    Some(DialogState::from_dialog(spec.npc_name, dialog))
}

fn enter_session(
    ctx: &mut ApplyContext<'_>,
    state: GameState,
    session: SessionState,
    intro: Option<crate::game::systems::lifecycle::IntroDialogSpec>,
) {
    *ctx.session = Some(session);
    ctx.transition_to(state);

    if let Some(s) = ctx.session.as_mut() {
        s.spawn_current_map_enemies(ctx.data);
    }

    *ctx.ui = crate::game::UiState::default();
    ctx.ui.dialog.set(dialog_state_from_intro(ctx.data, intro));
}

fn apply_transition(ctx: &mut ApplyContext<'_>, event: TransitionEvent) -> Result<()> {
    match event {
        TransitionEvent::MapChanged => apply_map_changed(ctx)?,
        TransitionEvent::ToExplore => ctx.transition_to(GameState::Explore),
        TransitionEvent::ToMenuFromGameOver => {
            ctx.transition_to(GameState::Menu);
            ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
        }
        TransitionEvent::ReleaseMovementDirection(direction) => {
            apply_release_movement_direction(ctx, direction)?
        }
    }
    Ok(())
}

fn apply_map_changed(ctx: &mut ApplyContext<'_>) -> Result<()> {
    let data = ctx.data_rc();
    let s = ctx
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.spawn_current_map_enemies(&data);
    Ok(())
}

fn apply_release_movement_direction(
    ctx: &mut ApplyContext<'_>,
    direction: crate::data::Direction,
) -> Result<()> {
    ensure!(
        matches!(ctx.state, GameState::Explore),
        "Invalid state: expected Explore"
    );
    let s = ctx
        .session_mut()
        .ok_or_else(|| anyhow!("No active session"))?;
    s.on_direction_released(direction);
    Ok(())
}
