use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, bail, ensure};

use super::{parse_int, parse_usize};
use crate::data::types::{Map, Tile};

pub fn parse_maps(data: &str) -> Result<Vec<Map>> {
    let mut maps = Vec::new();
    let mut current_map: Option<MapBuilder> = None;

    for raw_line in data.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("@MAP:") {
            if let Some(builder) = current_map.take() {
                maps.push(builder.build()?);
            }
            ensure!(!rest.is_empty(), "missing map id in: {}", line);
            let (id_raw, name_raw) = rest
                .split_once(':')
                .ok_or_else(|| anyhow!("missing ':' separator in map line: {}", line))?;
            let id = id_raw.to_string();
            let name = name_raw.to_string();

            current_map = Some(MapBuilder::new(id, name));
        } else if line == "@END" {
            if let Some(builder) = current_map.take() {
                maps.push(builder.build()?);
            }
        } else if let Some(rest) = line.strip_prefix("@ENCOUNTERS:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.split(':');
                while let Some(enemy_id) = parts.next() {
                    let Some(weight_raw) = parts.next() else {
                        break;
                    };
                    let weight = parse_int(weight_raw, "encounter weight", line)?;
                    builder.encounters.push((enemy_id.to_string(), weight));
                }
            }
        } else if let Some(rest) = line.strip_prefix("@NEXT:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.splitn(3, ':');
                let Some(x_raw) = parts.next() else {
                    bail!("too few fields in @NEXT directive: {}", line);
                };
                let Some(y_raw) = parts.next() else {
                    bail!("too few fields in @NEXT directive: {}", line);
                };
                let Some(target_raw) = parts.next() else {
                    bail!("too few fields in @NEXT directive: {}", line);
                };
                let x = parse_usize(x_raw, "exit x", line)?;
                let y = parse_usize(y_raw, "exit y", line)?;
                let target = target_raw.to_string();
                builder.exits.push((x, y, target));
            }
        } else if let Some(rest) = line.strip_prefix("@DUNGEON:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.splitn(3, ':');
                let Some(x_raw) = parts.next() else {
                    bail!("too few fields in @DUNGEON directive: {}", line);
                };
                let Some(y_raw) = parts.next() else {
                    bail!("too few fields in @DUNGEON directive: {}", line);
                };
                let Some(target_raw) = parts.next() else {
                    bail!("too few fields in @DUNGEON directive: {}", line);
                };
                let x = parse_usize(x_raw, "dungeon x", line)?;
                let y = parse_usize(y_raw, "dungeon y", line)?;
                let target = target_raw.to_string();
                builder.dungeons.push((x, y, target));
            }
        } else if line == "@PEACEFUL" {
            if let Some(ref mut builder) = current_map {
                builder.peaceful = true;
            }
        } else if let Some(rest) = line.strip_prefix("@NPC:") {
            if let Some(ref mut builder) = current_map {
                let mut parts = rest.splitn(3, ':');
                let Some(x_raw) = parts.next() else {
                    bail!("too few fields in @NPC directive: {}", line);
                };
                let Some(y_raw) = parts.next() else {
                    bail!("too few fields in @NPC directive: {}", line);
                };
                let Some(id_raw) = parts.next() else {
                    bail!("too few fields in @NPC directive: {}", line);
                };
                let x = parse_usize(x_raw, "npc x", line)?;
                let y = parse_usize(y_raw, "npc y", line)?;
                let npc_id = id_raw.to_string();
                builder.npcs.push((x, y, npc_id));
            }
        } else if !line.is_empty()
            && let Some(ref mut builder) = current_map
        {
            builder.add_row(line);
        }
    }

    if let Some(builder) = current_map {
        maps.push(builder.build()?);
    }

    Ok(maps)
}

struct MapBuilder {
    id: String,
    name: String,
    rows: Vec<String>,
    encounters: Vec<(String, i32)>,
    exits: Vec<(usize, usize, String)>,
    dungeons: Vec<(usize, usize, String)>,
    npcs: Vec<(usize, usize, String)>,
    peaceful: bool,
}

impl MapBuilder {
    fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            rows: Vec::new(),
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        }
    }

    fn add_row(&mut self, row: &str) {
        self.rows.push(row.to_string());
    }

    fn build(self) -> Result<Map> {
        ensure!(!self.rows.is_empty(), "map '{}' has no tile rows", self.id);

        let height = self.rows.len();
        let width = self.rows[0].len();

        let mut tiles = vec![Tile::Floor; width * height];
        let mut auto_exits = Vec::new();

        for (y, row) in self.rows.iter().enumerate() {
            let row_width = row.len();
            ensure!(
                row_width <= width,
                "map '{}' row {} is longer than first row ({} > {})",
                self.id,
                y,
                row_width,
                width
            );
            for (x, b) in row.bytes().enumerate() {
                let tile = Tile::from_char(b as char);
                tiles[y * width + x] = tile;

                if tile == Tile::Exit {
                    auto_exits.push((x, y));
                }
            }
        }

        let mut exits = self.exits;
        for (x, y) in auto_exits {
            if !exits.iter().any(|(ex, ey, _)| *ex == x && *ey == y) {
                exits.push((x, y, String::new()));
            }
        }

        Ok(Map {
            id: self.id,
            name: self.name,
            width,
            height,
            tiles,
            encounters: self.encounters,
            exits,
            dungeons: self.dungeons,
            npcs: self.npcs,
            peaceful: self.peaceful,
        })
    }
}
