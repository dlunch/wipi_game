use alloc::vec::Vec;
use alloc::{rc::Rc, string::String};

use anyhow::Result;

use crate::game::{GameData, GameEvent, GameState, SessionState, ShopState, UiState};

pub struct ResolveContext<'a> {
    pub state: &'a GameState,
    pub data: &'a mut Rc<GameData>,
    pub session: Option<&'a SessionState>,
    pub ui: &'a UiState,
}

impl<'a> ResolveContext<'a> {
    pub fn data(&self) -> &GameData {
        self.data
    }
}

pub struct ApplyContext<'a> {
    pub state: &'a mut GameState,
    pub data: &'a Rc<GameData>,
    pub session: &'a mut Option<SessionState>,
    pub ui: &'a mut UiState,
}

impl<'a> ApplyContext<'a> {
    pub fn ui_mut(&mut self) -> &mut UiState {
        self.ui
    }

    pub fn data_rc(&self) -> Rc<GameData> {
        Rc::clone(self.data)
    }

    pub fn transition_to(&mut self, next: GameState) {
        if next.requires_session() && self.session.is_none() {
            *self.state = GameState::Error(alloc::format!(
                "Missing session for state transition: {:?}",
                next
            ));
            return;
        }

        if self.state.can_transition_to(&next) {
            *self.state = next;
            if !self.state.requires_session() {
                *self.session = None;
            }
            return;
        }

        *self.state = GameState::Error(alloc::format!(
            "Invalid state transition: {:?} -> {:?}",
            self.state,
            next
        ));
    }

    pub fn set_error(&mut self, message: String) {
        *self.state = GameState::Error(message);
    }

    pub fn session(&self) -> Option<&SessionState> {
        self.session.as_ref()
    }

    pub fn session_mut(&mut self) -> Option<&mut SessionState> {
        self.session.as_mut()
    }

    pub fn open_shop_by_id(&mut self, shop_id: &str) -> bool {
        let Some(shop) = self.data.find_shop(shop_id).cloned() else {
            return false;
        };
        let shop_items = self.data.get_shop_items(&shop);
        self.ui.shop.open(ShopState::new(shop, shop_items));
        self.transition_to(GameState::Shop);
        true
    }
}

pub trait DomainEventResolver {
    fn handles(&self, event: &GameEvent) -> bool;
    fn resolve(&self, ctx: &mut ResolveContext<'_>, event: &GameEvent) -> Result<Vec<GameEvent>>;
}

pub trait DomainEventApplier {
    fn handles(&self, event: &GameEvent) -> bool;
    fn apply(&self, ctx: &mut ApplyContext<'_>, event: &GameEvent) -> Result<()>;
}
