use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::data::{Dialog, DialogLine, Direction, Item, Shop, Skill};
use crate::game::selection::{step_down, step_up};
use crate::game::{ExploreCommand, GameEvent, GameState, SessionState, TransitionEvent};

pub const INVENTORY_VISIBLE_ITEMS: usize = 8;
pub const SHOP_VISIBLE_ITEMS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKey {
    Ok,
    Back,
    Up,
    Down,
    Left,
    Right,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
}

impl InputKey {
    pub fn direction(self) -> Option<Direction> {
        match self {
            InputKey::Up => Some(Direction::Up),
            InputKey::Down => Some(Direction::Down),
            InputKey::Left => Some(Direction::Left),
            InputKey::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameInput {
    KeyDown(InputKey),
    KeyUp(InputKey),
}

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
    fn resolve_input(
        &mut self,
        input: GameInput,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<UiEvent>;
}

pub enum UiEvent {
    OverlayCloseRequested,
    GameOverConfirmRequested,
    ErrorConfirmRequested,
    MovementKeyReleased(Direction),
    MenuInput(InputKey),
    ExploreInput(InputKey),
    InventoryInput(InputKey),
    DialogInput(InputKey),
    PauseMenuInput(InputKey),
    ShopBuySelected(usize),
    ShopSellSelected(usize),
    ShopClose,
}

impl UiInputEventResolver for UiState {
    fn resolve_input(
        &mut self,
        input: GameInput,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<UiEvent> {
        resolve_input(input, game_state, self, session)
    }
}

pub trait UiEventApplier {
    fn apply_ui_event(&mut self, session: Option<&SessionState>, event: UiEvent) -> Vec<GameEvent>;
}

#[derive(Debug, Clone, Copy)]
pub enum MenuEvent {
    None,
    SetSelected(usize),
    Action(MenuAction),
}

#[derive(Clone, Copy)]
pub enum PauseMenuEvent {
    None,
    SetSelected(usize),
    OpenInventory,
    OpenStats,
    OpenQuestLog,
    SaveAndReturnExplore,
    BackToExplore,
}

#[derive(Clone)]
pub enum InventoryEvent {
    None,
    SetSelected(usize),
    UseSelected(usize),
    CloseToExplore,
}

#[derive(Clone, Copy)]
pub enum DialogCommand {
    Confirm,
    Back,
}

#[derive(Debug, Clone, Copy)]
pub enum DialogTransition {
    SetLine(usize),
    CloseToExplore,
}

#[derive(Clone, Copy)]
pub enum ShopCommand {
    BuySelected(usize),
    SellSelected(usize),
    Close,
}

impl UiEventApplier for UiState {
    fn apply_ui_event(&mut self, session: Option<&SessionState>, event: UiEvent) -> Vec<GameEvent> {
        match event {
            UiEvent::OverlayCloseRequested => {
                vec![GameEvent::Transition(TransitionEvent::ToExplore)]
            }
            UiEvent::GameOverConfirmRequested => {
                vec![GameEvent::Transition(TransitionEvent::ToMenuFromGameOver)]
            }
            UiEvent::ErrorConfirmRequested => vec![GameEvent::Exit(1)],
            UiEvent::MovementKeyReleased(direction) => vec![GameEvent::Transition(
                TransitionEvent::ReleaseMovementDirection(direction),
            )],
            UiEvent::MenuInput(key) => self.resolve_menu_input(key),
            UiEvent::PauseMenuInput(key) => self.resolve_pause_menu_input(key),
            UiEvent::ExploreInput(key) => self.resolve_explore_input(key),
            UiEvent::InventoryInput(key) => self.resolve_inventory_input(session, key),
            UiEvent::DialogInput(key) => self.resolve_dialog_input(key),
            UiEvent::ShopBuySelected(selected) => {
                vec![GameEvent::ShopCommand(ShopCommand::BuySelected(selected))]
            }
            UiEvent::ShopSellSelected(selected) => {
                vec![GameEvent::ShopCommand(ShopCommand::SellSelected(selected))]
            }
            UiEvent::ShopClose => vec![GameEvent::ShopCommand(ShopCommand::Close)],
        }
    }
}

impl UiState {
    fn resolve_explore_input(&self, key: InputKey) -> Vec<GameEvent> {
        let event = match key {
            InputKey::Up => Some(ExploreCommand::Move(Direction::Up)),
            InputKey::Down => Some(ExploreCommand::Move(Direction::Down)),
            InputKey::Left => Some(ExploreCommand::Move(Direction::Left)),
            InputKey::Right => Some(ExploreCommand::Move(Direction::Right)),
            InputKey::Ok => Some(ExploreCommand::Confirm),
            InputKey::Key1 => Some(ExploreCommand::UseSlot(0)),
            InputKey::Key2 => Some(ExploreCommand::UseSlot(1)),
            InputKey::Key3 => Some(ExploreCommand::UseSlot(2)),
            InputKey::Key0 => Some(ExploreCommand::OpenPauseMenu),
            InputKey::Back => Some(ExploreCommand::OpenMenu),
            _ => None,
        };

        event.map(GameEvent::ExploreCommand).into_iter().collect()
    }

    fn resolve_inventory_input(
        &self,
        session: Option<&SessionState>,
        key: InputKey,
    ) -> Vec<GameEvent> {
        let Some(s) = session else {
            return Vec::new();
        };

        let selected = self.inventory.selected;
        let event = match key {
            InputKey::Up => {
                let next = step_up(selected);
                if next != selected {
                    InventoryEvent::SetSelected(next)
                } else {
                    InventoryEvent::None
                }
            }
            InputKey::Down => {
                let next = step_down(selected, s.leader.inventory.len());
                if next != selected {
                    InventoryEvent::SetSelected(next)
                } else {
                    InventoryEvent::None
                }
            }
            InputKey::Ok => InventoryEvent::UseSelected(selected),
            InputKey::Back => InventoryEvent::CloseToExplore,
            _ => InventoryEvent::None,
        };

        match event {
            InventoryEvent::None => Vec::new(),
            event => vec![GameEvent::Inventory(event)],
        }
    }

    fn resolve_dialog_input(&self, key: InputKey) -> Vec<GameEvent> {
        let event = match key {
            InputKey::Ok => Some(DialogCommand::Confirm),
            InputKey::Back => Some(DialogCommand::Back),
            _ => None,
        };

        event.map(GameEvent::DialogCommand).into_iter().collect()
    }

    fn resolve_menu_input(&mut self, key: InputKey) -> Vec<GameEvent> {
        let selected = self.menu.selected;
        let items = &self.menu.state.items;

        let event = match key {
            InputKey::Up => {
                let next = step_up(selected);
                if next != selected {
                    MenuEvent::SetSelected(next)
                } else {
                    MenuEvent::None
                }
            }
            InputKey::Down => {
                let next = step_down(selected, items.len());
                if next != selected {
                    MenuEvent::SetSelected(next)
                } else {
                    MenuEvent::None
                }
            }
            InputKey::Ok => {
                if let Some((_, action)) = items.get(selected).copied() {
                    MenuEvent::Action(action)
                } else {
                    MenuEvent::None
                }
            }
            _ => MenuEvent::None,
        };

        match event {
            MenuEvent::None => Vec::new(),
            MenuEvent::SetSelected(selected) => {
                self.menu.set_selected(selected);
                Vec::new()
            }
            MenuEvent::Action(action) => match action {
                MenuAction::NewGame => vec![GameEvent::StartNewGame],
                MenuAction::Continue => vec![GameEvent::ContinueGame],
                MenuAction::Exit => vec![GameEvent::Exit(0)],
            },
        }
    }

    fn resolve_pause_menu_input(&self, key: InputKey) -> Vec<GameEvent> {
        let selected = self.pause_menu.selected;
        let item_count = self.pause_menu.state.items.len();

        let event = match key {
            InputKey::Up => {
                let next = step_up(selected);
                if next != selected {
                    PauseMenuEvent::SetSelected(next)
                } else {
                    PauseMenuEvent::None
                }
            }
            InputKey::Down => {
                let next = step_down(selected, item_count);
                if next != selected {
                    PauseMenuEvent::SetSelected(next)
                } else {
                    PauseMenuEvent::None
                }
            }
            InputKey::Ok => match selected {
                0 => PauseMenuEvent::OpenInventory,
                1 => PauseMenuEvent::OpenStats,
                2 => PauseMenuEvent::OpenQuestLog,
                3 => PauseMenuEvent::SaveAndReturnExplore,
                _ => PauseMenuEvent::None,
            },
            InputKey::Back | InputKey::Key0 => PauseMenuEvent::BackToExplore,
            _ => PauseMenuEvent::None,
        };

        match event {
            PauseMenuEvent::None => Vec::new(),
            event => vec![GameEvent::PauseMenu(event)],
        }
    }
}

fn resolve_input(
    input: GameInput,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<UiEvent> {
    match input {
        GameInput::KeyDown(key) => resolve_keydown(key, game_state, ui, session),
        GameInput::KeyUp(key) => resolve_keyup(key, game_state, session),
    }
}

fn resolve_keydown(
    key: InputKey,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<UiEvent> {
    match game_state {
        GameState::Loading(_) => Vec::new(),
        GameState::Menu => ui.menu.event_for_key(key).into_iter().collect(),
        GameState::Explore => ui.explore.events_for_key(key),
        GameState::Inventory => ui.inventory.event_for_key(key).into_iter().collect(),
        GameState::Stats | GameState::QuestLog => {
            if matches!(key, InputKey::Back | InputKey::Ok) {
                vec![UiEvent::OverlayCloseRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Dialog => ui.dialog.event_for_key(key).into_iter().collect(),
        GameState::Shop => {
            let inventory_len = session
                .map(|session_state| session_state.leader.inventory.len())
                .unwrap_or(0);
            ui.shop
                .event_for_key(key, inventory_len)
                .into_iter()
                .collect()
        }
        GameState::PauseMenu => ui.pause_menu.event_for_key(key).into_iter().collect(),
        GameState::GameOver => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::GameOverConfirmRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Error(_) => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::ErrorConfirmRequested]
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
) -> Vec<UiEvent> {
    if matches!(game_state, GameState::Explore)
        && session.is_some()
        && let Some(direction) = key.direction()
    {
        vec![UiEvent::MovementKeyReleased(direction)]
    } else {
        Vec::new()
    }
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
    pub fn events_for_key(&self, key: InputKey) -> Vec<UiEvent> {
        if matches!(
            key,
            InputKey::Up
                | InputKey::Down
                | InputKey::Left
                | InputKey::Right
                | InputKey::Ok
                | InputKey::Key0
                | InputKey::Key1
                | InputKey::Key2
                | InputKey::Key3
                | InputKey::Back
        ) {
            vec![UiEvent::ExploreInput(key)]
        } else {
            Vec::new()
        }
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
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(key, InputKey::Up | InputKey::Down | InputKey::Ok) {
            Some(UiEvent::MenuInput(key))
        } else {
            None
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
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back | InputKey::Key0
        ) {
            Some(UiEvent::PauseMenuInput(key))
        } else {
            None
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
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back
        ) {
            Some(UiEvent::InventoryInput(key))
        } else {
            None
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
    pub fn event_for_key(&mut self, key: InputKey, inventory_len: usize) -> Option<UiEvent> {
        self.handle_key(key, inventory_len)
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

    pub fn handle_key(&mut self, key: InputKey, inventory_len: usize) -> Option<UiEvent> {
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
                InputKey::Back => Some(UiEvent::ShopClose),
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
                InputKey::Ok => Some(UiEvent::ShopBuySelected(self.selected)),
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
                InputKey::Ok => Some(UiEvent::ShopSellSelected(self.selected)),
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
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(key, InputKey::Ok | InputKey::Back) {
            Some(UiEvent::DialogInput(key))
        } else {
            None
        }
    }

    pub fn open(&mut self, state: DialogState) {
        self.state = Some(state);
    }

    pub fn close(&mut self) {
        self.state = None;
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

#[derive(Debug, Clone)]
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

    #[test]
    fn explore_ui_maps_ok_to_npc_interact_with_fallback_action() {
        let ui = ExploreUiState::default();
        let events = ui.events_for_key(InputKey::Ok);
        assert!(matches!(
            events.as_slice(),
            [UiEvent::ExploreInput(InputKey::Ok)]
        ));
    }

    #[test]
    fn menu_ui_maps_up_down_ok_keys() {
        let ui = MenuUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Up),
            Some(UiEvent::MenuInput(InputKey::Up))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Down),
            Some(UiEvent::MenuInput(InputKey::Down))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::MenuInput(InputKey::Ok))
        ));
    }

    #[test]
    fn pause_menu_ui_maps_back_and_zero_to_back_intent() {
        let ui = PauseMenuUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::PauseMenuInput(InputKey::Back))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Key0),
            Some(UiEvent::PauseMenuInput(InputKey::Key0))
        ));
    }

    #[test]
    fn inventory_ui_maps_expected_keys() {
        let ui = InventoryUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Up),
            Some(UiEvent::InventoryInput(InputKey::Up))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Down),
            Some(UiEvent::InventoryInput(InputKey::Down))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::InventoryInput(InputKey::Ok))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::InventoryInput(InputKey::Back))
        ));
    }

    #[test]
    fn dialog_ui_maps_expected_keys() {
        let ui = DialogUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::DialogInput(InputKey::Ok))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::DialogInput(InputKey::Back))
        ));
    }
}
