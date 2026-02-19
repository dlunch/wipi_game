use alloc::{rc::Rc, string::String, vec, vec::Vec};

use anyhow::{Result, anyhow};

use super::resolver::DomainEventResolver;
use crate::{
    data::{Dialog, DialogCondition, DialogLine, Direction, NpcType},
    game::{
        game_data::GameData,
        game_event::{ExploreEvent, GameEvent, GameEventKind},
        ui::state::DialogState,
        world::WorldState,
    },
};

#[derive(Debug)]
pub struct DialogSpec {
    pub npc_name: String,
    pub lines: Vec<DialogLine>,
    pub restore: bool,
}

#[derive(Debug)]
pub enum NpcEvent {
    OpenDialog(DialogSpec),
    OpenShop(String),
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

    let Some(npc) = data.find_npc_at(&leader.map_id, target_x, target_y) else {
        return Ok(None);
    };

    match npc.npc_type {
        NpcType::Healer => {
            let dialog = data.find_dialog(&npc.dialog_id)?;
            let lines = filter_dialog_lines(world, leader_id, dialog)?;
            if !lines.is_empty() {
                return Ok(Some(NpcEvent::OpenDialog(DialogSpec {
                    npc_name: npc.name.clone(),
                    lines,
                    restore: true,
                })));
            }

            return Ok(Some(NpcEvent::RestoreStats));
        }
        NpcType::ShopKeeper => {
            let shop = npc
                .shop_id
                .as_ref()
                .map(|sid| data.find_shop(sid))
                .transpose()?
                .or_else(|| data.shops.first())
                .ok_or_else(|| anyhow!("No shop available for NPC '{}'", npc.id))?;
            return Ok(Some(NpcEvent::OpenShop(shop.id.clone())));
        }
        NpcType::QuestGiver | NpcType::Villager => {}
    }

    let dialog = data.find_dialog(&npc.dialog_id)?;
    let lines = filter_dialog_lines(world, leader_id, dialog)?;
    if !lines.is_empty() {
        return Ok(Some(NpcEvent::OpenDialog(DialogSpec {
            npc_name: npc.name.clone(),
            lines,
            restore: false,
        })));
    }

    Ok(None)
}

fn filter_dialog_lines(
    world: &WorldState,
    leader_id: u32,
    dialog: &Dialog,
) -> Result<Vec<DialogLine>> {
    let mut out = Vec::with_capacity(dialog.lines.len());
    for line in &dialog.lines {
        let include = match &line.condition {
            None => true,
            Some(DialogCondition::HasQuest(id)) => world.has_quest(id),
            Some(DialogCondition::QuestComplete(id)) => world.is_quest_complete(id),
            Some(DialogCondition::HasItem(id)) => world.has_item(leader_id, id)?,
            Some(DialogCondition::HasGold(amount)) => world.gold_amount(leader_id)? >= *amount,
        };
        if include {
            out.push(line.clone());
        }
    }
    Ok(out)
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

                let is_peaceful = data.find_map(&leader.map_id)?.peaceful;
                if !is_peaceful && let Some(action) = fallback_action {
                    out.push(GameEvent::CombatPlayerAction(*action));
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
