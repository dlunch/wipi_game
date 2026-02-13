use alloc::string::String;
use alloc::vec::Vec;

/// 아이템 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Weapon,     // W - 무기
    Armor,      // A - 방어구
    Accessory,  // C - 악세서리
    Consumable, // I - 소비 아이템
}

/// 아이템 데이터
/// 포맷: TYPE:id:name:param1:param2:param3
/// W:sword:녹슨 검:5:0:100    (무기: atk:crit:price)
/// A:leather:가죽갑옷:3:0:150  (방어구: def:mdef:price)
/// C:ring:힘의 반지:2:0:200   (악세서리: atk_bonus:def_bonus:price)
/// I:potion:회복약:30:50      (소비: hp_restore:price)
#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub kind: ItemKind,
    pub param1: i32,
    pub param2: i32,
    pub param3: i32,
    pub price: i32,
}

impl Item {
    pub fn atk(&self) -> i32 {
        match self.kind {
            ItemKind::Weapon => self.param1,
            ItemKind::Accessory => self.param1,
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

impl Tile {
    pub fn from_char(c: char) -> Self {
        match c {
            '#' => Tile::Wall,
            '.' => Tile::Floor,
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

    pub fn find_player_start(&self) -> Option<(usize, usize)> {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get_tile(x, y) == Tile::PlayerStart {
                    return Some((x, y));
                }
            }
        }
        None
    }
}

/// 플레이어 스탯
#[derive(Debug, Clone)]
pub struct PlayerStats {
    pub level: i32,
    pub exp: i32,
    pub exp_to_next: i32,
    pub max_hp: i32,
    pub current_hp: i32,
    pub max_mp: i32,
    pub current_mp: i32,
    pub base_atk: i32,
    pub base_def: i32,
    pub gold: i32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            level: 1,
            exp: 0,
            exp_to_next: 100,
            max_hp: 80,
            current_hp: 80,
            max_mp: 30,
            current_mp: 30,
            base_atk: 12,
            base_def: 8,
            gold: 50,
        }
    }
}

impl PlayerStats {
    pub fn total_atk(&self, weapon: Option<&Item>, accessory: Option<&Item>) -> i32 {
        let weapon_atk = weapon.map(|w| w.atk()).unwrap_or(0);
        let accessory_atk = accessory.map(|a| a.atk()).unwrap_or(0);
        self.base_atk + weapon_atk + accessory_atk
    }

    pub fn total_def(&self, armor: Option<&Item>, accessory: Option<&Item>) -> i32 {
        let armor_def = armor.map(|a| a.def()).unwrap_or(0);
        let accessory_def = accessory.map(|a| a.def()).unwrap_or(0);
        self.base_def + armor_def + accessory_def
    }

    pub fn heal(&mut self, amount: i32) {
        self.current_hp = (self.current_hp + amount).min(self.max_hp);
    }

    pub fn recover_mp(&mut self, amount: i32) {
        self.current_mp = (self.current_mp + amount).min(self.max_mp);
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.current_hp = (self.current_hp - damage).max(0);
    }

    pub fn is_dead(&self) -> bool {
        self.current_hp <= 0
    }

    pub fn add_exp(&mut self, exp: i32) -> bool {
        self.exp += exp;
        let mut leveled_up = false;
        while self.exp >= self.exp_to_next {
            self.level_up();
            leveled_up = true;
        }
        leveled_up
    }

    fn level_up(&mut self) {
        self.exp -= self.exp_to_next;
        self.level += 1;
        self.exp_to_next = self.level * 100;
        self.max_hp += 10;
        self.current_hp = self.max_hp;
        self.max_mp += 5;
        self.current_mp = self.max_mp;
        self.base_atk += 2;
        self.base_def += 1;
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
    Attack,
    Ranged,
    Heal,
    Area,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Skill {
    pub id: &'static str,
    pub name: &'static str,
    pub skill_type: SkillType,
    pub power: i32,
    pub heal_power: i32,
    pub mp_cost: i32,
    pub range: usize,
    pub cooldown: u32,
}

impl Skill {
    pub const FIREBALL: Skill = Skill {
        id: "fireball",
        name: "Fireball",
        skill_type: SkillType::Ranged,
        power: 20,
        heal_power: 0,
        mp_cost: 10,
        range: 3,
        cooldown: 30,
    };

    pub const HEAL: Skill = Skill {
        id: "heal",
        name: "Heal",
        skill_type: SkillType::Heal,
        power: 0,
        heal_power: 30,
        mp_cost: 15,
        range: 0,
        cooldown: 60,
    };

    pub const SPIN_ATTACK: Skill = Skill {
        id: "spin",
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
    use alloc::vec;

    use super::*;

    fn make_item(kind: ItemKind, p1: i32, p2: i32, p3: i32) -> Item {
        Item {
            id: String::from("test"),
            name: String::from("Test"),
            kind,
            param1: p1,
            param2: p2,
            param3: p3,
            price: 100,
        }
    }

    #[test]
    fn item_atk_weapon() {
        let item = make_item(ItemKind::Weapon, 15, 0, 0);
        assert_eq!(item.atk(), 15);
        assert_eq!(item.def(), 0);
        assert_eq!(item.hp_restore(), 0);
    }

    #[test]
    fn item_def_armor() {
        let item = make_item(ItemKind::Armor, 10, 5, 0);
        assert_eq!(item.atk(), 0);
        assert_eq!(item.def(), 10);
    }

    #[test]
    fn item_accessory_gives_both() {
        let item = make_item(ItemKind::Accessory, 3, 2, 0);
        assert_eq!(item.atk(), 3);
        assert_eq!(item.def(), 2);
    }

    #[test]
    fn item_consumable_hp_restore() {
        let item = make_item(ItemKind::Consumable, 50, 0, 0);
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
        assert_eq!(map.find_player_start(), Some((1, 1)));
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
        assert_eq!(map.find_player_start(), None);
    }

    #[test]
    fn player_stats_default() {
        let stats = PlayerStats::default();
        assert_eq!(stats.level, 1);
        assert_eq!(stats.max_hp, 80);
        assert_eq!(stats.current_hp, 80);
        assert_eq!(stats.gold, 50);
    }

    #[test]
    fn player_stats_heal_clamps() {
        let mut stats = PlayerStats::default();
        stats.current_hp = 50;
        stats.heal(100);
        assert_eq!(stats.current_hp, stats.max_hp);
    }

    #[test]
    fn player_stats_take_damage_clamps() {
        let mut stats = PlayerStats::default();
        stats.take_damage(9999);
        assert_eq!(stats.current_hp, 0);
        assert!(stats.is_dead());
    }

    #[test]
    fn player_stats_recover_mp_clamps() {
        let mut stats = PlayerStats::default();
        stats.current_mp = 28;
        stats.recover_mp(100);
        assert_eq!(stats.current_mp, stats.max_mp);
    }

    #[test]
    fn player_stats_level_up() {
        let mut stats = PlayerStats::default();
        let leveled = stats.add_exp(100);
        assert!(leveled);
        assert_eq!(stats.level, 2);
        assert_eq!(stats.exp, 0);
        assert_eq!(stats.exp_to_next, 200);
        assert_eq!(stats.max_hp, 90);
        assert_eq!(stats.current_hp, 90); // healed on level up
    }

    #[test]
    fn player_stats_multi_level_up() {
        let mut stats = PlayerStats::default();
        stats.add_exp(300); // 100 for lv2, 200 for lv3
        assert_eq!(stats.level, 3);
        assert_eq!(stats.exp, 0);
    }

    #[test]
    fn player_stats_no_level_up() {
        let mut stats = PlayerStats::default();
        let leveled = stats.add_exp(50);
        assert!(!leveled);
        assert_eq!(stats.level, 1);
        assert_eq!(stats.exp, 50);
    }

    #[test]
    fn player_stats_total_atk_with_equipment() {
        let stats = PlayerStats::default();
        let weapon = make_item(ItemKind::Weapon, 10, 0, 0);
        let accessory = make_item(ItemKind::Accessory, 3, 2, 0);
        assert_eq!(
            stats.total_atk(Some(&weapon), Some(&accessory)),
            12 + 10 + 3
        );
        assert_eq!(stats.total_atk(None, None), 12);
    }

    #[test]
    fn player_stats_total_def_with_equipment() {
        let stats = PlayerStats::default();
        let armor = make_item(ItemKind::Armor, 5, 0, 0);
        let accessory = make_item(ItemKind::Accessory, 0, 3, 0);
        assert_eq!(stats.total_def(Some(&armor), Some(&accessory)), 8 + 5 + 3);
        assert_eq!(stats.total_def(None, None), 8);
    }
}
