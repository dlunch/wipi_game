use alloc::vec::Vec;
use core::str;

use wipi::resource::Resource;
use wipi::WIPICError;

use crate::data::{
    parse_dialogs, parse_enemies, parse_items, parse_maps, parse_npcs, parse_quests, parse_shops,
    Dialog, Enemy, Item, Map, Npc, Quest, Shop,
};

pub enum LoadError {
    Resource,
    Utf8,
}

impl From<WIPICError> for LoadError {
    fn from(_: WIPICError) -> Self {
        LoadError::Resource
    }
}

impl From<core::str::Utf8Error> for LoadError {
    fn from(_: core::str::Utf8Error) -> Self {
        LoadError::Utf8
    }
}

#[derive(Default)]
pub struct GameData {
    pub items: Vec<Item>,
    pub enemies: Vec<Enemy>,
    pub maps: Vec<Map>,
    pub npcs: Vec<Npc>,
    pub dialogs: Vec<Dialog>,
    pub quests: Vec<Quest>,
    pub shops: Vec<Shop>,
}

impl GameData {
    pub const LOAD_STEPS: usize = 7;
    pub const LOAD_LABELS: [&str; 7] = [
        "Items", "Enemies", "Maps", "NPCs", "Dialogs", "Quests", "Shops",
    ];

    pub fn load_step(&mut self, step: usize) -> bool {
        match step {
            0 => self.items = Self::load_items().unwrap_or_default(),
            1 => self.enemies = Self::load_enemies().unwrap_or_default(),
            2 => self.maps = Self::load_maps().unwrap_or_default(),
            3 => self.npcs = Self::load_npcs().unwrap_or_default(),
            4 => self.dialogs = Self::load_dialogs().unwrap_or_default(),
            5 => self.quests = Self::load_quests().unwrap_or_default(),
            6 => self.shops = Self::load_shops().unwrap_or_default(),
            _ => return true,
        }
        false
    }

    fn load_resource<T>(path: &str, parser: fn(&str) -> T) -> Result<T, LoadError> {
        let resource = Resource::new(path)?;
        let text = str::from_utf8(resource.read())?;
        Ok(parser(text))
    }

    fn load_items() -> Result<Vec<Item>, LoadError> {
        Self::load_resource("data/items.dat", parse_items)
    }

    fn load_enemies() -> Result<Vec<Enemy>, LoadError> {
        Self::load_resource("data/enemies.dat", parse_enemies)
    }

    fn load_maps() -> Result<Vec<Map>, LoadError> {
        Self::load_resource("data/maps.dat", parse_maps)
    }

    fn load_npcs() -> Result<Vec<Npc>, LoadError> {
        Self::load_resource("data/npcs.dat", parse_npcs)
    }

    fn load_dialogs() -> Result<Vec<Dialog>, LoadError> {
        Self::load_resource("data/dialogs.dat", parse_dialogs)
    }

    fn load_quests() -> Result<Vec<Quest>, LoadError> {
        Self::load_resource("data/quests.dat", parse_quests)
    }

    fn load_shops() -> Result<Vec<Shop>, LoadError> {
        Self::load_resource("data/shops.dat", parse_shops)
    }

    pub fn find_map(&self, id: &str) -> Option<&Map> {
        self.maps.iter().find(|m| m.id == id)
    }

    pub fn find_map_by_player(&self, current_map_id: &str) -> Option<&Map> {
        self.maps.iter().find(|m| m.id == current_map_id)
    }

    pub fn find_item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn find_dialog(&self, id: &str) -> Option<&Dialog> {
        self.dialogs.iter().find(|d| d.id == id)
    }

    pub fn find_quest(&self, id: &str) -> Option<&Quest> {
        self.quests.iter().find(|q| q.id == id)
    }

    pub fn find_shop(&self, id: &str) -> Option<&Shop> {
        self.shops.iter().find(|s| s.id == id)
    }

    pub fn find_npc_at(&self, map_id: &str, x: usize, y: usize) -> Option<&Npc> {
        self.npcs
            .iter()
            .find(|npc| npc.map_id == map_id && npc.x == x && npc.y == y)
    }

    pub fn get_shop_items(&self, shop: &Shop) -> Vec<Item> {
        shop.items
            .iter()
            .filter_map(|item_id| self.find_item(item_id).cloned())
            .collect()
    }
}
