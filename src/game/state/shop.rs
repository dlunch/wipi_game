use alloc::vec::Vec;

use crate::data::{Item, Shop};

#[derive(Debug)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    Select,
}

impl ShopState {
    pub fn new(shop: Shop, items: Vec<Item>) -> Self {
        Self { shop, items }
    }
}
