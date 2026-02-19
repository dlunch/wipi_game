use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};
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
    item_index: BTreeMap<String, usize>,
    item_data_id_index: BTreeMap<u32, usize>,
    enemy_index: BTreeMap<String, usize>,
    enemy_name_index: BTreeMap<String, usize>,
    enemy_data_id_index: BTreeMap<u32, usize>,
    map_index: BTreeMap<String, usize>,
    npc_data_id_index: BTreeMap<u32, usize>,
    dialog_index: BTreeMap<String, usize>,
    quest_index: BTreeMap<String, usize>,
    shop_index: BTreeMap<String, usize>,
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
            item_data_id_index: BTreeMap::new(),
            enemy_index: BTreeMap::new(),
            enemy_name_index: BTreeMap::new(),
            enemy_data_id_index: BTreeMap::new(),
            map_index: BTreeMap::new(),
            npc_data_id_index: BTreeMap::new(),
            dialog_index: BTreeMap::new(),
            quest_index: BTreeMap::new(),
            shop_index: BTreeMap::new(),
        }
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

    pub fn find_map(&self, id: &str) -> Result<&Map> {
        Self::find_indexed(&self.maps, &self.map_index, id, |map| map.id.as_str())
            .ok_or_else(|| anyhow!("map not found: {}", id))
    }

    pub fn find_item(&self, id: &str) -> Result<&Item> {
        Self::find_indexed(&self.items, &self.item_index, id, |item| item.id.as_str())
            .ok_or_else(|| anyhow!("item not found: {}", id))
    }

    pub fn find_enemy(&self, id: &str) -> Result<&Enemy> {
        Self::find_indexed(&self.enemies, &self.enemy_index, id, |enemy| {
            enemy.id.as_str()
        })
        .ok_or_else(|| anyhow!("enemy not found: {}", id))
    }

    pub fn find_enemy_by_name(&self, name: &str) -> Result<&Enemy> {
        Self::find_indexed(&self.enemies, &self.enemy_name_index, name, |enemy| {
            enemy.name.as_str()
        })
        .ok_or_else(|| anyhow!("enemy name not found: {}", name))
    }

    pub fn find_dialog(&self, id: &str) -> Result<&Dialog> {
        Self::find_indexed(&self.dialogs, &self.dialog_index, id, |dialog| {
            dialog.id.as_str()
        })
        .ok_or_else(|| anyhow!("dialog not found: {}", id))
    }

    pub fn find_quest(&self, id: &str) -> Result<&Quest> {
        Self::find_indexed(&self.quests, &self.quest_index, id, |quest| {
            quest.id.as_str()
        })
        .ok_or_else(|| anyhow!("quest not found: {}", id))
    }

    pub fn find_shop(&self, id: &str) -> Result<&Shop> {
        Self::find_indexed(&self.shops, &self.shop_index, id, |shop| shop.id.as_str())
            .ok_or_else(|| anyhow!("shop not found: {}", id))
    }

    pub fn newgame_config(&self) -> &NewGameConfig {
        &self.newgame
    }

    pub fn npcs(&self) -> &[Npc] {
        &self.npcs
    }

    pub fn find_npc_at(&self, map_id: &str, x: usize, y: usize) -> Option<&Npc> {
        self.npcs
            .iter()
            .find(|npc| npc.map_id == map_id && npc.x == x && npc.y == y)
    }

    fn resolve_map_npcs(maps: &[Map], npc_defs: Vec<Npc>) -> Result<Vec<Npc>> {
        let placement_count = maps.iter().map(|map| map.npcs.len()).sum();
        let mut npcs = Vec::with_capacity(placement_count);
        let mut npc_index = BTreeMap::new();

        for npc in npc_defs {
            let npc_id = npc.id.clone();
            ensure!(
                npc_index.insert(npc_id.clone(), npc).is_none(),
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
                npc.map_id = map.id.clone();
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
        Self::rebuild_index(&mut self.item_index, &self.items, |item| item.id.as_str());
    }

    fn rebuild_item_data_id_index(&mut self) {
        self.item_data_id_index.clear();
        for (idx, item) in self.items.iter().enumerate() {
            self.item_data_id_index.insert(item.data_id, idx);
        }
    }

    fn rebuild_enemy_index(&mut self) {
        Self::rebuild_index(&mut self.enemy_index, &self.enemies, |enemy| {
            enemy.id.as_str()
        });
    }

    fn rebuild_enemy_name_index(&mut self) {
        Self::rebuild_index(&mut self.enemy_name_index, &self.enemies, |enemy| {
            enemy.name.as_str()
        });
    }

    fn rebuild_enemy_data_id_index(&mut self) {
        self.enemy_data_id_index.clear();
        for (idx, enemy) in self.enemies.iter().enumerate() {
            self.enemy_data_id_index.insert(enemy.data_id, idx);
        }
    }

    fn rebuild_npc_data_id_index(&mut self) {
        self.npc_data_id_index.clear();
        for (idx, npc) in self.npcs.iter().enumerate() {
            self.npc_data_id_index.insert(npc.data_id, idx);
        }
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

pub fn load_step(data: &mut GameData, step: usize) -> Result<bool> {
    match step {
        0 => {
            data.items = load_resource_with(data, "data/items.dat", parse_items)?;
            data.rebuild_item_index();
            data.rebuild_item_data_id_index();
        }
        1 => {
            data.enemies = load_resource_with(data, "data/enemies.dat", parse_enemies)?;
            data.rebuild_enemy_index();
            data.rebuild_enemy_name_index();
            data.rebuild_enemy_data_id_index();
        }
        2 => {
            data.maps = load_resource_with(data, "data/maps.dat", parse_maps)?;
            data.rebuild_map_index();
        }
        3 => {
            let npc_defs = load_resource_with(data, "data/npcs.dat", parse_npcs)?;
            data.npcs = GameData::resolve_map_npcs(&data.maps, npc_defs)?;
            data.rebuild_npc_data_id_index();
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
