use anyhow::{Result, anyhow};

use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{
    DialogState, GameEvent, GameState, MenuState, PlayerAction, PlayerEvent, has_save_data,
};

struct UiGameEventApplier;

static UI_GAME_EVENT_APPLIER: UiGameEventApplier = UiGameEventApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&UI_GAME_EVENT_APPLIER]
}

impl DomainEventApplier for UiGameEventApplier {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Menu(_)
                | GameEvent::PauseMenu(_)
                | GameEvent::StartNewGame
                | GameEvent::ContinueGame
                | GameEvent::OpenPauseMenu
                | GameEvent::OpenMenuFromExplore
                | GameEvent::Explore(_)
                | GameEvent::Inventory(_)
                | GameEvent::Dialog(_)
                | GameEvent::ApplyDialogTransition(_)
                | GameEvent::Shop(_)
                | GameEvent::OpenDialogState(_)
                | GameEvent::OpenShopById(_)
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()> {
        match event {
            GameEvent::StartNewGame | GameEvent::ContinueGame => {
                *ctx.ui = crate::game::UiState::default();
                if matches!(ctx.state, GameState::Dialog)
                    && let Some(dialog_state) = intro_dialog_state(ctx.data)
                {
                    ctx.ui.dialog.set(Some(dialog_state));
                }
            }
            GameEvent::Menu(event) => match event {
                crate::game::MenuEvent::None => {}
                crate::game::MenuEvent::SetSelected(selected) => {
                    ctx.ui_mut().menu.set_selected(*selected)
                }
                crate::game::MenuEvent::Action(_) => {}
            },
            GameEvent::PauseMenu(event) => match event {
                crate::game::PauseMenuEvent::None => {}
                crate::game::PauseMenuEvent::SetSelected(selected) => {
                    ctx.ui_mut().pause_menu.set_selected(*selected)
                }
                crate::game::PauseMenuEvent::OpenInventory => {
                    ctx.ui_mut().inventory.reset();
                    ctx.transition_to(GameState::Inventory);
                }
                crate::game::PauseMenuEvent::OpenStats => ctx.transition_to(GameState::Stats),
                crate::game::PauseMenuEvent::OpenQuestLog => ctx.transition_to(GameState::QuestLog),
                crate::game::PauseMenuEvent::SaveAndReturnExplore => {
                    {
                        let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
                        let _ = crate::game::save_game(&s.player);
                    }
                    ctx.ui_mut().shop.reset();
                    ctx.transition_to(GameState::Explore);
                }
                crate::game::PauseMenuEvent::BackToExplore => ctx.transition_to(GameState::Explore),
            },
            GameEvent::OpenPauseMenu => {
                ctx.ui_mut().pause_menu.reset();
                ctx.transition_to(GameState::PauseMenu);
            }
            GameEvent::OpenMenuFromExplore => {
                {
                    let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
                    let _ = crate::game::save_game(&s.player);
                }
                ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
                ctx.transition_to(GameState::Menu);
            }
            GameEvent::Explore(crate::game::AppExploreEvent::MoveDirection(direction)) => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.movement.on_direction_pressed(*direction);
            }
            GameEvent::Explore(_) => {}
            GameEvent::Inventory(event) => match event {
                crate::game::InventoryEvent::None => {}
                crate::game::InventoryEvent::SetSelected(selected) => {
                    ctx.ui_mut().inventory.set_selected(*selected)
                }
                crate::game::InventoryEvent::UseSelected(index) => {
                    let s = ctx
                        .session_mut()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let _ = s.player.apply(PlayerAction::UseItem { index: *index });
                }
                crate::game::InventoryEvent::CloseToExplore => {
                    ctx.transition_to(GameState::Explore)
                }
            },
            GameEvent::Dialog(_) => {}
            GameEvent::ApplyDialogTransition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = ctx.ui_mut().dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                    ctx.transition_to(GameState::Dialog);
                }
                crate::game::DialogTransition::CloseToExplore => {
                    ctx.ui_mut().dialog.close();
                    ctx.transition_to(GameState::Explore);
                }
            },
            GameEvent::Shop(event) => match event {
                crate::game::ShopEvent::BuyItem(item) => {
                    let s = ctx
                        .session_mut()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let _ = s.player.apply(PlayerAction::AddGold(-item.price));
                    let _ = s.player.apply(PlayerAction::AddItem(item.clone()));
                }
                crate::game::ShopEvent::SellSelected(index) => {
                    let (sold, len_after) = {
                        let s = ctx
                            .session_mut()
                            .ok_or_else(|| anyhow!("No active session"))?;
                        let sold = if let PlayerEvent::ItemRemoved(Some(item)) =
                            s.player.apply(PlayerAction::RemoveItemAt(*index))
                        {
                            let _ = s.player.apply(PlayerAction::AddGold(item.price / 2));
                            true
                        } else {
                            false
                        };
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
            },
            GameEvent::OpenDialogState(dialog_state) => {
                ctx.ui_mut().dialog.open(dialog_state.clone());
                ctx.transition_to(GameState::Dialog);
            }
            GameEvent::OpenShopById(shop_id) => {
                let _ = ctx.open_shop_by_id(shop_id);
            }
            _ => {}
        }
        Ok(())
    }
}

fn intro_dialog_state(data: &crate::game::GameData) -> Option<DialogState> {
    let (dialog_id, npc_name) = data.newgame.intro_dialog.as_ref()?;
    let dialog = data.find_dialog(dialog_id)?;
    Some(DialogState::from_dialog(npc_name.clone(), dialog))
}
