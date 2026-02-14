use alloc::vec::Vec;

use crate::data::{Item, Shop};

#[derive(Debug)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
    pub selected: usize,
    pub scroll: usize,
    pub mode: ShopMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    Select,
}

impl ShopState {
    pub fn new(shop: Shop, items: Vec<Item>) -> Self {
        Self {
            shop,
            items,
            selected: 0,
            scroll: 0,
            mode: ShopMode::Select,
        }
    }
}
