use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::str;

use anyhow::{Context, Result, anyhow, bail, ensure};
use wipi::resource::Resource;

use crate::data::{
    Dialog, Enemy, Item, Map, NewGameConfig, Npc, Quest, Shop, parse_dialogs, parse_enemies,
    parse_items, parse_maps, parse_newgame, parse_npcs, parse_quests, parse_shops,
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
    pub newgame: NewGameConfig,
    item_index: BTreeMap<String, usize>,
    enemy_index: BTreeMap<String, usize>,
    map_index: BTreeMap<String, usize>,
    dialog_index: BTreeMap<String, usize>,
    quest_index: BTreeMap<String, usize>,
    shop_index: BTreeMap<String, usize>,
}

impl GameData {
    pub fn load_step(&mut self, step: usize) -> Result<bool> {
        match step {
            0 => {
                self.items = Self::load_resource("data/items.dat", parse_items)?;
                self.rebuild_item_index();
            }
            1 => {
                self.enemies = Self::load_resource("data/enemies.dat", parse_enemies)?;
                self.rebuild_enemy_index();
            }
            2 => {
                self.maps = Self::load_resource("data/maps.dat", parse_maps)?;
                self.rebuild_map_index();
            }
            3 => {
                let npc_defs = Self::load_resource("data/npcs.dat", parse_npcs)?;
                self.npcs = Self::resolve_map_npcs(&self.maps, &npc_defs)?;
            }
            4 => {
                self.dialogs = Self::load_resource("data/dialogs.dat", parse_dialogs)?;
                self.rebuild_dialog_index();
            }
            5 => {
                self.quests = Self::load_resource("data/quests.dat", parse_quests)?;
                self.rebuild_quest_index();
            }
            6 => {
                self.shops = Self::load_resource("data/shops.dat", parse_shops)?;
                self.rebuild_shop_index();
            }
            7 => self.newgame = Self::load_resource("data/newgame.dat", parse_newgame)?,
            _ => return Ok(true),
        }
        Ok(false)
    }

    fn load_resource<T>(path: &str, parser: fn(&str) -> Result<T>) -> Result<T> {
        let resource = Resource::new(path)
            .map_err(|e| anyhow!("failed to open resource '{}': {:?}", path, e))?;
        let text = str::from_utf8(resource.read())
            .with_context(|| format!("invalid UTF-8 in '{}'", path))?;
        parser(text).with_context(|| format!("failed to parse '{}'", path))
    }

    pub fn find_map(&self, id: &str) -> Option<&Map> {
        if let Some(idx) = self.map_index.get(id).copied() {
            return self.maps.get(idx);
        }
        self.maps.iter().find(|map| map.id == id)
    }

    pub fn find_item(&self, id: &str) -> Option<&Item> {
        if let Some(idx) = self.item_index.get(id).copied() {
            return self.items.get(idx);
        }
        self.items.iter().find(|item| item.id == id)
    }

    pub fn find_enemy(&self, id: &str) -> Option<&Enemy> {
        if let Some(idx) = self.enemy_index.get(id).copied() {
            return self.enemies.get(idx);
        }
        self.enemies.iter().find(|enemy| enemy.id == id)
    }

    pub fn find_dialog(&self, id: &str) -> Option<&Dialog> {
        if let Some(idx) = self.dialog_index.get(id).copied() {
            return self.dialogs.get(idx);
        }
        self.dialogs.iter().find(|dialog| dialog.id == id)
    }

    pub fn find_quest(&self, id: &str) -> Option<&Quest> {
        if let Some(idx) = self.quest_index.get(id).copied() {
            return self.quests.get(idx);
        }
        self.quests.iter().find(|quest| quest.id == id)
    }

    pub fn find_shop(&self, id: &str) -> Option<&Shop> {
        if let Some(idx) = self.shop_index.get(id).copied() {
            return self.shops.get(idx);
        }
        self.shops.iter().find(|shop| shop.id == id)
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

    fn resolve_map_npcs(maps: &[Map], npc_defs: &[Npc]) -> Result<Vec<Npc>> {
        let mut npcs = Vec::new();
        let mut npc_index: BTreeMap<&str, usize> = BTreeMap::new();
        let mut used = vec![false; npc_defs.len()];

        for (idx, def) in npc_defs.iter().enumerate() {
            npc_index.insert(def.id.as_str(), idx);
        }

        let placement_count = maps.iter().map(|map| map.npcs.len()).sum();
        npcs.reserve(placement_count);

        for map in maps {
            for (x, y, npc_id) in &map.npcs {
                ensure!(
                    *x < map.width && *y < map.height,
                    "npc '{}' out of bounds in map '{}' at ({}, {})",
                    npc_id,
                    map.id,
                    x,
                    y
                );
                ensure!(
                    map.get_tile(*x, *y).is_passable(),
                    "npc '{}' is on impassable tile in map '{}' at ({}, {})",
                    npc_id,
                    map.id,
                    x,
                    y
                );

                let Some(def_idx) = npc_index.get(npc_id.as_str()).copied() else {
                    bail!("map '{}' references unknown npc id '{}'", map.id, npc_id);
                };
                let def = &npc_defs[def_idx];
                used[def_idx] = true;

                let mut npc = def.clone();
                npc.map_id = map.id.clone();
                npc.x = *x;
                npc.y = *y;
                npcs.push(npc);
            }
        }

        for (idx, def) in npc_defs.iter().enumerate() {
            ensure!(
                used[idx],
                "npc '{}' is defined but not placed in any map",
                def.id
            );
        }

        Ok(npcs)
    }

    fn rebuild_item_index(&mut self) {
        self.item_index.clear();
        for (idx, item) in self.items.iter().enumerate() {
            self.item_index.insert(item.id.clone(), idx);
        }
    }

    fn rebuild_enemy_index(&mut self) {
        self.enemy_index.clear();
        for (idx, enemy) in self.enemies.iter().enumerate() {
            self.enemy_index.insert(enemy.id.clone(), idx);
        }
    }

    fn rebuild_map_index(&mut self) {
        self.map_index.clear();
        for (idx, map) in self.maps.iter().enumerate() {
            self.map_index.insert(map.id.clone(), idx);
        }
    }

    fn rebuild_dialog_index(&mut self) {
        self.dialog_index.clear();
        for (idx, dialog) in self.dialogs.iter().enumerate() {
            self.dialog_index.insert(dialog.id.clone(), idx);
        }
    }

    fn rebuild_quest_index(&mut self) {
        self.quest_index.clear();
        for (idx, quest) in self.quests.iter().enumerate() {
            self.quest_index.insert(quest.id.clone(), idx);
        }
    }

    fn rebuild_shop_index(&mut self) {
        self.shop_index.clear();
        for (idx, shop) in self.shops.iter().enumerate() {
            self.shop_index.insert(shop.id.clone(), idx);
        }
    }
}
