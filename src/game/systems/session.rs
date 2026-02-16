use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{DialogAction, QuestProgress, QuestType};
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{CombatEvent, GameEvent, MovementEvent, SessionEvent, TransitionEvent};

struct SessionLogicResolver;

static SESSION_LOGIC_RESOLVER: SessionLogicResolver = SessionLogicResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&SESSION_LOGIC_RESOLVER]
}

impl DomainEventResolver for SessionLogicResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::Movement(MovementEvent::Tick(_, Some(_)))
                | GameEvent::ApplyDialogAction(DialogAction::GiveQuest(_))
                | GameEvent::ApplyDialogAction(DialogAction::CompleteQuest(_))
                | GameEvent::Combat(CombatEvent::RecoverMp(_))
                | GameEvent::Combat(CombatEvent::Heal(_))
                | GameEvent::Combat(CombatEvent::GrantKillReward { .. })
                | GameEvent::Combat(CombatEvent::TakeDamage(_))
        )
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let session = ctx.session.ok_or_else(|| anyhow!("No active session"))?;
        match event {
            GameEvent::Movement(MovementEvent::Tick(movement, Some(tile_event))) => {
                Ok(resolve_tile_event(ctx, session, movement.step, tile_event))
            }
            GameEvent::ApplyDialogAction(DialogAction::GiveQuest(id)) => {
                Ok(resolve_give_quest(session, id))
            }
            GameEvent::ApplyDialogAction(DialogAction::CompleteQuest(id)) => {
                Ok(resolve_complete_quest(ctx, session, id))
            }
            GameEvent::Combat(CombatEvent::RecoverMp(recover_mp)) => {
                Ok(resolve_recover_mp(session, *recover_mp))
            }
            GameEvent::Combat(CombatEvent::Heal(heal)) => Ok(resolve_heal(session, *heal)),
            GameEvent::Combat(CombatEvent::GrantKillReward {
                enemy_id,
                exp,
                gold,
            }) => Ok(resolve_kill_reward(ctx, session, enemy_id, *exp, *gold)),
            GameEvent::Combat(CombatEvent::TakeDamage(damage)) => {
                Ok(resolve_take_damage(session, *damage))
            }
            _ => Ok(Vec::new()),
        }
    }
}

fn resolve_tile_event(
    ctx: &ResolveContext<'_>,
    session: &crate::game::SessionState,
    step: Option<(i32, i32)>,
    tile_event: &crate::game::TileEvent,
) -> Vec<GameEvent> {
    let (next_x, next_y) = if let Some((dx, dy)) = step {
        (
            session
                .leader
                .x
                .checked_add_signed(dx as isize)
                .unwrap_or(session.leader.x),
            session
                .leader
                .y
                .checked_add_signed(dy as isize)
                .unwrap_or(session.leader.y),
        )
    } else {
        (session.leader.x, session.leader.y)
    };

    match tile_event {
        crate::game::TileEvent::Treasure => {
            let map_id = session.leader.current_map_id.clone();
            if session.is_treasure_opened(&map_id, next_x, next_y) {
                return Vec::new();
            }
            let mut out = vec![GameEvent::Session(SessionEvent::AddOpenedTreasure {
                map_id,
                x: next_x,
                y: next_y,
            })];
            if let Some(item_id) = ctx.data().newgame.treasure_item.as_deref()
                && let Some(item) = ctx.data().find_item(item_id).cloned()
            {
                out.push(GameEvent::Session(SessionEvent::AddPlayerItem(item)));
            }
            out
        }
        crate::game::TileEvent::MapExit(target)
        | crate::game::TileEvent::DungeonEntrance(target) => {
            if target.is_empty() {
                return Vec::new();
            }
            let Some(map) = ctx.data().find_map(target) else {
                return Vec::new();
            };
            let (x, y) = map.find_player_start().unwrap_or((next_x, next_y));
            vec![
                GameEvent::Session(SessionEvent::SetPlayerMap(map.id.clone())),
                GameEvent::Session(SessionEvent::SetPlayerPosition { x, y }),
            ]
        }
    }
}

fn resolve_give_quest(session: &crate::game::SessionState, id: &str) -> Vec<GameEvent> {
    if session.quests.iter().any(|quest| quest.quest_id == id) {
        return Vec::new();
    }
    vec![GameEvent::Session(SessionEvent::AddQuestProgress(
        QuestProgress {
            quest_id: id.into(),
            current_count: 0,
            completed: false,
            rewarded: false,
        },
    ))]
}

fn resolve_complete_quest(
    ctx: &ResolveContext<'_>,
    session: &crate::game::SessionState,
    id: &str,
) -> Vec<GameEvent> {
    let can_reward = session
        .quests
        .iter()
        .any(|quest| quest.quest_id == id && quest.completed && !quest.rewarded);
    if !can_reward {
        return Vec::new();
    }

    let Some(quest) = ctx.data().find_quest(id) else {
        return Vec::new();
    };

    let mut stats = session.leader.stats.clone();
    stats.add_exp(quest.reward_exp);
    stats.gold = (stats.gold + quest.reward_gold).max(0);

    let mut out = vec![GameEvent::Session(SessionEvent::SetPlayerStats(stats))];
    if let Some(item_id) = &quest.reward_item
        && let Some(item) = ctx.data().find_item(item_id).cloned()
    {
        out.push(GameEvent::Session(SessionEvent::AddPlayerItem(item)));
    }

    if let Some(mut progress) = session.quests.iter().find(|q| q.quest_id == id).cloned() {
        progress.rewarded = true;
        out.push(GameEvent::Session(SessionEvent::AddQuestProgress(progress)));
    }
    out
}

fn resolve_recover_mp(session: &crate::game::SessionState, recover_mp: i32) -> Vec<GameEvent> {
    let mut stats = session.leader.stats.clone();
    if recover_mp > 0 {
        stats.recover_mp(recover_mp);
    } else if recover_mp < 0 {
        stats.current_mp = (stats.current_mp + recover_mp).max(0);
    }
    vec![GameEvent::Session(SessionEvent::SetPlayerStats(stats))]
}

fn resolve_heal(session: &crate::game::SessionState, heal: i32) -> Vec<GameEvent> {
    if heal <= 0 {
        return Vec::new();
    }
    let mut stats = session.leader.stats.clone();
    stats.heal(heal);
    vec![GameEvent::Session(SessionEvent::SetPlayerStats(stats))]
}

fn resolve_kill_reward(
    ctx: &ResolveContext<'_>,
    session: &crate::game::SessionState,
    enemy_id: &str,
    exp: i32,
    gold: i32,
) -> Vec<GameEvent> {
    let mut stats = session.leader.stats.clone();
    stats.add_exp(exp);
    stats.gold = (stats.gold + gold).max(0);

    let mut out = vec![GameEvent::Session(SessionEvent::SetPlayerStats(stats))];
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
            out.push(GameEvent::Session(SessionEvent::AddQuestProgress(next)));
        }
    }
    out
}

fn resolve_take_damage(session: &crate::game::SessionState, damage: i32) -> Vec<GameEvent> {
    if damage <= 0 {
        return Vec::new();
    }
    let mut stats = session.leader.stats.clone();
    stats.take_damage(damage);
    let mut out = vec![GameEvent::Session(SessionEvent::SetPlayerStats(
        stats.clone(),
    ))];
    if stats.is_dead() {
        out.push(GameEvent::Transition(TransitionEvent::ToGameOver));
    }
    out
}
