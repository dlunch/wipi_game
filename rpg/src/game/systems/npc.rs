use alloc::{rc::Rc, vec, vec::Vec};

use anyhow::{Result, anyhow};

use super::resolver::DomainEventResolver;
use crate::{
    data::{Dialog, DialogCondition, Direction, NpcType},
    game::{
        game_data::GameData,
        game_event::{ExploreEvent, GameEvent, GameEventKind},
        world::WorldState,
    },
};

#[derive(Debug)]
pub struct DialogSpec {
    npc_id: u32,
    dialog_id: u32,
    restore: bool,
}

#[derive(Debug)]
pub enum NpcEvent {
    OpenDialog(DialogSpec),
    OpenShop(u32),
    RestoreStats,
}

fn try_interact_npc(
    world: &WorldState,
    data: &GameData,
    facing: Direction,
) -> Result<Option<NpcEvent>> {
    let leader = world.leader_entity()?;
    let leader_id = world.leader_id()?;
    let (target_x, target_y) = facing.apply(leader.x, leader.y);

    let Some(npc) = data.find_npc_at(leader.map_id, target_x, target_y) else {
        return Ok(None);
    };

    match npc.npc_type {
        NpcType::Healer => {
            let dialog = data.find_dialog(npc.dialog_id)?;
            if has_visible_dialog_line(world, leader_id, dialog)? {
                return Ok(Some(NpcEvent::OpenDialog(DialogSpec {
                    npc_id: npc.id,
                    dialog_id: npc.dialog_id,
                    restore: true,
                })));
            }

            return Ok(Some(NpcEvent::RestoreStats));
        }
        NpcType::ShopKeeper => {
            let shop_id = npc
                .shop_id
                .as_ref()
                .ok_or_else(|| anyhow!("No shop id for NPC '{}'", npc.id))?;
            let shop = data.find_shop(*shop_id)?;
            return Ok(Some(NpcEvent::OpenShop(shop.id)));
        }
        NpcType::QuestGiver | NpcType::Villager => {}
    }

    let dialog = data.find_dialog(npc.dialog_id)?;
    if has_visible_dialog_line(world, leader_id, dialog)? {
        return Ok(Some(NpcEvent::OpenDialog(DialogSpec {
            npc_id: npc.id,
            dialog_id: npc.dialog_id,
            restore: false,
        })));
    }

    Ok(None)
}

fn has_visible_dialog_line(world: &WorldState, leader_id: u32, dialog: &Dialog) -> Result<bool> {
    for line in &dialog.lines {
        let visible = match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => world.has_quest(*id),
            Some(DialogCondition::QuestComplete(id)) => world.is_quest_complete(*id),
            Some(DialogCondition::HasItem(id)) => world.has_item(leader_id, *id)?,
            Some(DialogCondition::HasGold(amount)) => world.gold_amount(leader_id)? >= *amount,
        };
        if visible {
            return Ok(true);
        }
    }
    Ok(false)
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
        data: &Rc<GameData>,
        world: Option<&WorldState>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        match event {
            GameEvent::Explore(ExploreEvent::TryNpcInteract {
                facing,
                fallback_action,
            }) => {
                let world = world.ok_or_else(|| anyhow!("No active world"))?;
                let leader = world.leader_entity()?;

                if let Some(npc_event) = try_interact_npc(world, data, *facing)? {
                    out.push(GameEvent::Explore(ExploreEvent::Npc(npc_event)));
                    return Ok(());
                }

                let is_peaceful = data.find_map(leader.map_id)?.peaceful;
                if !is_peaceful && let Some(action) = fallback_action {
                    out.push(GameEvent::CombatPlayerAction(*action));
                }
            }
            GameEvent::Explore(ExploreEvent::Npc(npc_event)) => match npc_event {
                NpcEvent::OpenDialog(dialog_spec) => {
                    if dialog_spec.restore {
                        out.push(GameEvent::RestoreHpMp);
                    }
                    out.push(GameEvent::OpenDialog {
                        dialog_id: dialog_spec.dialog_id,
                        npc_id: dialog_spec.npc_id,
                    });
                }
                NpcEvent::OpenShop(shop_id) => {
                    out.push(GameEvent::OpenShopById(*shop_id));
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
