use crate::data::Direction;
use crate::game::state::{FieldEnemy, SkillEffect};
use crate::game::{
    DialogIntent, ExploreIntent, GameInput, InputKey, InventoryIntent, MenuIntent, PauseMenuIntent,
    ShopIntent,
};

pub enum CombatRuntimeEvent {
    EnemySpawn(FieldEnemy),
    EnemyDespawn(u32),
    EnemyMove { enemy_id: u32, x: usize, y: usize },
    EnemyHpSet { enemy_id: u32, hp: i32 },
    EnemyAttackCooldownSet { enemy_id: u32, cooldown: u32 },
    EnemyHitFlashSet { enemy_id: u32, hit_flash: u32 },
    SetPlayerAttackCooldown(u32),
    SetPlayerHitFlash(u32),
    SetSkillEffects(alloc::vec::Vec<SkillEffect>),
    SetUpdateCounter(u32),
    SetRespawnTimer(u32),
    SetNextEnemyInstanceId(u32),
    SetSkillCooldowns([u32; 3]),
    SetMpRegenTimer(u32),
    RecoverMp(i32),
    TakeDamage(i32),
}

pub enum RuntimeEvent {
    Tick,
    KeyDown(InputKey),
    KeyUp(InputKey),
    OverlayCloseRequested,
    GameOverConfirmRequested,
    ErrorConfirmRequested,
    UpdateLoading,
    UpdateMovement,
    UpdateCombat,
    MenuInput(MenuIntent),
    ExploreInput(ExploreIntent),
    InventoryInput(InventoryIntent),
    DialogInput(DialogIntent),
    ShopInput(ShopIntent),
    PauseMenuInput(PauseMenuIntent),
    CombatPlayerAction(crate::game::ExploreAction),
    Loading(crate::game::LoadingEvent),
    Movement(AppMovementEvent),
    Combat(CombatRuntimeEvent),
    Menu(crate::game::MenuEvent),
    Explore(AppExploreEvent),
    Inventory(crate::game::InventoryEvent),
    Dialog(crate::game::DialogEvent),
    Shop(crate::game::ShopEvent),
    PauseMenu(crate::game::PauseMenuEvent),
    Transition(TransitionEvent),
    Exit(i32),
}

impl From<GameInput> for RuntimeEvent {
    fn from(input: GameInput) -> Self {
        match input {
            GameInput::Tick => RuntimeEvent::Tick,
            GameInput::KeyDown(key) => RuntimeEvent::KeyDown(key),
            GameInput::KeyUp(key) => RuntimeEvent::KeyUp(key),
        }
    }
}

pub enum TransitionEvent {
    MapChanged,
    ToExplore,
    ToMenuFromGameOver,
    ReleaseMovementDirection(Direction),
}

pub enum AppExploreEvent {
    MoveDirection(Direction),
    Npc(crate::game::NpcEvent),
    UseAction(crate::game::ExploreAction),
    EnterPauseMenu,
    EnterMenu,
}

pub enum AppMovementEvent {
    Tick(
        crate::game::MovementTickEvent,
        Option<crate::game::TileEvent>,
    ),
}
