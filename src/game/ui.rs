use crate::game::ShopMode;

pub const INVENTORY_VISIBLE_ITEMS: usize = 8;
pub const SHOP_VISIBLE_ITEMS: usize = 8;

#[derive(Debug, Default)]
pub struct UiState {
    pub menu: MenuUiState,
    pub pause_menu: PauseMenuUiState,
    pub inventory: InventoryUiState,
    pub shop: ShopUiState,
}

#[derive(Debug, Default)]
pub struct MenuUiState {
    pub selected: usize,
}

#[derive(Debug, Default)]
pub struct PauseMenuUiState {
    pub selected: usize,
}

#[derive(Debug, Default)]
pub struct InventoryUiState {
    pub selected: usize,
}

#[derive(Debug)]
pub struct ShopUiState {
    pub mode: ShopMode,
    pub selected: usize,
}

impl Default for ShopUiState {
    fn default() -> Self {
        Self {
            mode: ShopMode::Select,
            selected: 0,
        }
    }
}
