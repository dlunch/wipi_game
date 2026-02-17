use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{DialogAction, QuestProgress, QuestType};
use crate::game::systems::resolver::{DomainEventResolver, ResolveContext};
use crate::game::{
    CombatEvent, GameEvent, GameEventKind, MovementEvent, TransitionEvent, WorldEvent,
};

struct SessionLogicResolver;

static SESSION_LOGIC_RESOLVER: SessionLogicResolver = SessionLogicResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&SESSION_LOGIC_RESOLVER]
}

impl DomainEventResolver for SessionLogicResolver {
    fn subscribed_kinds(&self) -> &'static [GameEventKind] {
        &[
            GameEventKind::Movement,
            GameEventKind::ApplyDialogAction,
            GameEventKind::Combat,
        ]
    }

    fn resolve(
        &self,
        ctx: &mut ResolveContext<'_>,
        event: &GameEvent,
        out: &mut Vec<GameEvent>,
    ) -> Result<()> {
        let session = ctx.world.ok_or_else(|| anyhow!("No active world"))?;
        match event {
            GameEvent::Movement(MovementEvent::Tick(movement, Some(tile_event))) => {
                resolve_tile_event(ctx, session, movement.step, tile_event, out);
            }
            GameEvent::ApplyDialogAction(DialogAction::GiveQuest(id)) => {
                resolve_give_quest(session, id, out);
            }
            GameEvent::ApplyDialogAction(DialogAction::CompleteQuest(id)) => {
                resolve_complete_quest(ctx, session, id, out);
            }
            GameEvent::Combat(CombatEvent::RecoverMp(recover_mp)) => {
                resolve_recover_mp(session, *recover_mp, out);
            }
            GameEvent::Combat(CombatEvent::Heal(heal)) => resolve_heal(session, *heal, out),
            GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id,
                exp,
                gold,
            }) => resolve_kill_reward(ctx, session, enemy_id, *exp, *gold, out),
            GameEvent::Combat(CombatEvent::TakeDamage(damage)) => {
                resolve_take_damage(session, *damage, out);
            }
            _ => {}
        }
        Ok(())
    }
}

fn resolve_tile_event(
    ctx: &ResolveContext<'_>,
    session: &crate::game::WorldState,
    step: Option<(i32, i32)>,
    tile_event: &crate::game::TileEvent,
    out: &mut Vec<GameEvent>,
) {
    let (next_x, next_y) = if let Some((dx, dy)) = step {
        (
            (session.leader.x as i32 + dx) as usize,
            (session.leader.y as i32 + dy) as usize,
        )
    } else {
        (session.leader.x, session.leader.y)
    };

    match tile_event {
        crate::game::TileEvent::Treasure => {
            let map_id = session.leader.current_map_id.clone();
            if session.is_treasure_opened(&map_id, next_x, next_y) {
                return;
            }
            out.push(GameEvent::World(WorldEvent::AddOpenedTreasure {
                map_id,
                x: next_x,
                y: next_y,
            }));
            if let Some(item_id) = ctx.data().newgame.treasure_item.as_deref()
                && let Some(item) = ctx.data().find_item(item_id).cloned()
            {
                out.push(GameEvent::World(WorldEvent::AddPlayerItem(item)));
            }
        }
        crate::game::TileEvent::MapExit(target)
        | crate::game::TileEvent::DungeonEntrance(target) => {
            if target.is_empty() {
                return;
            }
            let Some(map) = ctx.data().find_map(target) else {
                return;
            };
            let (x, y) = map.find_player_start().unwrap_or((next_x, next_y));
            out.push(GameEvent::World(WorldEvent::SetPlayerMap(map.id.clone())));
            out.push(GameEvent::World(WorldEvent::SetPlayerPosition { x, y }));
            out.push(GameEvent::Transition(TransitionEvent::MapChanged));
        }
    }
}

fn resolve_give_quest(session: &crate::game::WorldState, id: &str, out: &mut Vec<GameEvent>) {
    if session.quests.iter().any(|quest| quest.quest_id == id) {
        return;
    }
    out.push(GameEvent::World(WorldEvent::AddQuestProgress(
        QuestProgress {
            quest_id: id.into(),
            current_count: 0,
            completed: false,
            rewarded: false,
        },
    )));
}

fn resolve_complete_quest(
    ctx: &ResolveContext<'_>,
    session: &crate::game::WorldState,
    id: &str,
    out: &mut Vec<GameEvent>,
) {
    let can_reward = session
        .quests
        .iter()
        .any(|quest| quest.quest_id == id && quest.completed && !quest.rewarded);
    if !can_reward {
        return;
    }

    let Some(quest) = ctx.data().find_quest(id) else {
        return;
    };

    let mut stats = session.leader.stats.clone();
    stats.add_exp(quest.reward_exp);
    stats.gold = (stats.gold + quest.reward_gold).max(0);

    out.push(GameEvent::World(WorldEvent::SetPlayerStats(stats)));
    if let Some(item_id) = &quest.reward_item
        && let Some(item) = ctx.data().find_item(item_id).cloned()
    {
        out.push(GameEvent::World(WorldEvent::AddPlayerItem(item)));
    }

    if let Some(mut progress) = session.quests.iter().find(|q| q.quest_id == id).cloned() {
        progress.rewarded = true;
        out.push(GameEvent::World(WorldEvent::AddQuestProgress(progress)));
    }
}

fn resolve_recover_mp(
    session: &crate::game::WorldState,
    recover_mp: i32,
    out: &mut Vec<GameEvent>,
) {
    let mut stats = session.leader.stats.clone();
    if recover_mp > 0 {
        stats.recover_mp(recover_mp);
    } else if recover_mp < 0 {
        stats.current_mp = (stats.current_mp + recover_mp).max(0);
    }
    out.push(GameEvent::World(WorldEvent::SetPlayerStats(stats)));
}

fn resolve_heal(session: &crate::game::WorldState, heal: i32, out: &mut Vec<GameEvent>) {
    if heal <= 0 {
        return;
    }
    let mut stats = session.leader.stats.clone();
    stats.heal(heal);
    out.push(GameEvent::World(WorldEvent::SetPlayerStats(stats)));
}

fn resolve_kill_reward(
    ctx: &ResolveContext<'_>,
    session: &crate::game::WorldState,
    enemy_id: &str,
    exp: i32,
    gold: i32,
    out: &mut Vec<GameEvent>,
) {
    let mut stats = session.leader.stats.clone();
    stats.add_exp(exp);
    stats.gold = (stats.gold + gold).max(0);

    out.push(GameEvent::World(WorldEvent::SetPlayerStats(stats)));
    for progress in &session.quests {
        if progress.completed || progress.rewarded {
            continue;
        }
        if let Some(quest) = ctx.data().find_quest(&progress.quest_id)
            && quest.quest_type == QuestType::Kill
            && quest.target_id == enemy_id
        {
            let mut next = progress.clone();
            next.current_count = (next.current_count + 1).min(quest.target_count);
            if next.current_count >= quest.target_count {
                next.completed = true;
            }
            out.push(GameEvent::World(WorldEvent::AddQuestProgress(next)));
        }
    }
}

fn resolve_take_damage(session: &crate::game::WorldState, damage: i32, out: &mut Vec<GameEvent>) {
    if damage <= 0 {
        return;
    }
    let mut stats = session.leader.stats.clone();
    stats.take_damage(damage);
    out.push(GameEvent::World(WorldEvent::SetPlayerStats(stats.clone())));
    if stats.is_dead() {
        out.push(GameEvent::Transition(TransitionEvent::ToGameOver));
    }
}
