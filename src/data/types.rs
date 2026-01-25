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
            max_hp: 50,
            current_hp: 50,
            max_mp: 20,
            current_mp: 20,
            base_atk: 10,
            base_def: 5,
            gold: 0,
        }
    }
}

impl PlayerStats {
    pub fn total_atk(&self, weapon: Option<&Item>) -> i32 {
        let weapon_atk = weapon.map(|w| w.param1).unwrap_or(0);
        self.base_atk + weapon_atk
    }

    pub fn total_def(&self, armor: Option<&Item>) -> i32 {
        let armor_def = armor.map(|a| a.param1).unwrap_or(0);
        self.base_def + armor_def
    }

    pub fn heal(&mut self, amount: i32) {
        self.current_hp = (self.current_hp + amount).min(self.max_hp);
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.current_hp = (self.current_hp - damage).max(0);
    }

    pub fn is_dead(&self) -> bool {
        self.current_hp <= 0
    }

    pub fn add_exp(&mut self, exp: i32) -> bool {
        self.exp += exp;
        if self.exp >= self.exp_to_next {
            self.level_up();
            true
        } else {
            false
        }
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
