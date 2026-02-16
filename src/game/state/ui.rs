use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{Dialog, DialogLine, Direction, Item, Shop, Skill};
use crate::game::selection::{step_down, step_up};
use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{
    DialogIntent, ExploreIntent, GameInput, GameState, InputKey, InventoryIntent, MenuIntent,
    PauseMenuIntent, RuntimeEvent, SessionState, ShopIntent, TransitionEvent, has_save_data,
};

pub const INVENTORY_VISIBLE_ITEMS: usize = 8;
pub const SHOP_VISIBLE_ITEMS: usize = 8;

#[derive(Debug, Default)]
pub struct UiState {
    pub explore: ExploreUiState,
    pub menu: MenuUiState,
    pub pause_menu: PauseMenuUiState,
    pub inventory: InventoryUiState,
    pub shop: ShopUiState,
    pub dialog: DialogUiState,
}

pub trait UiInputEventResolver {
    fn resolve_input_event(
        &mut self,
        event: &RuntimeEvent,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<RuntimeEvent>;
}

impl UiInputEventResolver for UiState {
    fn resolve_input_event(
        &mut self,
        event: &RuntimeEvent,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<RuntimeEvent> {
        match event {
            RuntimeEvent::Tick => resolve_input(GameInput::Tick, game_state, self, session),
            RuntimeEvent::KeyDown(key) => {
                resolve_input(GameInput::KeyDown(*key), game_state, self, session)
            }
            RuntimeEvent::KeyUp(key) => {
                resolve_input(GameInput::KeyUp(*key), game_state, self, session)
            }
            _ => Vec::new(),
        }
    }
}

fn resolve_input(
    input: GameInput,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<RuntimeEvent> {
    match input {
        GameInput::Tick => resolve_tick(game_state),
        GameInput::KeyDown(key) => resolve_keydown(key, game_state, ui, session),
        GameInput::KeyUp(key) => resolve_keyup(key, game_state, session),
    }
}

fn resolve_tick(game_state: &GameState) -> Vec<RuntimeEvent> {
    match game_state {
        GameState::Loading(_) => vec![RuntimeEvent::UpdateLoading],
        GameState::Explore => vec![RuntimeEvent::UpdateMovement, RuntimeEvent::UpdateCombat],
        _ => Vec::new(),
    }
}

fn resolve_keydown(
    key: InputKey,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<RuntimeEvent> {
    match game_state {
        GameState::Loading(_) => Vec::new(),
        GameState::Menu => ui.menu.event_for_key(key).into_iter().collect(),
        GameState::Explore => {
            let facing = session
                .map(|session_state| session_state.player.facing)
                .unwrap_or(Direction::Down);
            ui.explore.events_for_key(key, facing)
        }
        GameState::Inventory => ui.inventory.event_for_key(key).into_iter().collect(),
        GameState::Stats | GameState::QuestLog => {
            if matches!(key, InputKey::Back | InputKey::Ok) {
                vec![RuntimeEvent::OverlayCloseRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Dialog => ui.dialog.event_for_key(key).into_iter().collect(),
        GameState::Shop => {
            let inventory_len = session
                .map(|session_state| session_state.player.inventory.len())
                .unwrap_or(0);
            ui.shop
                .event_for_key(key, inventory_len)
                .into_iter()
                .collect()
        }
        GameState::PauseMenu => ui.pause_menu.event_for_key(key).into_iter().collect(),
        GameState::GameOver => {
            if matches!(key, InputKey::Ok) {
                vec![RuntimeEvent::GameOverConfirmRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Error(_) => {
            if matches!(key, InputKey::Ok) {
                vec![RuntimeEvent::ErrorConfirmRequested]
            } else {
                Vec::new()
            }
        }
    }
}

fn resolve_keyup(
    key: InputKey,
    game_state: &GameState,
    session: Option<&SessionState>,
) -> Vec<RuntimeEvent> {
    if matches!(game_state, GameState::Explore)
        && session.is_some()
        && let Some(direction) = key.direction()
    {
        vec![RuntimeEvent::Transition(
            TransitionEvent::ReleaseMovementDirection(direction),
        )]
    } else {
        Vec::new()
    }
}

struct UiDomainApplier;

static UI_DOMAIN_APPLIER: UiDomainApplier = UiDomainApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&UI_DOMAIN_APPLIER]
}

impl DomainEventApplier for UiDomainApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Menu(_)
                | RuntimeEvent::PauseMenu(_)
                | RuntimeEvent::StartNewGame
                | RuntimeEvent::ContinueGame
                | RuntimeEvent::OpenPauseMenu
                | RuntimeEvent::OpenMenuFromExplore
                | RuntimeEvent::Explore(_)
                | RuntimeEvent::Inventory(_)
                | RuntimeEvent::Dialog(_)
                | RuntimeEvent::ApplyDialogTransition(_)
                | RuntimeEvent::Shop(_)
                | RuntimeEvent::OpenDialogState(_)
                | RuntimeEvent::OpenShopById(_)
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::StartNewGame | RuntimeEvent::ContinueGame => {
                *ctx.ui = UiState::default();
                if matches!(ctx.state, GameState::Dialog)
                    && let Some(dialog_state) = intro_dialog_state(ctx.data)
                {
                    ctx.ui.dialog.set(Some(dialog_state));
                }
            }
            RuntimeEvent::Menu(event) => match event {
                crate::game::MenuEvent::None => {}
                crate::game::MenuEvent::SetSelected(selected) => {
                    ctx.ui_mut().menu.set_selected(*selected)
                }
                crate::game::MenuEvent::Action(_) => {}
            },
            RuntimeEvent::PauseMenu(event) => match event {
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
            RuntimeEvent::OpenPauseMenu => {
                ctx.ui_mut().pause_menu.reset();
                ctx.transition_to(GameState::PauseMenu);
            }
            RuntimeEvent::OpenMenuFromExplore => {
                {
                    let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
                    let _ = crate::game::save_game(&s.player);
                }
                ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
                ctx.transition_to(GameState::Menu);
            }
            RuntimeEvent::Explore(crate::game::AppExploreEvent::MoveDirection(direction)) => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.on_direction_pressed(*direction);
            }
            RuntimeEvent::Explore(_) => {}
            RuntimeEvent::Inventory(event) => match event {
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
                crate::game::InventoryEvent::CloseToExplore => {
                    ctx.transition_to(GameState::Explore)
                }
            },
            RuntimeEvent::Dialog(_) => {}
            RuntimeEvent::ApplyDialogTransition(transition) => match transition {
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
            RuntimeEvent::Shop(event) => match event {
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
            },
            RuntimeEvent::OpenDialogState(dialog_state) => {
                ctx.ui_mut().dialog.open(dialog_state.clone());
                ctx.transition_to(GameState::Dialog);
            }
            RuntimeEvent::OpenShopById(shop_id) => {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExploreAction {
    BasicAttack,
    Fireball,
    Heal,
    SpinAttack,
}

impl ExploreAction {
    pub fn skill(self) -> Option<(usize, &'static Skill)> {
        match self {
            ExploreAction::BasicAttack => None,
            ExploreAction::Fireball => Some((0, &Skill::FIREBALL)),
            ExploreAction::Heal => Some((1, &Skill::HEAL)),
            ExploreAction::SpinAttack => Some((2, &Skill::SPIN_ATTACK)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExploreAction::BasicAttack => "Attack",
            ExploreAction::Fireball => Skill::FIREBALL.name,
            ExploreAction::Heal => Skill::HEAL.name,
            ExploreAction::SpinAttack => Skill::SPIN_ATTACK.name,
        }
    }
}

#[derive(Debug)]
pub struct ExploreUiState {
    pub ok_action: ExploreAction,
    pub key_actions: [Option<ExploreAction>; 3],
}

impl Default for ExploreUiState {
    fn default() -> Self {
        Self {
            ok_action: ExploreAction::BasicAttack,
            key_actions: [
                Some(ExploreAction::Fireball),
                Some(ExploreAction::Heal),
                Some(ExploreAction::SpinAttack),
            ],
        }
    }
}

impl ExploreUiState {
    pub fn events_for_key(&self, key: InputKey, facing: Direction) -> Vec<RuntimeEvent> {
        self.intents_for_key(key, facing)
            .into_iter()
            .map(RuntimeEvent::ExploreInput)
            .collect()
    }

    pub fn intents_for_key(&self, key: InputKey, facing: Direction) -> Vec<ExploreIntent> {
        let mut intents = Vec::new();
        match key {
            InputKey::Up => intents.push(ExploreIntent::MoveDirection(Direction::Up)),
            InputKey::Down => intents.push(ExploreIntent::MoveDirection(Direction::Down)),
            InputKey::Left => intents.push(ExploreIntent::MoveDirection(Direction::Left)),
            InputKey::Right => intents.push(ExploreIntent::MoveDirection(Direction::Right)),
            InputKey::Ok => {
                intents.push(ExploreIntent::TryNpcInteract {
                    facing,
                    fallback_action: Some(self.ok_action),
                });
            }
            InputKey::Key1 => {
                if let Some(action) = self.key_actions[0] {
                    intents.push(ExploreIntent::UseAction(action));
                }
            }
            InputKey::Key2 => {
                if let Some(action) = self.key_actions[1] {
                    intents.push(ExploreIntent::UseAction(action));
                }
            }
            InputKey::Key3 => {
                if let Some(action) = self.key_actions[2] {
                    intents.push(ExploreIntent::UseAction(action));
                }
            }
            InputKey::Key0 => intents.push(ExploreIntent::Pause),
            InputKey::Back => intents.push(ExploreIntent::BackToMenu),
            _ => {}
        }
        intents
    }
}

#[derive(Debug)]
pub struct MenuUiState {
    pub state: MenuState,
    pub selected: usize,
}

impl Default for MenuUiState {
    fn default() -> Self {
        Self {
            state: MenuState::new(false),
            selected: 0,
        }
    }
}

impl MenuUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<RuntimeEvent> {
        self.intent_for_key(key).map(RuntimeEvent::MenuInput)
    }

    pub fn intent_for_key(&self, key: InputKey) -> Option<MenuIntent> {
        match key {
            InputKey::Up => Some(MenuIntent::MoveUp),
            InputKey::Down => Some(MenuIntent::MoveDown),
            InputKey::Ok => Some(MenuIntent::Select),
            _ => None,
        }
    }

    pub fn set_menu(&mut self, state: MenuState) {
        self.state = state;
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug)]
pub struct PauseMenuUiState {
    pub state: PauseMenuState,
    pub selected: usize,
}

impl Default for PauseMenuUiState {
    fn default() -> Self {
        Self {
            state: PauseMenuState::new(),
            selected: 0,
        }
    }
}

impl PauseMenuUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<RuntimeEvent> {
        self.intent_for_key(key).map(RuntimeEvent::PauseMenuInput)
    }

    pub fn intent_for_key(&self, key: InputKey) -> Option<PauseMenuIntent> {
        match key {
            InputKey::Up => Some(PauseMenuIntent::MoveUp),
            InputKey::Down => Some(PauseMenuIntent::MoveDown),
            InputKey::Ok => Some(PauseMenuIntent::Select),
            InputKey::Back | InputKey::Key0 => Some(PauseMenuIntent::Back),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug, Default)]
pub struct InventoryUiState {
    pub selected: usize,
}

impl InventoryUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<RuntimeEvent> {
        self.intent_for_key(key).map(RuntimeEvent::InventoryInput)
    }

    pub fn intent_for_key(&self, key: InputKey) -> Option<InventoryIntent> {
        match key {
            InputKey::Up => Some(InventoryIntent::MoveUp),
            InputKey::Down => Some(InventoryIntent::MoveDown),
            InputKey::Ok => Some(InventoryIntent::UseSelected),
            InputKey::Back => Some(InventoryIntent::Back),
            _ => None,
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug)]
pub struct ShopUiState {
    pub state: Option<ShopState>,
    pub mode: ShopMode,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum ShopUiIntent {
    BuySelected(usize),
    SellSelected(usize),
    Close,
}

impl Default for ShopUiState {
    fn default() -> Self {
        Self {
            state: None,
            mode: ShopMode::Select,
            selected: 0,
        }
    }
}

impl ShopUiState {
    pub fn event_for_key(&mut self, key: InputKey, inventory_len: usize) -> Option<RuntimeEvent> {
        self.handle_key(key, inventory_len)
            .map(|ui_intent| match ui_intent {
                ShopUiIntent::BuySelected(selected) => {
                    RuntimeEvent::ShopInput(ShopIntent::BuySelected(selected))
                }
                ShopUiIntent::SellSelected(selected) => {
                    RuntimeEvent::ShopInput(ShopIntent::SellSelected(selected))
                }
                ShopUiIntent::Close => RuntimeEvent::ShopInput(ShopIntent::Close),
            })
    }

    pub fn open(&mut self, state: ShopState) {
        self.state = Some(state);
        self.mode = ShopMode::Select;
        self.selected = 0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }

    pub fn handle_key(&mut self, key: InputKey, inventory_len: usize) -> Option<ShopUiIntent> {
        let shop_items_len = self
            .state
            .as_ref()
            .map(|state| state.items.len())
            .unwrap_or(0);

        match self.mode {
            ShopMode::Select => match key {
                InputKey::Up => {
                    self.selected = step_up(self.selected);
                    None
                }
                InputKey::Down => {
                    self.selected = step_down(self.selected, 2);
                    None
                }
                InputKey::Ok => {
                    if self.selected == 0 {
                        self.mode = ShopMode::Buy;
                    } else {
                        self.mode = ShopMode::Sell;
                    }
                    self.selected = 0;
                    None
                }
                InputKey::Back => Some(ShopUiIntent::Close),
                _ => None,
            },
            ShopMode::Buy => match key {
                InputKey::Up => {
                    self.selected = step_up(self.selected);
                    None
                }
                InputKey::Down => {
                    self.selected = step_down(self.selected, shop_items_len);
                    None
                }
                InputKey::Ok => Some(ShopUiIntent::BuySelected(self.selected)),
                InputKey::Back => {
                    self.mode = ShopMode::Select;
                    self.selected = 0;
                    None
                }
                _ => None,
            },
            ShopMode::Sell => match key {
                InputKey::Up => {
                    self.selected = step_up(self.selected);
                    None
                }
                InputKey::Down => {
                    self.selected = step_down(self.selected, inventory_len);
                    None
                }
                InputKey::Ok => Some(ShopUiIntent::SellSelected(self.selected)),
                InputKey::Back => {
                    self.mode = ShopMode::Select;
                    self.selected = 0;
                    None
                }
                _ => None,
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct DialogUiState {
    pub state: Option<DialogState>,
}

impl DialogUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<RuntimeEvent> {
        self.intent_for_key(key).map(RuntimeEvent::DialogInput)
    }

    pub fn intent_for_key(&self, key: InputKey) -> Option<DialogIntent> {
        match key {
            InputKey::Ok => Some(DialogIntent::Confirm),
            InputKey::Back => Some(DialogIntent::Back),
            _ => None,
        }
    }

    pub fn open(&mut self, state: DialogState) {
        self.state = Some(state);
    }

    pub fn close(&mut self) {
        self.state = None;
    }

    pub fn set(&mut self, state: Option<DialogState>) {
        self.state = state;
    }
}

#[derive(Debug)]
pub struct MenuState {
    pub title: &'static str,
    pub items: Vec<(&'static str, MenuAction)>,
}

impl MenuState {
    pub fn new(has_save: bool) -> Self {
        let items = if has_save {
            vec![
                ("NEW GAME", MenuAction::NewGame),
                ("CONTINUE", MenuAction::Continue),
                ("EXIT", MenuAction::Exit),
            ]
        } else {
            vec![
                ("NEW GAME", MenuAction::NewGame),
                ("EXIT", MenuAction::Exit),
            ]
        };

        Self {
            title: "LOST KINGDOM",
            items,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

#[derive(Debug)]
pub struct PauseMenuState {
    pub items: Vec<&'static str>,
}

impl PauseMenuState {
    pub fn new() -> Self {
        Self {
            items: vec!["Inventory", "Stats", "Quests", "Save"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DialogState {
    pub npc_name: String,
    pub lines: Vec<DialogLine>,
    pub current_line: usize,
}

impl DialogState {
    pub fn from_dialog(npc_name: String, dialog: &Dialog) -> Self {
        Self::new(npc_name, dialog.lines.clone())
    }

    pub fn new(npc_name: String, lines: Vec<DialogLine>) -> Self {
        Self {
            npc_name,
            lines,
            current_line: 0,
        }
    }
}

#[derive(Debug)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
}

impl ShopState {
    pub fn new(shop: Shop, items: Vec<Item>) -> Self {
        Self { shop, items }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    Select,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Direction;

    #[test]
    fn explore_ui_maps_ok_to_npc_interact_with_fallback_action() {
        let ui = ExploreUiState::default();
        let intents = ui.intents_for_key(InputKey::Ok, Direction::Left);

        assert!(matches!(
            intents.as_slice(),
            [ExploreIntent::TryNpcInteract {
                facing: Direction::Left,
                fallback_action: Some(ExploreAction::BasicAttack)
            }]
        ));
    }

    #[test]
    fn menu_ui_maps_up_down_ok_keys() {
        let ui = MenuUiState::default();

        assert!(matches!(
            ui.intent_for_key(InputKey::Up),
            Some(MenuIntent::MoveUp)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Down),
            Some(MenuIntent::MoveDown)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Ok),
            Some(MenuIntent::Select)
        ));
    }

    #[test]
    fn pause_menu_ui_maps_back_and_zero_to_back_intent() {
        let ui = PauseMenuUiState::default();

        assert!(matches!(
            ui.intent_for_key(InputKey::Back),
            Some(PauseMenuIntent::Back)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Key0),
            Some(PauseMenuIntent::Back)
        ));
    }

    #[test]
    fn inventory_ui_maps_expected_keys() {
        let ui = InventoryUiState::default();

        assert!(matches!(
            ui.intent_for_key(InputKey::Up),
            Some(InventoryIntent::MoveUp)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Down),
            Some(InventoryIntent::MoveDown)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Ok),
            Some(InventoryIntent::UseSelected)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Back),
            Some(InventoryIntent::Back)
        ));
    }

    #[test]
    fn dialog_ui_maps_expected_keys() {
        let ui = DialogUiState::default();

        assert!(matches!(
            ui.intent_for_key(InputKey::Ok),
            Some(DialogIntent::Confirm)
        ));
        assert!(matches!(
            ui.intent_for_key(InputKey::Back),
            Some(DialogIntent::Back)
        ));
    }
}
