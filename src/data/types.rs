use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

/// 아이템 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Weapon,     // W - 무기
    Armor,      // A - 방어구
    Accessory,  // C - 악세서리
    Consumable, // I - 소비 아이템
}

/// 아이템 데이터
/// 포맷: TYPE:id:name:param1:param2:price
/// W:sword:녹슨 검:5:0:100    (무기: atk:unused:price)
/// A:leather:가죽갑옷:3:0:150  (방어구: def:unused:price)
/// C:ring:힘의 반지:2:0:200   (악세서리: atk_bonus:def_bonus:price)
/// I:potion:회복약:30:50      (소비: hp_restore:price)
#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub kind: ItemKind,
    pub param1: i32,
    pub param2: i32,
    pub price: i32,
}

impl Item {
    pub fn atk(&self) -> i32 {
        match self.kind {
            ItemKind::Weapon | ItemKind::Accessory => self.param1,
            _ => 0,
        }
    }

    pub fn def(&self) -> i32 {
        match self.kind {
            ItemKind::Armor => self.param1,
            ItemKind::Accessory => self.param2,
            _ => 0,
        }
    }

    pub fn hp_restore(&self) -> i32 {
        match self.kind {
            ItemKind::Consumable => self.param1,
            _ => 0,
        }
    }
}

/// 적 데이터
/// 포맷: id:name:hp:atk:def:exp:gold
/// slime:슬라임:20:5:2:10:5
#[derive(Debug, Clone)]
pub struct Enemy {
    pub id: String,
    pub name: String,
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub exp: i32,
    pub gold: i32,
}

/// 맵 타일
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,        // # - 벽
    Floor,       // . - 바닥
    PlayerStart, // P - 시작점
    House,       // H - 집/NPC
    Dungeon,     // D - 던전 입구
    Treasure,    // T - 보물상자
    Enemy,       // E - 적 출현 지역
    Exit,        // > - 다음 맵
    Water,       // ~ - 물
    Tree,        // * - 나무
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn apply(&self, x: usize, y: usize) -> (usize, usize) {
        self.apply_distance(x, y, 1)
    }

    pub fn apply_distance(&self, x: usize, y: usize, dist: usize) -> (usize, usize) {
        match self {
            Direction::Up => (x, y.saturating_sub(dist)),
            Direction::Down => (x, y.saturating_add(dist)),
            Direction::Left => (x.saturating_sub(dist), y),
            Direction::Right => (x.saturating_add(dist), y),
        }
    }
}

impl Tile {
    pub fn from_char(c: char) -> Self {
        match c {
            '#' => Tile::Wall,
            'P' => Tile::PlayerStart,
            'H' => Tile::House,
            'D' => Tile::Dungeon,
            'T' => Tile::Treasure,
            'E' => Tile::Enemy,
            '>' => Tile::Exit,
            '~' => Tile::Water,
            '*' => Tile::Tree,
            _ => Tile::Floor,
        }
    }

    pub fn is_passable(&self) -> bool {
        matches!(
            self,
            Tile::Floor
                | Tile::PlayerStart
                | Tile::House
                | Tile::Dungeon
                | Tile::Treasure
                | Tile::Enemy
                | Tile::Exit
        )
    }
}

/// 맵 데이터
/// 포맷:
/// @MAP:map_id:display_name
/// ################
/// #..............#
/// #...H....H.....#
/// ################
/// @ENCOUNTERS:slime:3:goblin:1
/// @NEXT:>:next_map_id
/// @END
#[derive(Debug, Clone)]
pub struct Map {
    pub id: String,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub encounters: Vec<(String, i32)>,
    pub exits: Vec<(usize, usize, String)>,
    pub dungeons: Vec<(usize, usize, String)>,
    pub npcs: Vec<(usize, usize, String)>,
    pub peaceful: bool,
}

impl Map {
    pub fn get_tile(&self, x: usize, y: usize) -> Tile {
        if x >= self.width || y >= self.height {
            return Tile::Wall;
        }
        self.tiles[y * self.width + x]
    }

    pub fn find_player_start(&self) -> Result<(usize, usize)> {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get_tile(x, y) == Tile::PlayerStart {
                    return Ok((x, y));
                }
            }
        }
        Err(anyhow!("Player start tile not found in map '{}'", self.id))
    }
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub id: String,
    pub name: String,
    pub map_id: String,
    pub x: usize,
    pub y: usize,
    pub npc_type: NpcType,
    pub dialog_id: String,
    pub shop_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Villager,
    ShopKeeper,
    QuestGiver,
    Healer,
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub id: String,
    pub lines: Vec<DialogLine>,
}

#[derive(Debug, Clone)]
pub struct DialogLine {
    pub text: String,
    pub condition: Option<DialogCondition>,
    pub action: Option<DialogAction>,
}

#[derive(Debug, Clone)]
pub enum DialogCondition {
    HasQuest(String),
    QuestComplete(String),
    HasItem(String),
    HasGold(i32),
}

#[derive(Debug, Clone)]
pub enum DialogAction {
    GiveQuest(String),
    CompleteQuest(String),
    GiveItem(String),
    TakeItem(String),
    GiveGold(i32),
    TakeGold(i32),
    OpenShop(String),
    Heal,
}

#[derive(Debug, Clone)]
pub struct Quest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub quest_type: QuestType,
    pub target_id: String,
    pub target_count: i32,
    pub reward_exp: i32,
    pub reward_gold: i32,
    pub reward_item: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestType {
    Kill,
    Collect,
    Talk,
    Reach,
}

#[derive(Debug, Clone, Default)]
pub struct QuestProgress {
    pub quest_id: String,
    pub current_count: i32,
    pub completed: bool,
    pub rewarded: bool,
}

#[derive(Debug, Clone)]
pub struct Shop {
    pub id: String,
    pub name: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillType {
    Ranged,
    Heal,
    Area,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: &'static str,
    pub skill_type: SkillType,
    pub power: i32,
    pub heal_power: i32,
    pub mp_cost: i32,
    pub range: usize,
    pub cooldown: u32,
}

#[derive(Debug, Clone)]
pub struct StartItem {
    pub item_id: String,
    pub count: i32,
}

#[derive(Debug, Clone)]
pub struct NewGameConfig {
    pub player_name: String,
    pub start_map: String,
    pub fallback_map: String,
    pub intro_dialog: Option<(String, String)>,
    pub equip_weapon: Option<String>,
    pub equip_armor: Option<String>,
    pub treasure_item: Option<String>,
    pub items: Vec<StartItem>,
}

impl Default for NewGameConfig {
    fn default() -> Self {
        Self {
            player_name: String::from("Hero"),
            start_map: String::from("village"),
            fallback_map: String::from("village"),
            intro_dialog: None,
            equip_weapon: None,
            equip_armor: None,
            treasure_item: Some(String::from("potion")),
            items: Vec::new(),
        }
    }
}

impl Skill {
    pub const FIREBALL: Skill = Skill {
        name: "Fireball",
        skill_type: SkillType::Ranged,
        power: 20,
        heal_power: 0,
        mp_cost: 10,
        range: 3,
        cooldown: 30,
    };

    pub const HEAL: Skill = Skill {
        name: "Heal",
        skill_type: SkillType::Heal,
        power: 0,
        heal_power: 30,
        mp_cost: 15,
        range: 0,
        cooldown: 60,
    };

    pub const SPIN_ATTACK: Skill = Skill {
        name: "Spin",
        skill_type: SkillType::Area,
        power: 15,
        heal_power: 0,
        mp_cost: 8,
        range: 1,
        cooldown: 20,
    };
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::{Item, ItemKind, Map, Tile};

    fn make_item(kind: ItemKind, p1: i32, p2: i32) -> Item {
        Item {
            id: String::from("test"),
            name: String::from("Test"),
            kind,
            param1: p1,
            param2: p2,
            price: 100,
        }
    }

    #[test]
    fn item_atk_weapon() {
        let item = make_item(ItemKind::Weapon, 15, 0);
        assert_eq!(item.atk(), 15);
        assert_eq!(item.def(), 0);
        assert_eq!(item.hp_restore(), 0);
    }

    #[test]
    fn item_def_armor() {
        let item = make_item(ItemKind::Armor, 10, 5);
        assert_eq!(item.atk(), 0);
        assert_eq!(item.def(), 10);
    }

    #[test]
    fn item_accessory_gives_both() {
        let item = make_item(ItemKind::Accessory, 3, 2);
        assert_eq!(item.atk(), 3);
        assert_eq!(item.def(), 2);
    }

    #[test]
    fn item_consumable_hp_restore() {
        let item = make_item(ItemKind::Consumable, 50, 0);
        assert_eq!(item.hp_restore(), 50);
        assert_eq!(item.atk(), 0);
        assert_eq!(item.def(), 0);
    }

    #[test]
    fn tile_from_char_all_types() {
        assert_eq!(Tile::from_char('#'), Tile::Wall);
        assert_eq!(Tile::from_char('.'), Tile::Floor);
        assert_eq!(Tile::from_char('P'), Tile::PlayerStart);
        assert_eq!(Tile::from_char('H'), Tile::House);
        assert_eq!(Tile::from_char('D'), Tile::Dungeon);
        assert_eq!(Tile::from_char('T'), Tile::Treasure);
        assert_eq!(Tile::from_char('E'), Tile::Enemy);
        assert_eq!(Tile::from_char('>'), Tile::Exit);
        assert_eq!(Tile::from_char('~'), Tile::Water);
        assert_eq!(Tile::from_char('*'), Tile::Tree);
        assert_eq!(Tile::from_char('?'), Tile::Floor); // unknown → Floor
    }

    #[test]
    fn tile_passability() {
        assert!(!Tile::Wall.is_passable());
        assert!(!Tile::Water.is_passable());
        assert!(!Tile::Tree.is_passable());
        assert!(Tile::Floor.is_passable());
        assert!(Tile::PlayerStart.is_passable());
        assert!(Tile::Enemy.is_passable());
        assert!(Tile::Exit.is_passable());
        assert!(Tile::Treasure.is_passable());
        assert!(Tile::Dungeon.is_passable());
        assert!(Tile::House.is_passable());
    }

    #[test]
    fn map_get_tile_out_of_bounds() {
        let map = Map {
            id: String::from("test"),
            name: String::from("Test"),
            width: 3,
            height: 2,
            tiles: vec![
                Tile::Wall,
                Tile::Floor,
                Tile::Wall,
                Tile::Wall,
                Tile::PlayerStart,
                Tile::Wall,
            ],
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        };
        assert_eq!(map.get_tile(1, 0), Tile::Floor);
        assert_eq!(map.get_tile(1, 1), Tile::PlayerStart);
        assert_eq!(map.get_tile(99, 0), Tile::Wall); // out of bounds
        assert_eq!(map.get_tile(0, 99), Tile::Wall); // out of bounds
    }

    #[test]
    fn map_find_player_start() {
        let map = Map {
            id: String::from("test"),
            name: String::from("Test"),
            width: 3,
            height: 2,
            tiles: vec![
                Tile::Wall,
                Tile::Floor,
                Tile::Wall,
                Tile::Wall,
                Tile::PlayerStart,
                Tile::Wall,
            ],
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        };
        assert_eq!(map.find_player_start().unwrap(), (1, 1));
    }

    #[test]
    fn map_find_player_start_none() {
        let map = Map {
            id: String::from("test"),
            name: String::from("Test"),
            width: 2,
            height: 1,
            tiles: vec![Tile::Wall, Tile::Floor],
            encounters: Vec::new(),
            exits: Vec::new(),
            dungeons: Vec::new(),
            npcs: Vec::new(),
            peaceful: false,
        };
        assert!(map.find_player_start().is_err());
    }
}
