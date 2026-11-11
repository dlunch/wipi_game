use anyhow::{Result, anyhow};

mod config;
mod dialog;
mod game_data;
mod world;

pub use config::parse_newgame;
pub use dialog::parse_dialogs;
pub use game_data::{parse_enemies, parse_items, parse_npcs, parse_quests, parse_shops};
pub use world::parse_maps;

fn parse_int(s: &str, field: &str, line: &str) -> Result<i32> {
    s.parse::<i32>()
        .map_err(|_| anyhow!("invalid {} '{}' in: {}", field, s, line))
}

fn parse_usize(s: &str, field: &str, line: &str) -> Result<usize> {
    s.parse::<usize>()
        .map_err(|_| anyhow!("invalid {} '{}' in: {}", field, s, line))
}

fn parse_u32(s: &str, field: &str, line: &str) -> Result<u32> {
    s.parse::<u32>()
        .map_err(|_| anyhow!("invalid {} '{}' in: {}", field, s, line))
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use anyhow::Result;

    use super::{
        parse_dialogs, parse_enemies, parse_items, parse_maps, parse_newgame, parse_npcs,
        parse_quests, parse_shops,
    };
    use crate::data::types::{DialogAction, DialogCondition, ItemKind, NpcType, QuestType, Tile};

    #[test]
    fn parse_items_weapon_and_consumable() -> Result<()> {
        let data = "W:1001:Iron Sword:10:5:200\nI:1301:Potion:30:50\n";
        let items = parse_items(data)?;
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, 1001);
        assert_eq!(items[0].kind, ItemKind::Weapon);
        assert_eq!(items[0].param1, 10);
        assert_eq!(items[0].price, 200);

        assert_eq!(items[1].id, 1301);
        assert_eq!(items[1].kind, ItemKind::Consumable);
        assert_eq!(items[1].param1, 30);
        assert_eq!(items[1].price, 50);
        Ok(())
    }

    #[test]
    fn parse_items_armor_and_accessory() -> Result<()> {
        let data = "A:1102:Leather:5:2:150\nC:1201:Ring:3:1:300\n";
        let items = parse_items(data)?;
        assert_eq!(items.len(), 2);

        assert_eq!(items[0].id, 1102);
        assert_eq!(items[0].kind, ItemKind::Armor);
        assert_eq!(items[0].param1, 5);

        assert_eq!(items[1].id, 1201);
        assert_eq!(items[1].kind, ItemKind::Accessory);
        assert_eq!(items[1].param1, 3);
        Ok(())
    }

    #[test]
    fn parse_items_skips_comments_and_empty() -> Result<()> {
        let data = "# comment\n\nW:1001:Sword:10:0:100\n";
        let items = parse_items(data)?;
        assert_eq!(items.len(), 1);
        Ok(())
    }

    #[test]
    fn parse_items_rejects_too_few_fields() {
        let data = "W:too:few\n";
        assert!(parse_items(data).is_err());
    }

    #[test]
    fn parse_items_rejects_unknown_kind() {
        let data = "X:unknown:type:1:2:3\n";
        assert!(parse_items(data).is_err());
    }

    #[test]
    fn parse_items_rejects_bad_number() {
        let data = "W:1001:Sword:abc:5:200\n";
        assert!(parse_items(data).is_err());
    }

    #[test]
    fn parse_enemies_basic() -> Result<()> {
        let data = "2001:Slime:20:5:2:10:5\n2004:Goblin:30:8:3:15:8\n";
        let enemies = parse_enemies(data)?;
        assert_eq!(enemies.len(), 2);
        assert_eq!(enemies[0].id, 2001);
        assert_eq!(enemies[0].hp, 20);
        assert_eq!(enemies[0].atk, 5);
        assert_eq!(enemies[1].id, 2004);
        assert_eq!(enemies[1].gold, 8);
        Ok(())
    }

    #[test]
    fn parse_enemies_rejects_short_lines() {
        let data = "too:few:fields\n";
        assert!(parse_enemies(data).is_err());
    }

    #[test]
    fn parse_enemies_rejects_zero_hp() {
        let data = "2001:Slime:0:5:2:10:5\n";
        assert!(parse_enemies(data).is_err());
    }

    #[test]
    fn parse_maps_basic() -> Result<()> {
        let data = "\
@MAP:1:Village
###
#P#
###
@ENCOUNTERS:2001:3
@NEXT:1:1:3
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps.len(), 1);
        let map = &maps[0];
        assert_eq!(map.id, 1);
        assert_eq!(map.name, "Village");
        assert_eq!(map.width, 3);
        assert_eq!(map.height, 3);
        assert_eq!(map.get_tile(1, 1), Tile::PlayerStart);
        assert_eq!(map.get_tile(0, 0), Tile::Wall);
        assert_eq!(map.encounters.len(), 1);
        assert_eq!(map.encounters[0].0, 2001);
        assert_eq!(map.exits.len(), 1);
        assert_eq!(map.exits[0].2, 3);
        Ok(())
    }

    #[test]
    fn parse_maps_multiple() -> Result<()> {
        let data = "\
@MAP:1:Map A
#P
@END
@MAP:2:Map B
P#
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].id, 1);
        assert_eq!(maps[1].id, 2);
        Ok(())
    }

    #[test]
    fn parse_maps_auto_exit_tiles() -> Result<()> {
        let data = "\
@MAP:1:Test
#>#
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps[0].exits.len(), 1);
        assert_eq!(maps[0].exits[0].0, 1);
        assert_eq!(maps[0].exits[0].1, 0);
        Ok(())
    }

    #[test]
    fn parse_maps_dungeon_directive() -> Result<()> {
        let data = "\
@MAP:1:Test
#D#
@DUNGEON:1:0:3
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps[0].dungeons.len(), 1);
        assert_eq!(maps[0].dungeons[0].2, 3);
        Ok(())
    }

    #[test]
    fn parse_maps_npc_directive() -> Result<()> {
        let data = "\
@MAP:1:Test
#P#
@NPC:1:0:3001
@END
";
        let maps = parse_maps(data)?;
        assert_eq!(maps[0].npcs.len(), 1);
        assert_eq!(maps[0].npcs[0].0, 1);
        assert_eq!(maps[0].npcs[0].1, 0);
        assert_eq!(maps[0].npcs[0].2, 3001);
        Ok(())
    }

    #[test]
    fn parse_maps_rejects_row_longer_than_first_row() {
        let data = r#"
@MAP:1:Test
##
###
@END
"#;

        assert!(parse_maps(data).is_err());
    }

    #[test]
    fn parse_dialogs_basic() -> Result<()> {
        let data = "\
@DIALOG:1001
Hello there!
HEAL:I'll heal you.
@END
";
        let dialogs = parse_dialogs(data)?;
        assert_eq!(dialogs.len(), 1);
        assert_eq!(dialogs[0].id, 1001);
        assert_eq!(dialogs[0].lines.len(), 2);
        assert_eq!(dialogs[0].lines[0].text, "Hello there!");
        assert!(dialogs[0].lines[0].action.is_none());
        assert!(matches!(
            dialogs[0].lines[1].action,
            Some(DialogAction::Heal)
        ));
        Ok(())
    }

    #[test]
    fn parse_dialogs_with_condition_and_action() -> Result<()> {
        let data = "\
@DIALOG:1002
HAS_QUEST=3001:COMPLETE_QUEST=3001:Well done!
GIVE_QUEST=3001:Kill the slimes!
@END
";
        let dialogs = parse_dialogs(data)?;
        let lines = &dialogs[0].lines;
        assert_eq!(lines.len(), 2);
        assert!(matches!(
            lines[0].condition,
            Some(DialogCondition::HasQuest(_))
        ));
        assert!(matches!(
            lines[0].action,
            Some(DialogAction::CompleteQuest(_))
        ));
        assert!(matches!(lines[1].action, Some(DialogAction::GiveQuest(_))));
        Ok(())
    }

    #[test]
    fn parse_quests_basic() -> Result<()> {
        let data = "3001:Slay Quest:KILL:2001:5:50:20:Kill 5 slimes\n";
        let quests = parse_quests(data)?;
        assert_eq!(quests.len(), 1);
        assert_eq!(quests[0].id, 3001);
        assert_eq!(quests[0].quest_type, QuestType::Kill);
        assert_eq!(quests[0].target_id, 2001);
        assert_eq!(quests[0].target_count, 5);
        assert_eq!(quests[0].reward_exp, 50);
        assert_eq!(quests[0].reward_gold, 20);
        assert!(quests[0].reward_item.is_none());
        Ok(())
    }

    #[test]
    fn parse_quests_with_reward_item() -> Result<()> {
        let data = "3002:Quest:COLLECT:1301:3:30:10:Collect herbs:1302\n";
        let quests = parse_quests(data)?;
        assert_eq!(quests[0].reward_item, Some(1302));
        Ok(())
    }

    #[test]
    fn parse_quests_rejects_unknown_type() {
        let data = "3001:Quest:INVALID:1:1:10:5:desc\n";
        assert!(parse_quests(data).is_err());
    }

    #[test]
    fn parse_shops_basic() -> Result<()> {
        let data = "2001:General Store:1301:1001:1101\n";
        let shops = parse_shops(data)?;
        assert_eq!(shops.len(), 1);
        assert_eq!(shops[0].id, 2001);
        assert_eq!(shops[0].name, "General Store");
        assert_eq!(shops[0].items, vec![1301, 1001, 1101]);
        Ok(())
    }

    #[test]
    fn parse_npcs_basic() -> Result<()> {
        let data = "3001:Elder:Q:1001\n";
        let npcs = parse_npcs(data)?;
        assert_eq!(npcs.len(), 1);
        assert_eq!(npcs[0].id, 3001);
        assert_eq!(npcs[0].name, "Elder");
        assert_eq!(npcs[0].npc_type, NpcType::QuestGiver);
        assert_eq!(npcs[0].dialog_id, 1001);
        Ok(())
    }

    #[test]
    fn parse_npcs_with_shop() -> Result<()> {
        let data = "3002:Merchant:S:1002:2001\n";
        let npcs = parse_npcs(data)?;
        assert_eq!(npcs[0].id, 3002);
        assert_eq!(npcs[0].npc_type, NpcType::ShopKeeper);
        assert_eq!(npcs[0].shop_id, Some(2001));
        Ok(())
    }

    #[test]
    fn parse_npcs_rejects_unknown_type() {
        let data = "3001:Test:Z:1001\n";
        assert!(parse_npcs(data).is_err());
    }

    #[test]
    fn parse_newgame_full_config() -> Result<()> {
        let data = "player_name:Hero\nstart_map:1\nfallback_map:1\nintro_dialog:1010:3010\nequip_weapon:1001\nequip_armor:1101\ntreasure_item:1302\nitem:1301:2\nitem:1302:1\n";
        let config = parse_newgame(data)?;
        assert_eq!(config.player_name, "Hero");
        assert_eq!(config.start_map, 1);
        assert_eq!(config.fallback_map, 1);
        assert_eq!(config.intro_dialog, Some((1010, 3010)));
        assert_eq!(config.equip_weapon, Some(1001));
        assert_eq!(config.equip_armor, Some(1101));
        assert_eq!(config.treasure_item, Some(1302));
        assert_eq!(config.items.len(), 2);
        assert_eq!(config.items[0].item_id, 1301);
        assert_eq!(config.items[0].count, 2);
        assert_eq!(config.items[1].item_id, 1302);
        assert_eq!(config.items[1].count, 1);
        Ok(())
    }

    #[test]
    fn parse_newgame_defaults() -> Result<()> {
        let data = "";
        let config = parse_newgame(data)?;
        assert_eq!(config.player_name, "Hero");
        assert_eq!(config.start_map, 1);
        assert!(config.intro_dialog.is_none());
        assert!(config.equip_weapon.is_none());
        assert_eq!(config.treasure_item, Some(1301));
        assert!(config.items.is_empty());
        Ok(())
    }

    #[test]
    fn parse_newgame_skips_comments_and_empty() -> Result<()> {
        let data = "# comment\n\nplayer_name:Test\n";
        let config = parse_newgame(data)?;
        assert_eq!(config.player_name, "Test");
        Ok(())
    }

    #[test]
    fn parse_newgame_rejects_unknown_directive() {
        let data = "unknown_key:value\n";
        assert!(parse_newgame(data).is_err());
    }

    #[test]
    fn parse_newgame_rejects_bad_item_count() {
        let data = "item:1301:abc\n";
        assert!(parse_newgame(data).is_err());
    }
}
