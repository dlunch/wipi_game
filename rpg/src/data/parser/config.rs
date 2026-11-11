use alloc::{string::ToString, vec::Vec};

use anyhow::{Result, bail, ensure};

use super::{parse_int, parse_u32};
use crate::data::types::{NewGameConfig, StartItem};

pub fn parse_newgame(data: &str) -> Result<NewGameConfig> {
    let mut config = NewGameConfig::default();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts = line.splitn(3, ':').collect::<Vec<_>>();
        ensure!(!parts.is_empty(), "empty directive in newgame config");

        match parts[0] {
            "player_name" => {
                ensure!(parts.len() >= 2, "missing value for player_name");
                config.player_name = parts[1].to_string();
            }
            "start_map" => {
                ensure!(parts.len() >= 2, "missing value for start_map");
                config.start_map = parse_u32(parts[1], "start_map", line)?;
            }
            "fallback_map" => {
                ensure!(parts.len() >= 2, "missing value for fallback_map");
                config.fallback_map = parse_u32(parts[1], "fallback_map", line)?;
            }
            "intro_dialog" => {
                ensure!(
                    parts.len() >= 3,
                    "intro_dialog requires dialog_id and npc_id"
                );
                config.intro_dialog = Some((
                    parse_u32(parts[1], "intro_dialog.dialog_id", line)?,
                    parse_u32(parts[2], "intro_dialog.npc_id", line)?,
                ));
            }
            "equip_weapon" => {
                ensure!(parts.len() >= 2, "missing value for equip_weapon");
                config.equip_weapon = Some(parse_u32(parts[1], "equip_weapon", line)?);
            }
            "equip_armor" => {
                ensure!(parts.len() >= 2, "missing value for equip_armor");
                config.equip_armor = Some(parse_u32(parts[1], "equip_armor", line)?);
            }
            "treasure_item" => {
                ensure!(parts.len() >= 2, "missing value for treasure_item");
                config.treasure_item = Some(parse_u32(parts[1], "treasure_item", line)?);
            }
            "item" => {
                ensure!(parts.len() >= 3, "item requires item_id and count");
                let count = parse_int(parts[2], "item count", line)?;
                config.items.push(StartItem {
                    item_id: parse_u32(parts[1], "item.item_id", line)?,
                    count,
                });
            }
            _ => bail!(
                "unknown directive '{}' in newgame config: {}",
                parts[0],
                line
            ),
        }
    }

    Ok(config)
}
