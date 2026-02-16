use crate::data::Direction;
use anyhow::Result;

use crate::game::ExploreAction;
use crate::game::systems::runtime::{DomainEventResolver, ResolveContext};
use crate::game::{AppExploreEvent, RuntimeEvent};

#[derive(Debug, Clone, Copy)]
pub enum ExploreIntent {
    MoveDirection(Direction),
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<ExploreAction>,
    },
    UseAction(ExploreAction),
    Pause,
    BackToMenu,
}

pub enum ExploreEvent {
    None,
    MoveDirection(Direction),
    TryNpcInteract {
        facing: Direction,
        fallback_action: Option<ExploreAction>,
    },
    UseAction(ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

pub fn resolve(is_peaceful: bool, intent: ExploreIntent) -> ExploreEvent {
    match intent {
        ExploreIntent::MoveDirection(direction) => ExploreEvent::MoveDirection(direction),
        ExploreIntent::TryNpcInteract {
            facing,
            fallback_action,
        } => ExploreEvent::TryNpcInteract {
            facing,
            fallback_action,
        },
        ExploreIntent::UseAction(action) if !is_peaceful => ExploreEvent::UseAction(action),
        ExploreIntent::UseAction(_) => ExploreEvent::None,
        ExploreIntent::Pause => ExploreEvent::EnterPauseMenu,
        ExploreIntent::BackToMenu => ExploreEvent::EnterMenu,
    }
}

pub fn resolve_many(is_peaceful: bool, intent: ExploreIntent) -> alloc::vec::Vec<ExploreEvent> {
    match resolve(is_peaceful, intent) {
        ExploreEvent::None => alloc::vec::Vec::new(),
        event => alloc::vec![event],
    }
}

struct ExploreUseActionCascadeResolver;
struct ExplorePauseCascadeResolver;
struct ExploreMenuCascadeResolver;

static EXPLORE_USE_ACTION_CASCADE_RESOLVER: ExploreUseActionCascadeResolver =
    ExploreUseActionCascadeResolver;
static EXPLORE_PAUSE_CASCADE_RESOLVER: ExplorePauseCascadeResolver = ExplorePauseCascadeResolver;
static EXPLORE_MENU_CASCADE_RESOLVER: ExploreMenuCascadeResolver = ExploreMenuCascadeResolver;

pub fn resolvers() -> alloc::vec::Vec<&'static dyn DomainEventResolver> {
    alloc::vec![
        &EXPLORE_USE_ACTION_CASCADE_RESOLVER,
        &EXPLORE_PAUSE_CASCADE_RESOLVER,
        &EXPLORE_MENU_CASCADE_RESOLVER,
    ]
}

impl DomainEventResolver for ExploreUseActionCascadeResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Explore(AppExploreEvent::UseAction(_)))
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        let RuntimeEvent::Explore(AppExploreEvent::UseAction(action)) = event else {
            return Ok(alloc::vec::Vec::new());
        };
        Ok(alloc::vec![RuntimeEvent::CombatPlayerAction(*action)])
    }
}

impl DomainEventResolver for ExplorePauseCascadeResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Explore(AppExploreEvent::EnterPauseMenu)
        )
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        Ok(alloc::vec![RuntimeEvent::OpenPauseMenu])
    }
}

impl DomainEventResolver for ExploreMenuCascadeResolver {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(event, RuntimeEvent::Explore(AppExploreEvent::EnterMenu))
    }

    fn resolve(
        &self,
        _ctx: &mut ResolveContext<'_>,
        _event: &RuntimeEvent,
    ) -> Result<alloc::vec::Vec<RuntimeEvent>> {
        Ok(alloc::vec![RuntimeEvent::OpenMenuFromExplore])
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::data::{Item, ItemKind, Map, Tile};
    use crate::game::TileEvent;
    use crate::game::state::TileApplyEvent;
    use crate::game::{GameData, PlayerState};

    fn make_test_map(
        id: &str,
        width: usize,
        height: usize,
        tiles: Vec<Tile>,
        exits: Vec<(usize, usize, String)>,
        dungeons: Vec<(usize, usize, String)>,
        encounters: Vec<(String, i32)>,
        peaceful: bool,
    ) -> Map {
        Map {
            id: id.into(),
            name: id.into(),
            width,
            height,
            tiles,
            encounters,
            exits,
            dungeons,
            npcs: Vec::new(),
            peaceful,
        }
    }

    fn make_game_data() -> GameData {
        let mut data = GameData::default();

        data.items.push(Item {
            id: "potion".into(),
            name: "Potion".into(),
            kind: ItemKind::Consumable,
            param1: 30,
            param2: 0,
            param3: 0,
            price: 50,
        });

        data.maps.push(make_test_map(
            "field",
            3,
            3,
            vec![
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Treasure,
                Tile::Exit,
                Tile::Floor,
                Tile::Dungeon,
                Tile::Floor,
            ],
            vec![(2, 1, "town".into())],
            vec![(1, 2, "cave".into())],
            Vec::new(),
            false,
        ));

        data.maps.push(make_test_map(
            "town",
            3,
            3,
            vec![
                Tile::PlayerStart,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        ));

        data.maps.push(make_test_map(
            "cave",
            3,
            3,
            vec![
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::Floor,
                Tile::PlayerStart,
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        ));

        data.maps.push(make_test_map(
            "safe_zone",
            2,
            2,
            vec![Tile::PlayerStart, Tile::Floor, Tile::Floor, Tile::Floor],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        ));

        data
    }

    fn make_player(map_id: &str, x: usize, y: usize) -> PlayerState {
        let mut player = PlayerState::new(String::from("Tester"), map_id);
        player.x = x;
        player.y = y;
        player
    }

    #[test]
    fn apply_tile_event_treasure_adds_potion_and_records_opened() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);

        let apply_event = player.apply_tile_event(&data, TileEvent::Treasure);

        assert!(matches!(apply_event, TileApplyEvent::None));
        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.inventory[0].id, "potion");
        assert_eq!(player.opened_treasures.len(), 1);
        assert!(player.is_treasure_opened("field", 1, 1));
    }

    #[test]
    fn apply_tile_event_already_opened_treasure_has_no_duplicate_rewards() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 1);

        player.apply_tile_event(&data, TileEvent::Treasure);
        player.apply_tile_event(&data, TileEvent::Treasure);

        assert_eq!(player.inventory.len(), 1);
        assert_eq!(player.opened_treasures.len(), 1);
    }

    #[test]
    fn apply_tile_event_map_exit_changes_map_when_exit_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);

        let apply_event = player.apply_tile_event(&data, TileEvent::MapExit("town".into()));

        assert!(matches!(apply_event, TileApplyEvent::MapChanged));
        assert_eq!(player.current_map_id, "town");
        assert_eq!(player.x, 0);
        assert_eq!(player.y, 0);
    }

    #[test]
    fn apply_tile_event_dungeon_tile_changes_map_when_dungeon_matches_position() {
        let data = make_game_data();
        let mut player = make_player("field", 1, 2);

        let apply_event = player.apply_tile_event(&data, TileEvent::DungeonEntrance("cave".into()));

        assert!(matches!(apply_event, TileApplyEvent::MapChanged));
        assert_eq!(player.current_map_id, "cave");
        assert_eq!(player.x, 2);
        assert_eq!(player.y, 2);
    }

    #[test]
    fn apply_tile_event_map_exit_with_missing_target_does_nothing() {
        let data = make_game_data();
        let mut player = make_player("field", 2, 1);

        let apply_event =
            player.apply_tile_event(&data, TileEvent::MapExit(String::from("missing")));

        assert!(matches!(apply_event, TileApplyEvent::None));
        assert_eq!(player.current_map_id, "field");
    }

    #[test]
    fn resolve_pause_switches_to_pause_menu_state() {
        let event = resolve(false, ExploreIntent::Pause);

        assert!(matches!(event, ExploreEvent::EnterPauseMenu));
    }

    #[test]
    fn resolve_attack_has_no_effect_in_peaceful_zone() {
        let event = resolve(true, ExploreIntent::UseAction(ExploreAction::BasicAttack));

        assert!(matches!(event, ExploreEvent::None));
    }
}
