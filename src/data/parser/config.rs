use alloc::string::ToString;
use alloc::vec::Vec;

use anyhow::{Result, bail, ensure};

use super::parse_int;
use crate::data::types::{NewGameConfig, StartItem};

pub fn parse_newgame(data: &str) -> Result<NewGameConfig> {
    let mut config = NewGameConfig::default();

    for raw_line in data.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        ensure!(!parts.is_empty(), "empty directive in newgame config");

        match parts[0] {
            "player_name" => {
                ensure!(parts.len() >= 2, "missing value for player_name");
                config.player_name = parts[1].to_string();
            }
            "start_map" => {
                ensure!(parts.len() >= 2, "missing value for start_map");
                config.start_map = parts[1].to_string();
            }
            "fallback_map" => {
                ensure!(parts.len() >= 2, "missing value for fallback_map");
                config.fallback_map = parts[1].to_string();
            }
            "intro_dialog" => {
                ensure!(
                    parts.len() >= 3,
                    "intro_dialog requires dialog_id and npc_name"
                );
                config.intro_dialog = Some((parts[1].to_string(), parts[2].to_string()));
            }
            "equip_weapon" => {
                ensure!(parts.len() >= 2, "missing value for equip_weapon");
                config.equip_weapon = Some(parts[1].to_string());
            }
            "equip_armor" => {
                ensure!(parts.len() >= 2, "missing value for equip_armor");
                config.equip_armor = Some(parts[1].to_string());
            }
            "treasure_item" => {
                ensure!(parts.len() >= 2, "missing value for treasure_item");
                config.treasure_item = Some(parts[1].to_string());
            }
            "item" => {
                ensure!(parts.len() >= 3, "item requires item_id and count");
                let count = parse_int(parts[2], "item count", line)?;
                config.items.push(StartItem {
                    item_id: parts[1].to_string(),
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
