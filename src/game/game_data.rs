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

    fn find_indexed<'a, T>(
        items: &'a [T],
        index: &BTreeMap<String, usize>,
        id: &str,
        id_of: fn(&T) -> &str,
    ) -> Option<&'a T> {
        index
            .get(id)
            .and_then(|idx| items.get(*idx))
            .or_else(|| items.iter().find(|item| id_of(item) == id))
    }

    fn rebuild_index<T>(index: &mut BTreeMap<String, usize>, items: &[T], id_of: fn(&T) -> &str) {
        index.clear();
        for (idx, item) in items.iter().enumerate() {
            index.insert(String::from(id_of(item)), idx);
        }
    }

    pub fn find_map(&self, id: &str) -> Option<&Map> {
        Self::find_indexed(&self.maps, &self.map_index, id, |map| map.id.as_str())
    }

    pub fn find_item(&self, id: &str) -> Option<&Item> {
        Self::find_indexed(&self.items, &self.item_index, id, |item| item.id.as_str())
    }

    pub fn find_enemy(&self, id: &str) -> Option<&Enemy> {
        Self::find_indexed(&self.enemies, &self.enemy_index, id, |enemy| {
            enemy.id.as_str()
        })
    }

    pub fn find_dialog(&self, id: &str) -> Option<&Dialog> {
        Self::find_indexed(&self.dialogs, &self.dialog_index, id, |dialog| {
            dialog.id.as_str()
        })
    }

    pub fn find_quest(&self, id: &str) -> Option<&Quest> {
        Self::find_indexed(&self.quests, &self.quest_index, id, |quest| {
            quest.id.as_str()
        })
    }

    pub fn find_shop(&self, id: &str) -> Option<&Shop> {
        Self::find_indexed(&self.shops, &self.shop_index, id, |shop| shop.id.as_str())
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
        Self::rebuild_index(&mut self.item_index, &self.items, |item| item.id.as_str());
    }

    fn rebuild_enemy_index(&mut self) {
        Self::rebuild_index(&mut self.enemy_index, &self.enemies, |enemy| {
            enemy.id.as_str()
        });
    }

    fn rebuild_map_index(&mut self) {
        Self::rebuild_index(&mut self.map_index, &self.maps, |map| map.id.as_str());
    }

    fn rebuild_dialog_index(&mut self) {
        Self::rebuild_index(&mut self.dialog_index, &self.dialogs, |dialog| {
            dialog.id.as_str()
        });
    }

    fn rebuild_quest_index(&mut self) {
        Self::rebuild_index(&mut self.quest_index, &self.quests, |quest| {
            quest.id.as_str()
        });
    }

    fn rebuild_shop_index(&mut self) {
        Self::rebuild_index(&mut self.shop_index, &self.shops, |shop| shop.id.as_str());
    }
}
