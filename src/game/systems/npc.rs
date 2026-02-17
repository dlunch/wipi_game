use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Dialog, DialogCondition, DialogLine, Direction, NpcType};

use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{
    DialogState, ExploreEvent, GameData, GameEvent, GameEventKind, GameState, WorldState,
};

#[derive(Debug, Clone)]
pub struct DialogSpec {
    pub npc_name: String,
    pub lines: Vec<DialogLine>,
    pub restore: bool,
}

#[derive(Debug, Clone)]
pub enum NpcEvent {
    OpenDialog(DialogSpec),
    OpenShop(String),
    RestoreStats,
}

impl WorldState {
    fn try_interact_npc(&self, data: &GameData, facing: Direction) -> Option<NpcEvent> {
        let (target_x, target_y) = facing.apply(self.leader.x, self.leader.y);

        let npc = data.find_npc_at(&self.leader.current_map_id, target_x, target_y)?;

        match npc.npc_type {
            NpcType::Healer => {
                if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
                    let lines = self.filter_dialog_lines(dialog);
                    if !lines.is_empty() {
                        return Some(NpcEvent::OpenDialog(DialogSpec {
                            npc_name: npc.name.clone(),
                            lines,
                            restore: true,
                        }));
                    }
                }

                return Some(NpcEvent::RestoreStats);
            }
            NpcType::ShopKeeper => {
                let shop = npc
                    .shop_id
                    .as_ref()
                    .and_then(|sid| data.find_shop(sid))
                    .or_else(|| data.shops.first())
                    .cloned();

                if let Some(shop) = shop {
                    return Some(NpcEvent::OpenShop(shop.id));
                }
            }
            NpcType::QuestGiver | NpcType::Villager => {}
        }

        if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
            let lines = self.filter_dialog_lines(dialog);
            if !lines.is_empty() {
                return Some(NpcEvent::OpenDialog(DialogSpec {
                    npc_name: npc.name.clone(),
                    lines,
                    restore: false,
                }));
            }
        }

        None
    }

    fn filter_dialog_lines(&self, dialog: &Dialog) -> Vec<DialogLine> {
        dialog
            .lines
            .iter()
            .filter(|line| match &line.condition {
                None => true,
                Some(DialogCondition::HasQuest(id)) => self.has_quest(id),
                Some(DialogCondition::QuestComplete(id)) => self.is_quest_complete(id),
                Some(DialogCondition::HasItem(id)) => self.leader.has_item(id),
                Some(DialogCondition::HasGold(amount)) => self.leader.stats.gold >= *amount,
            })
            .cloned()
            .collect()
    }
}

struct NpcResolver;

static NPC_RESOLVER: NpcResolver = NpcResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&NPC_RESOLVER]
}

impl DomainEventResolver for NpcResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[GameEventKind::Explore]
    }

    fn resolve(
        &self,
        ctx: &ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::Explore(ExploreEvent::TryNpcInteract {
                facing,
                fallback_action,
            }) => {
                ensure!(
                    matches!(ctx.state, GameState::Explore),
                    "Invalid state: expected Explore"
                );
                let s = ctx.world.ok_or_else(|| anyhow!("No active world"))?;

                if let Some(npc_event) = s.try_interact_npc(ctx.data, *facing) {
                    out.push(GameEvent::Explore(ExploreEvent::Npc(npc_event)));
                    return Ok(());
                }

                let is_peaceful = ctx
                    .data
                    .find_map(&s.leader.current_map_id)
                    .is_some_and(|map| map.peaceful);
                if !is_peaceful && let Some(action) = fallback_action {
                    out.push(GameEvent::Explore(ExploreEvent::UseAction(*action)));
                }
            }
            GameEvent::Explore(ExploreEvent::Npc(npc_event)) => match npc_event {
                NpcEvent::OpenDialog(dialog_spec) => {
                    if dialog_spec.restore {
                        out.push(GameEvent::RestoreHpMp);
                    }
                    out.push(GameEvent::OpenDialogState(DialogState::new(
                        dialog_spec.npc_name.clone(),
                        dialog_spec.lines.clone(),
                    )));
                }
                NpcEvent::OpenShop(shop_id) => {
                    out.push(GameEvent::OpenShopById(shop_id.clone()));
                }
                NpcEvent::RestoreStats => {
                    out.push(GameEvent::RestoreHpMp);
                }
            },
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Npc, QuestProgress, Shop};

    fn make_session() -> WorldState {
        let mut session = WorldState::empty();
        session.leader = crate::game::CharacterState::new(String::from("H"), "v");
        session
    }

    fn make_npc(npc_type: NpcType) -> Npc {
        Npc {
            id: String::from("npc1"),
            name: String::from("NPC"),
            map_id: String::from("v"),
            x: 1,
            y: 0,
            npc_type,
            dialog_id: String::from("d1"),
            shop_id: Some(String::from("s1")),
        }
    }

    fn make_dialog(lines: Vec<&str>) -> Dialog {
        let mut raw = String::from("@DIALOG:d1\n");
        for line in lines {
            raw.push_str(line);
            raw.push('\n');
        }
        raw.push_str("@END\n");

        let Ok(dialogs) = crate::data::parse_dialogs(&raw) else {
            panic!("failed to parse dialog");
        };
        let Some(dialog) = dialogs.into_iter().next() else {
            panic!("dialog parse returned empty list");
        };
        dialog
    }

    fn make_game_data_with_npc(npc: Npc, dialog: Dialog) -> GameData {
        let mut data = GameData::default();
        data.npcs = vec![npc];
        data.dialogs = vec![dialog];
        data
    }

    fn make_shop(id: &str, items: Vec<String>) -> Shop {
        Shop {
            id: String::from(id),
            name: String::from("General Store"),
            items,
        }
    }

    #[test]
    fn try_interact_returns_none_when_no_npc_at_facing_position() {
        let session = make_session();
        let data = GameData::default();

        let next_state = session.try_interact_npc(&data, Direction::Right);

        assert!(next_state.is_none());
    }

    #[test]
    fn try_interact_with_villager_returns_dialog_state() {
        let session = make_session();
        let npc = make_npc(NpcType::Villager);
        let dialog = make_dialog(vec!["Hello"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = session.try_interact_npc(&data, Direction::Right);

        let Some(NpcEvent::OpenDialog(dialog_spec)) = next_state else {
            panic!("expected dialog state");
        };
        assert!(!dialog_spec.restore);
        assert_eq!(dialog_spec.npc_name, "NPC");
        assert_eq!(dialog_spec.lines[0].text, "Hello");
    }

    #[test]
    fn try_interact_with_healer_requests_restore_and_returns_dialog_state() {
        let session = make_session();
        let before_hp = session.leader.stats.current_hp;
        let before_mp = session.leader.stats.current_mp;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(vec!["Be healed"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = session.try_interact_npc(&data, Direction::Right);

        assert_eq!(session.leader.stats.current_hp, before_hp);
        assert_eq!(session.leader.stats.current_mp, before_mp);
        assert!(matches!(
            next_state,
            Some(NpcEvent::OpenDialog(DialogSpec { restore: true, .. }))
        ));
    }

    #[test]
    fn try_interact_with_healer_without_dialog_still_requests_restore() {
        let mut session = make_session();

        session.leader.stats.current_hp = 7;
        session.leader.stats.current_mp = 3;
        let before_hp = session.leader.stats.current_hp;
        let before_mp = session.leader.stats.current_mp;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(Vec::new());
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = session.try_interact_npc(&data, Direction::Right);

        assert_eq!(session.leader.stats.current_hp, before_hp);
        assert_eq!(session.leader.stats.current_mp, before_mp);
        assert!(matches!(next_state, Some(NpcEvent::RestoreStats)));
    }

    #[test]
    fn try_interact_with_shopkeeper_returns_shop_state() {
        let session = make_session();
        let npc = make_npc(NpcType::ShopKeeper);
        let dialog = make_dialog(vec!["Welcome"]);
        let mut data = make_game_data_with_npc(npc, dialog);
        data.shops = vec![make_shop("s1", Vec::new())];

        let next_state = session.try_interact_npc(&data, Direction::Right);

        let Some(NpcEvent::OpenShop(shop_id)) = next_state else {
            panic!("expected shop state");
        };
        assert_eq!(shop_id, "s1");
    }

    #[test]
    fn filter_lines_without_conditions_keeps_all_lines() {
        let session = make_session();
        let dialog = make_dialog(vec!["A", "B", "C"]);

        let filtered = session.filter_dialog_lines(&dialog);

        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filter_lines_has_quest_keeps_only_matching_lines() {
        let mut session = make_session();
        session.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 0,
            completed: false,
            rewarded: false,
        });
        let dialog = make_dialog(vec!["HAS_QUEST=q1:talk:q1", "HAS_QUEST=q2:talk:q2"]);

        let filtered = session.filter_dialog_lines(&dialog);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "q1");
    }

    #[test]
    fn filter_lines_quest_complete_condition_filters_correctly() {
        let mut session = make_session();
        session.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 1,
            completed: true,
            rewarded: false,
        });
        let dialog = make_dialog(vec![
            "QUEST_DONE=q1:talk:done",
            "QUEST_DONE=q2:talk:not done",
        ]);

        let filtered = session.filter_dialog_lines(&dialog);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "done");
    }

    #[test]
    fn filter_lines_has_gold_condition_filters_correctly() {
        let session = make_session();
        let dialog = make_dialog(vec![
            "HAS_GOLD=10:talk:cheap",
            "HAS_GOLD=100:talk:expensive",
        ]);

        let filtered = session.filter_dialog_lines(&dialog);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "cheap");
    }
}
