use alloc::{boxed::Box, collections::BTreeMap, format, vec::Vec};
use core::str;

use anyhow::{Context, Result, anyhow, bail, ensure};

use crate::data::{
    Dialog, Enemy, Item, Map, NewGameConfig, Npc, Quest, Shop, parse_dialogs, parse_enemies,
    parse_items, parse_maps, parse_newgame, parse_npcs, parse_quests, parse_shops,
};

pub struct GameData {
    resource_loader: ResourceLoader,
    items: Vec<Item>,
    enemies: Vec<Enemy>,
    maps: Vec<Map>,
    npcs: Vec<Npc>,
    dialogs: Vec<Dialog>,
    quests: Vec<Quest>,
    shops: Vec<Shop>,
    newgame: NewGameConfig,
    item_index: BTreeMap<u32, usize>,
    enemy_index: BTreeMap<u32, usize>,
    map_index: BTreeMap<u32, usize>,
    npc_index: BTreeMap<u32, usize>,
    dialog_index: BTreeMap<u32, usize>,
    quest_index: BTreeMap<u32, usize>,
    shop_index: BTreeMap<u32, usize>,
}

type ResourceLoader = Box<dyn Fn(&str) -> Result<Vec<u8>>>;

impl GameData {
    pub fn new<F>(resource_loader: F) -> Self
    where
        F: Fn(&str) -> Result<Vec<u8>> + 'static,
    {
        Self {
            resource_loader: Box::new(resource_loader),
            items: Vec::new(),
            enemies: Vec::new(),
            maps: Vec::new(),
            npcs: Vec::new(),
            dialogs: Vec::new(),
            quests: Vec::new(),
            shops: Vec::new(),
            newgame: NewGameConfig::default(),
            item_index: BTreeMap::new(),
            enemy_index: BTreeMap::new(),
            map_index: BTreeMap::new(),
            npc_index: BTreeMap::new(),
            dialog_index: BTreeMap::new(),
            quest_index: BTreeMap::new(),
            shop_index: BTreeMap::new(),
        }
    }

    pub fn find_map(&self, id: u32) -> Result<&Map> {
        self.map_index
            .get(&id)
            .and_then(|idx| self.maps.get(*idx))
            .ok_or_else(|| anyhow!("map not found: {}", id))
    }

    pub fn find_item(&self, id: u32) -> Result<&Item> {
        self.item_index
            .get(&id)
            .and_then(|idx| self.items.get(*idx))
            .ok_or_else(|| anyhow!("item not found: {}", id))
    }

    pub fn find_enemy(&self, id: u32) -> Result<&Enemy> {
        self.enemy_index
            .get(&id)
            .and_then(|idx| self.enemies.get(*idx))
            .ok_or_else(|| anyhow!("enemy not found: {}", id))
    }

    pub fn find_dialog(&self, id: u32) -> Result<&Dialog> {
        self.dialog_index
            .get(&id)
            .and_then(|idx| self.dialogs.get(*idx))
            .ok_or_else(|| anyhow!("dialog not found: {}", id))
    }

    pub fn find_npc(&self, id: u32) -> Result<&Npc> {
        self.npc_index
            .get(&id)
            .and_then(|idx| self.npcs.get(*idx))
            .ok_or_else(|| anyhow!("npc not found: {}", id))
    }

    pub fn find_quest(&self, id: u32) -> Result<&Quest> {
        self.quest_index
            .get(&id)
            .and_then(|idx| self.quests.get(*idx))
            .ok_or_else(|| anyhow!("quest not found: {}", id))
    }

    pub fn find_shop(&self, id: u32) -> Result<&Shop> {
        self.shop_index
            .get(&id)
            .and_then(|idx| self.shops.get(*idx))
            .ok_or_else(|| anyhow!("shop not found: {}", id))
    }

    pub fn newgame_config(&self) -> &NewGameConfig {
        &self.newgame
    }

    pub fn npcs(&self) -> &[Npc] {
        &self.npcs
    }

    pub fn find_npc_at(&self, map_id: u32, x: usize, y: usize) -> Option<&Npc> {
        self.npcs
            .iter()
            .find(|npc| npc.map_id == map_id && npc.x == x && npc.y == y)
    }

    fn resolve_map_npcs(maps: &[Map], npc_defs: Vec<Npc>) -> Result<Vec<Npc>> {
        let placement_count = maps.iter().map(|map| map.npcs.len()).sum();
        let mut npcs = Vec::with_capacity(placement_count);
        let mut npc_index = BTreeMap::new();

        for npc in npc_defs {
            let npc_id = npc.id;
            ensure!(
                npc_index.insert(npc_id, npc).is_none(),
                "duplicate npc id '{}'",
                npc_id
            );
        }

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

                let mut npc = npc_index.remove(npc_id).ok_or_else(|| {
                    anyhow!("map '{}' references unknown npc id '{}'", map.id, npc_id)
                })?;
                npc.map_id = map.id;
                npc.x = *x;
                npc.y = *y;
                npcs.push(npc);
            }
        }

        if let Some((npc_id, _)) = npc_index.into_iter().next() {
            bail!("npc '{}' is defined but not placed in any map", npc_id);
        }

        Ok(npcs)
    }

    fn rebuild_item_index(&mut self) {
        self.item_index.clear();
        for (idx, item) in self.items.iter().enumerate() {
            self.item_index.insert(item.id, idx);
        }
    }

    fn rebuild_enemy_index(&mut self) {
        self.enemy_index.clear();
        for (idx, enemy) in self.enemies.iter().enumerate() {
            self.enemy_index.insert(enemy.id, idx);
        }
    }

    fn rebuild_npc_index(&mut self) {
        self.npc_index.clear();
        for (idx, npc) in self.npcs.iter().enumerate() {
            self.npc_index.insert(npc.id, idx);
        }
    }

    fn rebuild_map_index(&mut self) {
        self.map_index.clear();
        for (idx, map) in self.maps.iter().enumerate() {
            self.map_index.insert(map.id, idx);
        }
    }

    fn rebuild_dialog_index(&mut self) {
        self.dialog_index.clear();
        for (idx, dialog) in self.dialogs.iter().enumerate() {
            self.dialog_index.insert(dialog.id, idx);
        }
    }

    fn rebuild_quest_index(&mut self) {
        self.quest_index.clear();
        for (idx, quest) in self.quests.iter().enumerate() {
            self.quest_index.insert(quest.id, idx);
        }
    }

    fn rebuild_shop_index(&mut self) {
        self.shop_index.clear();
        for (idx, shop) in self.shops.iter().enumerate() {
            self.shop_index.insert(shop.id, idx);
        }
    }
}

pub fn load_step(data: &mut GameData, step: usize) -> Result<bool> {
    match step {
        0 => {
            data.items = load_resource_with(data, "data/items.dat", parse_items)?;
            data.rebuild_item_index();
        }
        1 => {
            data.enemies = load_resource_with(data, "data/enemies.dat", parse_enemies)?;
            data.rebuild_enemy_index();
        }
        2 => {
            data.maps = load_resource_with(data, "data/maps.dat", parse_maps)?;
            data.rebuild_map_index();
        }
        3 => {
            let npc_defs = load_resource_with(data, "data/npcs.dat", parse_npcs)?;
            data.npcs = GameData::resolve_map_npcs(&data.maps, npc_defs)?;
            data.rebuild_npc_index();
        }
        4 => {
            data.dialogs = load_resource_with(data, "data/dialogs.dat", parse_dialogs)?;
            data.rebuild_dialog_index();
        }
        5 => {
            data.quests = load_resource_with(data, "data/quests.dat", parse_quests)?;
            data.rebuild_quest_index();
        }
        6 => {
            data.shops = load_resource_with(data, "data/shops.dat", parse_shops)?;
            data.rebuild_shop_index();
        }
        7 => data.newgame = load_resource_with(data, "data/newgame.dat", parse_newgame)?,
        _ => return Ok(true),
    }
    Ok(false)
}

fn load_resource_with<T>(data: &GameData, path: &str, parser: fn(&str) -> Result<T>) -> Result<T> {
    let bytes = (data.resource_loader)(path)?;
    let text = str::from_utf8(&bytes).with_context(|| format!("invalid UTF-8 in '{}'", path))?;
    parser(text).with_context(|| format!("failed to parse '{}'", path))
}
