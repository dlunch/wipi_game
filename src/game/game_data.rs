use alloc::vec::Vec;
use core::str;

use anyhow::{Context, Result};
use wipi::resource::Resource;

use crate::data::{
    Dialog, Enemy, Item, Map, Npc, Quest, Shop, parse_dialogs, parse_enemies, parse_items,
    parse_maps, parse_npcs, parse_quests, parse_shops,
};

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

    pub fn load_step(&mut self, step: usize) -> Result<bool> {
        match step {
            0 => self.items = Self::load_resource("data/items.dat", parse_items)?,
            1 => self.enemies = Self::load_resource("data/enemies.dat", parse_enemies)?,
            2 => self.maps = Self::load_resource("data/maps.dat", parse_maps)?,
            3 => self.npcs = Self::load_resource("data/npcs.dat", parse_npcs)?,
            4 => self.dialogs = Self::load_resource("data/dialogs.dat", parse_dialogs)?,
            5 => self.quests = Self::load_resource("data/quests.dat", parse_quests)?,
            6 => self.shops = Self::load_resource("data/shops.dat", parse_shops)?,
            _ => return Ok(true),
        }
        Ok(false)
    }

    fn load_resource<T>(path: &str, parser: fn(&str) -> Result<T>) -> Result<T> {
        let resource = Resource::new(path)
            .map_err(|e| anyhow::anyhow!("failed to open resource '{}': {:?}", path, e))?;
        let text = str::from_utf8(resource.read())
            .with_context(|| alloc::format!("invalid UTF-8 in '{}'", path))?;
        parser(text).with_context(|| alloc::format!("failed to parse '{}'", path))
    }

    pub fn find_map(&self, id: &str) -> Option<&Map> {
        self.maps.iter().find(|m| m.id == id)
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
