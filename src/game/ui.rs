use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::data::{Dialog, DialogLine, Direction, Item, Shop, Skill};
use crate::game::{
    DialogIntent, ExploreIntent, InputKey, InventoryIntent, MenuIntent, PauseMenuIntent,
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
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    None
                }
                InputKey::Down => {
                    if self.selected + 1 < 2 {
                        self.selected += 1;
                    }
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
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    None
                }
                InputKey::Down => {
                    if self.selected + 1 < shop_items_len {
                        self.selected += 1;
                    }
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
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    None
                }
                InputKey::Down => {
                    if self.selected + 1 < inventory_len {
                        self.selected += 1;
                    }
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

#[derive(Debug)]
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
