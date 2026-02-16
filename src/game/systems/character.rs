use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{DialogAction, Item, ItemKind, PlayerStats};
use crate::game::state::CharacterState;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{GameEvent, SessionEvent};

struct CharacterMutationResolver;

static CHARACTER_MUTATION_RESOLVER: CharacterMutationResolver = CharacterMutationResolver;

pub fn resolvers() -> Vec<&'static dyn DomainEventResolver> {
    vec![&CHARACTER_MUTATION_RESOLVER]
}

impl DomainEventResolver for CharacterMutationResolver {
    fn handles(&self, event: &GameEvent) -> bool {
        matches!(
            event,
            GameEvent::UseInventorySelected(_)
                | GameEvent::ShopBuyItem(_)
                | GameEvent::ShopSellSelected(_)
                | GameEvent::RestoreHpMp
                | GameEvent::ApplyDialogAction(_)
        )
    }

    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>> {
        let leader = &ctx
            .session
            .ok_or_else(|| anyhow!("No active session"))?
            .leader;

        match event {
            GameEvent::UseInventorySelected(index) => Ok(resolve_use_item(leader, *index)),
            GameEvent::ShopBuyItem(item) => Ok(resolve_shop_buy(leader, item)),
            GameEvent::ShopSellSelected(index) => Ok(resolve_shop_sell(leader, *index)),
            GameEvent::RestoreHpMp => Ok(resolve_restore_hp_mp(leader)),
            GameEvent::ApplyDialogAction(action) => resolve_dialog_action(ctx, leader, action),
            _ => Ok(Vec::new()),
        }
    }
}

fn resolve_use_item(character: &CharacterState, index: usize) -> Vec<GameEvent> {
    if index >= character.inventory.len() {
        return Vec::new();
    }

    let mut stats = character.stats.clone();
    let mut inventory = character.inventory.clone();
    let mut equipped_weapon = character.equipped_weapon;
    let mut equipped_armor = character.equipped_armor;
    let mut equipped_accessory = character.equipped_accessory;

    let item = &inventory[index];
    match item.kind {
        ItemKind::Consumable => {
            stats.heal(item.hp_restore());
            inventory.remove(index);
            fix_equipped_after_remove(&mut equipped_weapon, index);
            fix_equipped_after_remove(&mut equipped_armor, index);
            fix_equipped_after_remove(&mut equipped_accessory, index);
        }
        ItemKind::Weapon => {
            equipped_weapon = Some(index);
        }
        ItemKind::Armor => {
            equipped_armor = Some(index);
        }
        ItemKind::Accessory => {
            equipped_accessory = Some(index);
        }
    }

    emit_character_events(
        stats,
        inventory,
        equipped_weapon,
        equipped_armor,
        equipped_accessory,
    )
}

fn resolve_shop_buy(character: &CharacterState, item: &Item) -> Vec<GameEvent> {
    let mut stats = character.stats.clone();
    stats.gold = (stats.gold - item.price).max(0);

    let mut inventory = character.inventory.clone();
    inventory.push(item.clone());

    emit_character_events(
        stats,
        inventory,
        character.equipped_weapon,
        character.equipped_armor,
        character.equipped_accessory,
    )
}

fn resolve_shop_sell(character: &CharacterState, index: usize) -> Vec<GameEvent> {
    if index >= character.inventory.len() {
        return Vec::new();
    }

    let mut stats = character.stats.clone();
    let mut inventory = character.inventory.clone();
    let sold = inventory.remove(index);
    stats.gold += sold.price / 2;

    let mut equipped_weapon = character.equipped_weapon;
    let mut equipped_armor = character.equipped_armor;
    let mut equipped_accessory = character.equipped_accessory;
    fix_equipped_after_remove(&mut equipped_weapon, index);
    fix_equipped_after_remove(&mut equipped_armor, index);
    fix_equipped_after_remove(&mut equipped_accessory, index);

    emit_character_events(
        stats,
        inventory,
        equipped_weapon,
        equipped_armor,
        equipped_accessory,
    )
}

fn resolve_dialog_action(
    ctx: &ResolveContext<'_>,
    character: &CharacterState,
    action: &DialogAction,
) -> Result<Vec<GameEvent>> {
    let mut stats = character.stats.clone();
    let mut inventory = character.inventory.clone();
    let mut changed = false;

    match action {
        DialogAction::GiveItem(id) => {
            if let Some(item) = ctx.data().find_item(id).cloned() {
                inventory.push(item);
                changed = true;
            }
        }
        DialogAction::TakeItem(id) => {
            if let Some(index) = inventory.iter().position(|item| item.id == *id) {
                inventory.remove(index);
                changed = true;
            }
        }
        DialogAction::GiveGold(amount) => {
            stats.gold = (stats.gold + *amount).max(0);
            changed = true;
        }
        DialogAction::TakeGold(amount) => {
            stats.gold = (stats.gold - *amount).max(0);
            changed = true;
        }
        DialogAction::Heal => {
            stats.current_hp = stats.max_hp;
            stats.current_mp = stats.max_mp;
            changed = true;
        }
        DialogAction::GiveQuest(_) | DialogAction::CompleteQuest(_) | DialogAction::OpenShop(_) => {
        }
    }

    if !changed {
        return Ok(Vec::new());
    }

    Ok(emit_character_events(
        stats,
        inventory,
        character.equipped_weapon,
        character.equipped_armor,
        character.equipped_accessory,
    ))
}

fn resolve_restore_hp_mp(character: &CharacterState) -> Vec<GameEvent> {
    let mut stats = character.stats.clone();
    stats.current_hp = stats.max_hp;
    stats.current_mp = stats.max_mp;
    vec![GameEvent::Session(SessionEvent::SetPlayerStats(stats))]
}

fn emit_character_events(
    stats: PlayerStats,
    inventory: Vec<Item>,
    equipped_weapon: Option<usize>,
    equipped_armor: Option<usize>,
    equipped_accessory: Option<usize>,
) -> Vec<GameEvent> {
    vec![
        GameEvent::Session(SessionEvent::SetPlayerStats(stats)),
        GameEvent::Session(SessionEvent::SetPlayerInventory(inventory)),
        GameEvent::Session(SessionEvent::SetEquippedWeapon(equipped_weapon)),
        GameEvent::Session(SessionEvent::SetEquippedArmor(equipped_armor)),
        GameEvent::Session(SessionEvent::SetEquippedAccessory(equipped_accessory)),
    ]
}

fn fix_equipped_after_remove(equipped: &mut Option<usize>, removed_index: usize) {
    if let Some(index) = equipped {
        if *index > removed_index {
            *index -= 1;
        } else if *index == removed_index {
            *equipped = None;
        }
    }
}
