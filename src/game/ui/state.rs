use alloc::{string::String, vec, vec::Vec};

use crate::data::{DialogLine, Direction, Skill};

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
    pub quest_log: QuestLogUiState,
    pub shop: ShopUiState,
    pub dialog: DialogUiState,
}

impl UiState {
    pub fn reset(&mut self) {
        self.explore = ExploreUiState::default();
        self.menu.state = MenuState::new(false);
        self.menu.selected = 0;
        self.pause_menu.state = PauseMenuState::new();
        self.pause_menu.selected = 0;
        self.inventory.selected = 0;
        self.quest_log.selected = 0;
        self.quest_log.tracked_quest_id = None;
        self.shop.shop_id = None;
        self.shop.buy_item_ids.clear();
        self.shop.sell_item_ids.clear();
        self.shop.mode = ShopMode::Select;
        self.shop.selected = 0;
        self.dialog.state = None;
    }
}

pub enum UiEvent {
    OverlayCloseRequested,
    ReviveRequested,
    ErrorConfirmRequested,
    MovementKeyReleased(Direction),
    MenuInput(InputKey),
    ExploreInput(InputKey),
    InventoryInput(InputKey),
    QuestLogInput(InputKey),
    DialogInput(InputKey),
    PauseMenuInput(InputKey),
    ShopInput(InputKey),
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

#[derive(Debug, Default)]
pub struct InventoryUiState {
    pub selected: usize,
}

#[derive(Debug, Default)]
pub struct QuestLogUiState {
    pub selected: usize,
    pub tracked_quest_id: Option<u32>,
}

#[derive(Debug)]
pub struct ShopUiState {
    pub shop_id: Option<u32>,
    pub buy_item_ids: Vec<u32>,
    pub sell_item_ids: Vec<u32>,
    pub mode: ShopMode,
    pub selected: usize,
}

impl Default for ShopUiState {
    fn default() -> Self {
        Self {
            shop_id: None,
            buy_item_ids: Vec::new(),
            sell_item_ids: Vec::new(),
            mode: ShopMode::Select,
            selected: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct DialogUiState {
    pub state: Option<DialogState>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn new(npc_name: String, lines: Vec<DialogLine>) -> Self {
        Self {
            npc_name,
            lines,
            current_line: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    ConfirmBuy,
    ConfirmSell,
    Select,
}
