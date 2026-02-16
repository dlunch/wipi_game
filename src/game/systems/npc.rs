use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow, ensure};

use crate::data::{Dialog, DialogCondition, DialogLine, Direction, NpcType};

use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{AppExploreEvent, CharacterState, DialogState, GameData, GameEvent, GameState};

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

#[derive(Debug)]
pub enum NpcIntent {
    Interact { facing: Direction },
}

pub fn resolve(player: &CharacterState, data: &GameData, intent: NpcIntent) -> Option<NpcEvent> {
    match intent {
        NpcIntent::Interact { facing } => try_interact(player, data, facing),
    }
}

fn try_interact(player: &CharacterState, data: &GameData, facing: Direction) -> Option<NpcEvent> {
    let (target_x, target_y) = facing.apply(player.x, player.y);

    let npc = data.find_npc_at(&player.current_map_id, target_x, target_y)?;

    match npc.npc_type {
        NpcType::Healer => {
            if let Some(dialog) = data.find_dialog(&npc.dialog_id) {
                let lines = filter_lines(player, dialog);
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
        let lines = filter_lines(player, dialog);
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

fn filter_lines(player: &CharacterState, dialog: &Dialog) -> Vec<DialogLine> {
    dialog
        .lines
        .iter()
        .filter(|line| match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => player.has_quest(id),
            Some(DialogCondition::QuestComplete(id)) => player.is_quest_complete(id),
            Some(DialogCondition::HasItem(id)) => player.has_item(id),
            Some(DialogCondition::HasGold(amount)) => player.stats.gold >= *amount,
        })
        .cloned()
        .collect()
}

struct ExploreNpcCascadeResolver;
struct ExploreNpcInteractResolver;

static EXPLORE_NPC_CASCADE_RESOLVER: ExploreNpcCascadeResolver = ExploreNpcCascadeResolver;
static EXPLORE_NPC_INTERACT_RESOLVER: ExploreNpcInteractResolver = ExploreNpcInteractResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![
        &EXPLORE_NPC_INTERACT_RESOLVER,
        &EXPLORE_NPC_CASCADE_RESOLVER,
    ]
}

impl DomainEventResolver for ExploreNpcInteractResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Explore(AppExploreEvent::TryNpcInteract { .. })
        )
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::Explore(AppExploreEvent::TryNpcInteract {
            facing,
            fallback_action,
        }) = event
        else {
            return Err(anyhow!("Invalid event: expected Explore(TryNpcInteract)"));
        };
        ensure!(
            matches!(ctx.state, GameState::Explore),
            "Invalid state: expected Explore"
        );
        let s = ctx.session.ok_or_else(|| anyhow!("No active session"))?;

        if let Some(npc_event) = resolve(
            &s.leader,
            ctx.data(),
            NpcIntent::Interact { facing: *facing },
        ) {
            return Ok(vec![GameEvent::Explore(AppExploreEvent::Npc(npc_event))]);
        }

        if let Some(action) = fallback_action {
            return Ok(vec![GameEvent::Explore(AppExploreEvent::UseAction(
                *action,
            ))]);
        }

        Ok(Vec::new())
    }
}

impl DomainEventResolver for ExploreNpcCascadeResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(event, GameEvent::Explore(AppExploreEvent::Npc(_)))
    }

    fn resolve(&self, _ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let GameEvent::Explore(AppExploreEvent::Npc(npc_event)) = event else {
            return Err(anyhow!("Invalid event: expected Explore(Npc)"));
        };
        match npc_event {
            NpcEvent::OpenDialog(dialog_spec) => {
                let mut events = Vec::with_capacity(2);
                if dialog_spec.restore {
                    events.push(GameEvent::RestoreSessionStats);
                }
                events.push(GameEvent::OpenDialogState(DialogState::new(
                    dialog_spec.npc_name.clone(),
                    dialog_spec.lines.clone(),
                )));
                Ok(events)
            }
            NpcEvent::OpenShop(shop_id) => Ok(vec![GameEvent::OpenShopById(shop_id.clone())]),
            NpcEvent::RestoreStats => Ok(vec![GameEvent::RestoreSessionStats]),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Npc, QuestProgress, Shop};

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
        GameData {
            npcs: vec![npc],
            dialogs: vec![dialog],
            ..GameData::default()
        }
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
        let player = CharacterState::new(String::from("H"), "v");
        let data = GameData::default();

        let next_state = try_interact(&player, &data, Direction::Right);

        assert!(next_state.is_none());
    }

    #[test]
    fn try_interact_with_villager_returns_dialog_state() {
        let player = CharacterState::new(String::from("H"), "v");
        let npc = make_npc(NpcType::Villager);
        let dialog = make_dialog(vec!["Hello"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&player, &data, Direction::Right);

        let Some(NpcEvent::OpenDialog(dialog_spec)) = next_state else {
            panic!("expected dialog state");
        };
        assert!(!dialog_spec.restore);
        assert_eq!(dialog_spec.npc_name, "NPC");
        assert_eq!(dialog_spec.lines[0].text, "Hello");
    }

    #[test]
    fn try_interact_with_healer_requests_restore_and_returns_dialog_state() {
        let player = CharacterState::new(String::from("H"), "v");
        let before_hp = player.stats.current_hp;
        let before_mp = player.stats.current_mp;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(vec!["Be healed"]);
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&player, &data, Direction::Right);

        assert_eq!(player.stats.current_hp, before_hp);
        assert_eq!(player.stats.current_mp, before_mp);
        assert!(matches!(
            next_state,
            Some(NpcEvent::OpenDialog(DialogSpec { restore: true, .. }))
        ));
    }

    #[test]
    fn try_interact_with_healer_without_dialog_still_requests_restore() {
        let mut player = CharacterState::new(String::from("H"), "v");

        player.stats.current_hp = 7;
        player.stats.current_mp = 3;
        let before_hp = player.stats.current_hp;
        let before_mp = player.stats.current_mp;

        let npc = make_npc(NpcType::Healer);
        let dialog = make_dialog(Vec::new());
        let data = make_game_data_with_npc(npc, dialog);

        let next_state = try_interact(&player, &data, Direction::Right);

        assert_eq!(player.stats.current_hp, before_hp);
        assert_eq!(player.stats.current_mp, before_mp);
        assert!(matches!(next_state, Some(NpcEvent::RestoreStats)));
    }

    #[test]
    fn try_interact_with_shopkeeper_returns_shop_state() {
        let player = CharacterState::new(String::from("H"), "v");
        let npc = make_npc(NpcType::ShopKeeper);
        let dialog = make_dialog(vec!["Welcome"]);
        let mut data = make_game_data_with_npc(npc, dialog);
        data.shops = vec![make_shop("s1", Vec::new())];

        let next_state = try_interact(&player, &data, Direction::Right);

        let Some(NpcEvent::OpenShop(shop_id)) = next_state else {
            panic!("expected shop state");
        };
        assert_eq!(shop_id, "s1");
    }

    #[test]
    fn filter_lines_without_conditions_keeps_all_lines() {
        let player = CharacterState::new(String::from("H"), "v");
        let dialog = make_dialog(vec!["A", "B", "C"]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filter_lines_has_quest_keeps_only_matching_lines() {
        let mut player = CharacterState::new(String::from("H"), "v");
        player.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 0,
            completed: false,
            rewarded: false,
        });
        let dialog = make_dialog(vec!["HAS_QUEST=q1:talk:q1", "HAS_QUEST=q2:talk:q2"]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "q1");
    }

    #[test]
    fn filter_lines_quest_complete_condition_filters_correctly() {
        let mut player = CharacterState::new(String::from("H"), "v");
        player.quests.push(QuestProgress {
            quest_id: String::from("q1"),
            current_count: 1,
            completed: true,
            rewarded: false,
        });
        let dialog = make_dialog(vec![
            "QUEST_DONE=q1:talk:done",
            "QUEST_DONE=q2:talk:not done",
        ]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "done");
    }

    #[test]
    fn filter_lines_has_gold_condition_filters_correctly() {
        let player = CharacterState::new(String::from("H"), "v");
        let dialog = make_dialog(vec![
            "HAS_GOLD=10:talk:cheap",
            "HAS_GOLD=100:talk:expensive",
        ]);

        let filtered = filter_lines(&player, &dialog);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "cheap");
    }
}
