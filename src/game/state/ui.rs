use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, anyhow};

use crate::data::{Dialog, DialogLine, Direction, Item, Shop, Skill};
use crate::game::selection::{step_down, step_up};
use crate::game::systems::runtime::{ApplyContext, DomainEventApplier};
use crate::game::{GameEvent, RuntimeEvent, UiEvent};
use crate::game::{
    GameState, PlayerAction, PlayerEvent, SessionState, TransitionEvent, has_save_data,
};

pub const INVENTORY_VISIBLE_ITEMS: usize = 8;
pub const SHOP_VISIBLE_ITEMS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKey {
    Ok,
    Back,
    Up,
    Down,
    Left,
    Right,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
}

impl InputKey {
    pub fn direction(self) -> Option<crate::data::Direction> {
        match self {
            InputKey::Up => Some(crate::data::Direction::Up),
            InputKey::Down => Some(crate::data::Direction::Down),
            InputKey::Left => Some(crate::data::Direction::Left),
            InputKey::Right => Some(crate::data::Direction::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GameInput {
    KeyDown(InputKey),
    KeyUp(InputKey),
}

#[derive(Debug, Default)]
pub struct UiState {
    pub explore: ExploreUiState,
    pub menu: MenuUiState,
    pub pause_menu: PauseMenuUiState,
    pub inventory: InventoryUiState,
    pub shop: ShopUiState,
    pub dialog: DialogUiState,
}

pub trait UiInputEventResolver {
    fn resolve_input(
        &mut self,
        input: GameInput,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<UiEvent>;
}

impl UiInputEventResolver for UiState {
    fn resolve_input(
        &mut self,
        input: GameInput,
        game_state: &GameState,
        session: Option<&SessionState>,
    ) -> Vec<UiEvent> {
        resolve_input(input, game_state, self, session)
    }
}

pub trait UiEventApplier {
    fn apply_ui_event(
        &mut self,
        event: UiEvent,
        game_state: &GameState,
        session: Option<&SessionState>,
        data: &crate::game::GameData,
    ) -> Vec<GameEvent>;
}

impl UiEventApplier for UiState {
    fn apply_ui_event(
        &mut self,
        event: UiEvent,
        game_state: &GameState,
        session: Option<&SessionState>,
        data: &crate::game::GameData,
    ) -> Vec<GameEvent> {
        match event {
            UiEvent::OverlayCloseRequested => {
                alloc::vec![GameEvent::Transition(TransitionEvent::ToExplore)]
            }
            UiEvent::GameOverConfirmRequested => {
                alloc::vec![GameEvent::Transition(TransitionEvent::ToMenuFromGameOver)]
            }
            UiEvent::ErrorConfirmRequested => alloc::vec![GameEvent::Exit(1)],
            UiEvent::MovementKeyReleased(direction) => alloc::vec![GameEvent::Transition(
                TransitionEvent::ReleaseMovementDirection(direction),
            )],
            UiEvent::MenuInput(key) => {
                resolve_menu_events(self.menu.selected, &self.menu.state.items, key)
                    .into_iter()
                    .map(GameEvent::Menu)
                    .collect()
            }
            UiEvent::PauseMenuInput(key) => resolve_pause_menu_events(
                self.pause_menu.selected,
                self.pause_menu.state.items.len(),
                key,
            )
            .into_iter()
            .map(GameEvent::PauseMenu)
            .collect(),
            UiEvent::ExploreInput(key) => {
                if !matches!(game_state, GameState::Explore) {
                    return Vec::new();
                }
                let Some(s) = session else {
                    return Vec::new();
                };
                self.explore
                    .resolve_events_for_key(key, &s.player, data)
                    .into_iter()
                    .map(GameEvent::Explore)
                    .collect()
            }
            UiEvent::InventoryInput(key) => {
                let Some(s) = session else {
                    return Vec::new();
                };
                resolve_inventory_events(self.inventory.selected, s.player.inventory.len(), key)
                    .into_iter()
                    .map(GameEvent::Inventory)
                    .collect()
            }
            UiEvent::DialogInput(key) => resolve_dialog_events(self.dialog.state.as_ref(), key)
                .into_iter()
                .map(GameEvent::Dialog)
                .collect(),
            UiEvent::ShopBuySelected(selected) => {
                let Some(s) = session else {
                    return Vec::new();
                };
                let shop_items = self
                    .shop
                    .state
                    .as_ref()
                    .map(|state| state.items.as_slice())
                    .unwrap_or(&[]);
                if let Some(item) = shop_items.get(selected).cloned()
                    && s.player.stats.gold >= item.price
                {
                    alloc::vec![GameEvent::Shop(crate::game::ShopEvent::BuyItem(item))]
                } else {
                    Vec::new()
                }
            }
            UiEvent::ShopSellSelected(selected) => {
                alloc::vec![GameEvent::Shop(crate::game::ShopEvent::SellSelected(
                    selected
                ))]
            }
            UiEvent::ShopClose => {
                alloc::vec![GameEvent::Shop(crate::game::ShopEvent::CloseToExplore)]
            }
        }
    }
}

fn resolve_input(
    input: GameInput,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<UiEvent> {
    match input {
        GameInput::KeyDown(key) => resolve_keydown(key, game_state, ui, session),
        GameInput::KeyUp(key) => resolve_keyup(key, game_state, session),
    }
}

fn resolve_keydown(
    key: InputKey,
    game_state: &GameState,
    ui: &mut UiState,
    session: Option<&SessionState>,
) -> Vec<UiEvent> {
    match game_state {
        GameState::Loading(_) => Vec::new(),
        GameState::Menu => ui.menu.event_for_key(key).into_iter().collect(),
        GameState::Explore => ui.explore.events_for_key(key),
        GameState::Inventory => ui.inventory.event_for_key(key).into_iter().collect(),
        GameState::Stats | GameState::QuestLog => {
            if matches!(key, InputKey::Back | InputKey::Ok) {
                vec![UiEvent::OverlayCloseRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Dialog => ui.dialog.event_for_key(key).into_iter().collect(),
        GameState::Shop => {
            let inventory_len = session
                .map(|session_state| session_state.player.inventory.len())
                .unwrap_or(0);
            ui.shop
                .event_for_key(key, inventory_len)
                .into_iter()
                .collect()
        }
        GameState::PauseMenu => ui.pause_menu.event_for_key(key).into_iter().collect(),
        GameState::GameOver => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::GameOverConfirmRequested]
            } else {
                Vec::new()
            }
        }
        GameState::Error(_) => {
            if matches!(key, InputKey::Ok) {
                vec![UiEvent::ErrorConfirmRequested]
            } else {
                Vec::new()
            }
        }
    }
}

fn resolve_keyup(
    key: InputKey,
    game_state: &GameState,
    session: Option<&SessionState>,
) -> Vec<UiEvent> {
    if matches!(game_state, GameState::Explore)
        && session.is_some()
        && let Some(direction) = key.direction()
    {
        vec![UiEvent::MovementKeyReleased(direction)]
    } else {
        Vec::new()
    }
}

fn resolve_menu_events(
    selected: usize,
    items: &[(&str, MenuAction)],
    key: InputKey,
) -> Vec<crate::game::MenuEvent> {
    let event = match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                crate::game::MenuEvent::SetSelected(next)
            } else {
                crate::game::MenuEvent::None
            }
        }
        InputKey::Down => {
            let next = step_down(selected, items.len());
            if next != selected {
                crate::game::MenuEvent::SetSelected(next)
            } else {
                crate::game::MenuEvent::None
            }
        }
        InputKey::Ok => {
            if let Some((_, action)) = items.get(selected).copied() {
                crate::game::MenuEvent::Action(action)
            } else {
                crate::game::MenuEvent::None
            }
        }
        _ => crate::game::MenuEvent::None,
    };

    match event {
        crate::game::MenuEvent::None => Vec::new(),
        event => vec![event],
    }
}

fn resolve_pause_menu_events(
    selected: usize,
    item_count: usize,
    key: InputKey,
) -> Vec<crate::game::PauseMenuEvent> {
    let event = match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                crate::game::PauseMenuEvent::SetSelected(next)
            } else {
                crate::game::PauseMenuEvent::None
            }
        }
        InputKey::Down => {
            let next = step_down(selected, item_count);
            if next != selected {
                crate::game::PauseMenuEvent::SetSelected(next)
            } else {
                crate::game::PauseMenuEvent::None
            }
        }
        InputKey::Ok => match selected {
            0 => crate::game::PauseMenuEvent::OpenInventory,
            1 => crate::game::PauseMenuEvent::OpenStats,
            2 => crate::game::PauseMenuEvent::OpenQuestLog,
            3 => crate::game::PauseMenuEvent::SaveAndReturnExplore,
            _ => crate::game::PauseMenuEvent::None,
        },
        InputKey::Back | InputKey::Key0 => crate::game::PauseMenuEvent::BackToExplore,
        _ => crate::game::PauseMenuEvent::None,
    };

    match event {
        crate::game::PauseMenuEvent::None => Vec::new(),
        event => vec![event],
    }
}

fn resolve_inventory_events(
    selected: usize,
    inventory_len: usize,
    key: InputKey,
) -> Vec<crate::game::InventoryEvent> {
    let event = match key {
        InputKey::Up => {
            let next = step_up(selected);
            if next != selected {
                crate::game::InventoryEvent::SetSelected(next)
            } else {
                crate::game::InventoryEvent::None
            }
        }
        InputKey::Down => {
            let next = step_down(selected, inventory_len);
            if next != selected {
                crate::game::InventoryEvent::SetSelected(next)
            } else {
                crate::game::InventoryEvent::None
            }
        }
        InputKey::Ok => crate::game::InventoryEvent::UseSelected(selected),
        InputKey::Back => crate::game::InventoryEvent::CloseToExplore,
        _ => crate::game::InventoryEvent::None,
    };

    match event {
        crate::game::InventoryEvent::None => Vec::new(),
        event => vec![event],
    }
}

fn resolve_dialog_events(
    dialog_state: Option<&DialogState>,
    key: InputKey,
) -> Vec<crate::game::DialogEvent> {
    let event = match key {
        InputKey::Back => {
            crate::game::DialogEvent::Transition(crate::game::DialogTransition::CloseToExplore)
        }
        InputKey::Ok => {
            if let Some(dialog_state_ref) = dialog_state {
                if dialog_state_ref.current_line >= dialog_state_ref.lines.len() {
                    crate::game::DialogEvent::Transition(
                        crate::game::DialogTransition::CloseToExplore,
                    )
                } else {
                    let transition = if dialog_state_ref.current_line + 1
                        < dialog_state_ref.lines.len()
                    {
                        crate::game::DialogTransition::SetLine(dialog_state_ref.current_line + 1)
                    } else {
                        crate::game::DialogTransition::CloseToExplore
                    };
                    if let Some(action) = dialog_state_ref
                        .lines
                        .get(dialog_state_ref.current_line)
                        .and_then(|line| line.action.as_ref())
                        .cloned()
                    {
                        crate::game::DialogEvent::Action(action, transition)
                    } else {
                        crate::game::DialogEvent::Transition(transition)
                    }
                }
            } else {
                crate::game::DialogEvent::None
            }
        }
        _ => crate::game::DialogEvent::None,
    };

    match event {
        crate::game::DialogEvent::None => Vec::new(),
        event => vec![event],
    }
}

struct UiDomainApplier;

static UI_DOMAIN_APPLIER: UiDomainApplier = UiDomainApplier;

pub fn domain_appliers() -> alloc::vec::Vec<&'static dyn DomainEventApplier> {
    alloc::vec![&UI_DOMAIN_APPLIER]
}

impl DomainEventApplier for UiDomainApplier {
    fn handles(&self, event: &RuntimeEvent) -> bool {
        matches!(
            event,
            RuntimeEvent::Menu(_)
                | RuntimeEvent::PauseMenu(_)
                | RuntimeEvent::StartNewGame
                | RuntimeEvent::ContinueGame
                | RuntimeEvent::OpenPauseMenu
                | RuntimeEvent::OpenMenuFromExplore
                | RuntimeEvent::Explore(_)
                | RuntimeEvent::Inventory(_)
                | RuntimeEvent::Dialog(_)
                | RuntimeEvent::ApplyDialogTransition(_)
                | RuntimeEvent::Shop(_)
                | RuntimeEvent::OpenDialogState(_)
                | RuntimeEvent::OpenShopById(_)
        )
    }

    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &RuntimeEvent) -> Result<()> {
        match event {
            RuntimeEvent::StartNewGame | RuntimeEvent::ContinueGame => {
                *ctx.ui = UiState::default();
                if matches!(ctx.state, GameState::Dialog)
                    && let Some(dialog_state) = intro_dialog_state(ctx.data)
                {
                    ctx.ui.dialog.set(Some(dialog_state));
                }
            }
            RuntimeEvent::Menu(event) => match event {
                crate::game::MenuEvent::None => {}
                crate::game::MenuEvent::SetSelected(selected) => {
                    ctx.ui_mut().menu.set_selected(*selected)
                }
                crate::game::MenuEvent::Action(_) => {}
            },
            RuntimeEvent::PauseMenu(event) => match event {
                crate::game::PauseMenuEvent::None => {}
                crate::game::PauseMenuEvent::SetSelected(selected) => {
                    ctx.ui_mut().pause_menu.set_selected(*selected)
                }
                crate::game::PauseMenuEvent::OpenInventory => {
                    ctx.ui_mut().inventory.reset();
                    ctx.transition_to(GameState::Inventory);
                }
                crate::game::PauseMenuEvent::OpenStats => ctx.transition_to(GameState::Stats),
                crate::game::PauseMenuEvent::OpenQuestLog => ctx.transition_to(GameState::QuestLog),
                crate::game::PauseMenuEvent::SaveAndReturnExplore => {
                    {
                        let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
                        let _ = crate::game::save_game(&s.player);
                    }
                    ctx.ui_mut().shop.reset();
                    ctx.transition_to(GameState::Explore);
                }
                crate::game::PauseMenuEvent::BackToExplore => ctx.transition_to(GameState::Explore),
            },
            RuntimeEvent::OpenPauseMenu => {
                ctx.ui_mut().pause_menu.reset();
                ctx.transition_to(GameState::PauseMenu);
            }
            RuntimeEvent::OpenMenuFromExplore => {
                {
                    let s = ctx.session().ok_or_else(|| anyhow!("No active session"))?;
                    let _ = crate::game::save_game(&s.player);
                }
                ctx.ui_mut().menu.set_menu(MenuState::new(has_save_data()));
                ctx.transition_to(GameState::Menu);
            }
            RuntimeEvent::Explore(crate::game::AppExploreEvent::MoveDirection(direction)) => {
                let s = ctx
                    .session_mut()
                    .ok_or_else(|| anyhow!("No active session"))?;
                s.movement.on_direction_pressed(*direction);
            }
            RuntimeEvent::Explore(_) => {}
            RuntimeEvent::Inventory(event) => match event {
                crate::game::InventoryEvent::None => {}
                crate::game::InventoryEvent::SetSelected(selected) => {
                    ctx.ui_mut().inventory.set_selected(*selected)
                }
                crate::game::InventoryEvent::UseSelected(index) => {
                    let s = ctx
                        .session_mut()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let _ = s.player.apply(PlayerAction::UseItem { index: *index });
                }
                crate::game::InventoryEvent::CloseToExplore => {
                    ctx.transition_to(GameState::Explore)
                }
            },
            RuntimeEvent::Dialog(_) => {}
            RuntimeEvent::ApplyDialogTransition(transition) => match transition {
                crate::game::DialogTransition::SetLine(line) => {
                    if let Some(dialog_state) = ctx.ui_mut().dialog.state.as_mut() {
                        dialog_state.current_line = *line;
                    }
                    ctx.transition_to(GameState::Dialog);
                }
                crate::game::DialogTransition::CloseToExplore => {
                    ctx.ui_mut().dialog.close();
                    ctx.transition_to(GameState::Explore);
                }
            },
            RuntimeEvent::Shop(event) => match event {
                crate::game::ShopEvent::BuyItem(item) => {
                    let s = ctx
                        .session_mut()
                        .ok_or_else(|| anyhow!("No active session"))?;
                    let _ = s.player.apply(PlayerAction::AddGold(-item.price));
                    let _ = s.player.apply(PlayerAction::AddItem(item.clone()));
                }
                crate::game::ShopEvent::SellSelected(index) => {
                    let (sold, len_after) = {
                        let s = ctx
                            .session_mut()
                            .ok_or_else(|| anyhow!("No active session"))?;
                        let sold = if let PlayerEvent::ItemRemoved(Some(item)) =
                            s.player.apply(PlayerAction::RemoveItemAt(*index))
                        {
                            let _ = s.player.apply(PlayerAction::AddGold(item.price / 2));
                            true
                        } else {
                            false
                        };
                        (sold, s.player.inventory.len())
                    };
                    if sold {
                        let current_selected = ctx.ui.shop.selected;
                        if current_selected >= len_after && current_selected > 0 {
                            ctx.ui_mut().shop.set_selected(current_selected - 1);
                        }
                    }
                }
                crate::game::ShopEvent::CloseToExplore => ctx.transition_to(GameState::Explore),
            },
            RuntimeEvent::OpenDialogState(dialog_state) => {
                ctx.ui_mut().dialog.open(dialog_state.clone());
                ctx.transition_to(GameState::Dialog);
            }
            RuntimeEvent::OpenShopById(shop_id) => {
                let _ = ctx.open_shop_by_id(shop_id);
            }
            _ => {}
        }
        Ok(())
    }
}

fn intro_dialog_state(data: &crate::game::GameData) -> Option<DialogState> {
    let (dialog_id, npc_name) = data.newgame.intro_dialog.as_ref()?;
    let dialog = data.find_dialog(dialog_id)?;
    Some(DialogState::from_dialog(npc_name.clone(), dialog))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExploreAction {
    BasicAttack,
    Fireball,
    Heal,
    SpinAttack,
}

impl ExploreAction {
    pub fn skill(self) -> Option<(usize, &'static Skill)> {
        match self {
            ExploreAction::BasicAttack => None,
            ExploreAction::Fireball => Some((0, &Skill::FIREBALL)),
            ExploreAction::Heal => Some((1, &Skill::HEAL)),
            ExploreAction::SpinAttack => Some((2, &Skill::SPIN_ATTACK)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExploreAction::BasicAttack => "Attack",
            ExploreAction::Fireball => Skill::FIREBALL.name,
            ExploreAction::Heal => Skill::HEAL.name,
            ExploreAction::SpinAttack => Skill::SPIN_ATTACK.name,
        }
    }
}

#[derive(Debug)]
pub struct ExploreUiState {
    pub ok_action: ExploreAction,
    pub key_actions: [Option<ExploreAction>; 3],
}

impl Default for ExploreUiState {
    fn default() -> Self {
        Self {
            ok_action: ExploreAction::BasicAttack,
            key_actions: [
                Some(ExploreAction::Fireball),
                Some(ExploreAction::Heal),
                Some(ExploreAction::SpinAttack),
            ],
        }
    }
}

impl ExploreUiState {
    pub fn events_for_key(&self, key: InputKey) -> Vec<UiEvent> {
        if matches!(
            key,
            InputKey::Up
                | InputKey::Down
                | InputKey::Left
                | InputKey::Right
                | InputKey::Ok
                | InputKey::Key0
                | InputKey::Key1
                | InputKey::Key2
                | InputKey::Key3
                | InputKey::Back
        ) {
            vec![UiEvent::ExploreInput(key)]
        } else {
            Vec::new()
        }
    }

    pub fn resolve_events_for_key(
        &self,
        key: InputKey,
        player: &crate::game::PlayerState,
        data: &crate::game::GameData,
    ) -> Vec<crate::game::AppExploreEvent> {
        let mut events = Vec::new();

        match key {
            InputKey::Up => {
                events.push(crate::game::AppExploreEvent::MoveDirection(Direction::Up));
            }
            InputKey::Down => {
                events.push(crate::game::AppExploreEvent::MoveDirection(Direction::Down));
            }
            InputKey::Left => {
                events.push(crate::game::AppExploreEvent::MoveDirection(Direction::Left));
            }
            InputKey::Right => {
                events.push(crate::game::AppExploreEvent::MoveDirection(
                    Direction::Right,
                ));
            }
            InputKey::Ok => {
                if let Some(npc_event) = crate::game::npc::resolve(
                    player,
                    data,
                    crate::game::npc::NpcIntent::Interact {
                        facing: player.facing,
                    },
                ) {
                    events.push(crate::game::AppExploreEvent::Npc(npc_event));
                } else {
                    events.push(crate::game::AppExploreEvent::UseAction(self.ok_action));
                }
            }
            InputKey::Key1 => {
                if let Some(action) = self.key_actions[0] {
                    events.push(crate::game::AppExploreEvent::UseAction(action));
                }
            }
            InputKey::Key2 => {
                if let Some(action) = self.key_actions[1] {
                    events.push(crate::game::AppExploreEvent::UseAction(action));
                }
            }
            InputKey::Key3 => {
                if let Some(action) = self.key_actions[2] {
                    events.push(crate::game::AppExploreEvent::UseAction(action));
                }
            }
            InputKey::Key0 => {
                events.push(crate::game::AppExploreEvent::EnterPauseMenu);
            }
            InputKey::Back => {
                events.push(crate::game::AppExploreEvent::EnterMenu);
            }
            _ => {}
        }

        let is_peaceful = data
            .find_map(&player.current_map_id)
            .is_some_and(|map| map.peaceful);
        if is_peaceful {
            events.retain(|event| !matches!(event, crate::game::AppExploreEvent::UseAction(_)));
        }

        events
    }
}

#[derive(Debug)]
pub struct MenuUiState {
    pub state: MenuState,
    pub selected: usize,
}

impl Default for MenuUiState {
    fn default() -> Self {
        Self {
            state: MenuState::new(false),
            selected: 0,
        }
    }
}

impl MenuUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(key, InputKey::Up | InputKey::Down | InputKey::Ok) {
            Some(UiEvent::MenuInput(key))
        } else {
            None
        }
    }

    pub fn set_menu(&mut self, state: MenuState) {
        self.state = state;
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug)]
pub struct PauseMenuUiState {
    pub state: PauseMenuState,
    pub selected: usize,
}

impl Default for PauseMenuUiState {
    fn default() -> Self {
        Self {
            state: PauseMenuState::new(),
            selected: 0,
        }
    }
}

impl PauseMenuUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back | InputKey::Key0
        ) {
            Some(UiEvent::PauseMenuInput(key))
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug, Default)]
pub struct InventoryUiState {
    pub selected: usize,
}

impl InventoryUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(
            key,
            InputKey::Up | InputKey::Down | InputKey::Ok | InputKey::Back
        ) {
            Some(UiEvent::InventoryInput(key))
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }
}

#[derive(Debug)]
pub struct ShopUiState {
    pub state: Option<ShopState>,
    pub mode: ShopMode,
    pub selected: usize,
}

impl Default for ShopUiState {
    fn default() -> Self {
        Self {
            state: None,
            mode: ShopMode::Select,
            selected: 0,
        }
    }
}

impl ShopUiState {
    pub fn event_for_key(&mut self, key: InputKey, inventory_len: usize) -> Option<UiEvent> {
        self.handle_key(key, inventory_len)
    }

    pub fn open(&mut self, state: ShopState) {
        self.state = Some(state);
        self.mode = ShopMode::Select;
        self.selected = 0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
    }

    pub fn handle_key(&mut self, key: InputKey, inventory_len: usize) -> Option<UiEvent> {
        let shop_items_len = self
            .state
            .as_ref()
            .map(|state| state.items.len())
            .unwrap_or(0);

        match self.mode {
            ShopMode::Select => match key {
                InputKey::Up => {
                    self.selected = step_up(self.selected);
                    None
                }
                InputKey::Down => {
                    self.selected = step_down(self.selected, 2);
                    None
                }
                InputKey::Ok => {
                    if self.selected == 0 {
                        self.mode = ShopMode::Buy;
                    } else {
                        self.mode = ShopMode::Sell;
                    }
                    self.selected = 0;
                    None
                }
                InputKey::Back => Some(UiEvent::ShopClose),
                _ => None,
            },
            ShopMode::Buy => match key {
                InputKey::Up => {
                    self.selected = step_up(self.selected);
                    None
                }
                InputKey::Down => {
                    self.selected = step_down(self.selected, shop_items_len);
                    None
                }
                InputKey::Ok => Some(UiEvent::ShopBuySelected(self.selected)),
                InputKey::Back => {
                    self.mode = ShopMode::Select;
                    self.selected = 0;
                    None
                }
                _ => None,
            },
            ShopMode::Sell => match key {
                InputKey::Up => {
                    self.selected = step_up(self.selected);
                    None
                }
                InputKey::Down => {
                    self.selected = step_down(self.selected, inventory_len);
                    None
                }
                InputKey::Ok => Some(UiEvent::ShopSellSelected(self.selected)),
                InputKey::Back => {
                    self.mode = ShopMode::Select;
                    self.selected = 0;
                    None
                }
                _ => None,
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct DialogUiState {
    pub state: Option<DialogState>,
}

impl DialogUiState {
    pub fn event_for_key(&self, key: InputKey) -> Option<UiEvent> {
        if matches!(key, InputKey::Ok | InputKey::Back) {
            Some(UiEvent::DialogInput(key))
        } else {
            None
        }
    }

    pub fn open(&mut self, state: DialogState) {
        self.state = Some(state);
    }

    pub fn close(&mut self) {
        self.state = None;
    }

    pub fn set(&mut self, state: Option<DialogState>) {
        self.state = state;
    }
}

#[derive(Debug)]
pub struct MenuState {
    pub title: &'static str,
    pub items: Vec<(&'static str, MenuAction)>,
}

impl MenuState {
    pub fn new(has_save: bool) -> Self {
        let items = if has_save {
            vec![
                ("NEW GAME", MenuAction::NewGame),
                ("CONTINUE", MenuAction::Continue),
                ("EXIT", MenuAction::Exit),
            ]
        } else {
            vec![
                ("NEW GAME", MenuAction::NewGame),
                ("EXIT", MenuAction::Exit),
            ]
        };

        Self {
            title: "LOST KINGDOM",
            items,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    NewGame,
    Continue,
    Exit,
}

#[derive(Debug)]
pub struct PauseMenuState {
    pub items: Vec<&'static str>,
}

impl PauseMenuState {
    pub fn new() -> Self {
        Self {
            items: vec!["Inventory", "Stats", "Quests", "Save"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct DialogState {
    pub npc_name: String,
    pub lines: Vec<DialogLine>,
    pub current_line: usize,
}

impl DialogState {
    pub fn from_dialog(npc_name: String, dialog: &Dialog) -> Self {
        Self::new(npc_name, dialog.lines.clone())
    }

    pub fn new(npc_name: String, lines: Vec<DialogLine>) -> Self {
        Self {
            npc_name,
            lines,
            current_line: 0,
        }
    }
}

#[derive(Debug)]
pub struct ShopState {
    pub shop: Shop,
    pub items: Vec<Item>,
}

impl ShopState {
    pub fn new(shop: Shop, items: Vec<Item>) -> Self {
        Self { shop, items }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopMode {
    Buy,
    Sell,
    Select,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explore_ui_maps_ok_to_npc_interact_with_fallback_action() {
        let ui = ExploreUiState::default();
        let events = ui.events_for_key(InputKey::Ok);
        assert!(matches!(
            events.as_slice(),
            [UiEvent::ExploreInput(InputKey::Ok)]
        ));
    }

    #[test]
    fn menu_ui_maps_up_down_ok_keys() {
        let ui = MenuUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Up),
            Some(UiEvent::MenuInput(InputKey::Up))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Down),
            Some(UiEvent::MenuInput(InputKey::Down))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::MenuInput(InputKey::Ok))
        ));
    }

    #[test]
    fn pause_menu_ui_maps_back_and_zero_to_back_intent() {
        let ui = PauseMenuUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::PauseMenuInput(InputKey::Back))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Key0),
            Some(UiEvent::PauseMenuInput(InputKey::Key0))
        ));
    }

    #[test]
    fn inventory_ui_maps_expected_keys() {
        let ui = InventoryUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Up),
            Some(UiEvent::InventoryInput(InputKey::Up))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Down),
            Some(UiEvent::InventoryInput(InputKey::Down))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::InventoryInput(InputKey::Ok))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::InventoryInput(InputKey::Back))
        ));
    }

    #[test]
    fn dialog_ui_maps_expected_keys() {
        let ui = DialogUiState::default();

        assert!(matches!(
            ui.event_for_key(InputKey::Ok),
            Some(UiEvent::DialogInput(InputKey::Ok))
        ));
        assert!(matches!(
            ui.event_for_key(InputKey::Back),
            Some(UiEvent::DialogInput(InputKey::Back))
        ));
    }
}
