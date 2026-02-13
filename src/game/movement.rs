use wipi::event::KeyCode;

use super::Player;
use super::combat::CombatSystem;
use crate::data::{Map, Npc};

const MOVE_COOLDOWN: u32 = 5;

pub enum MovementIntent {
    DirectionPressed(KeyCode),
    KeyReleased(KeyCode),
    Tick,
}

pub struct MovementContext<'a> {
    pub player: &'a mut Player,
    pub map: &'a Map,
    pub combat: &'a CombatSystem,
    pub npcs: &'a [Npc],
}

#[derive(Default)]
pub struct MovementController {
    pub pressed_direction: Option<KeyCode>,
    pub move_cooldown: u32,
}

impl MovementController {
    pub fn reduce(&mut self, intent: MovementIntent, ctx: Option<MovementContext<'_>>) -> bool {
        match intent {
            MovementIntent::DirectionPressed(key) => {
                self.on_direction_pressed(key);
                false
            }
            MovementIntent::KeyReleased(key) => {
                self.on_key_released(key);
                false
            }
            MovementIntent::Tick => {
                let Some(ctx) = ctx else {
                    return false;
                };
                self.update(ctx.player, ctx.map, ctx.combat, ctx.npcs)
            }
        }
    }

    pub fn on_direction_pressed(&mut self, key: KeyCode) {
        self.pressed_direction = Some(key);
        self.move_cooldown = 0;
    }

    pub fn on_key_released(&mut self, key: KeyCode) {
        if self.pressed_direction == Some(key) {
            self.pressed_direction = None;
        }
    }

    pub fn update(
        &mut self,
        player: &mut Player,
        map: &Map,
        combat: &CombatSystem,
        npcs: &[Npc],
    ) -> bool {
        if self.move_cooldown > 0 {
            self.move_cooldown -= 1;
            return false;
        }

        let Some(key) = self.pressed_direction else {
            return false;
        };

        let moved = self.try_move(player, map, combat, npcs, key);
        self.move_cooldown = MOVE_COOLDOWN;
        moved
    }

    fn try_move(
        &self,
        player: &mut Player,
        map: &Map,
        combat: &CombatSystem,
        npcs: &[Npc],
        key: KeyCode,
    ) -> bool {
        let (dx, dy) = match key {
            KeyCode::Up => (0, -1),
            KeyCode::Down => (0, 1),
            KeyCode::Left => (-1, 0),
            KeyCode::Right => (1, 0),
            _ => return false,
        };

        player.set_facing(dx, dy);

        if !player.can_move(map, dx, dy) {
            return false;
        }

        let Some(new_x) = player.x.checked_add_signed(dx as isize) else {
            return false;
        };
        let Some(new_y) = player.y.checked_add_signed(dy as isize) else {
            return false;
        };

        if combat.enemy_at(new_x, new_y) {
            return false;
        }

        if Self::npc_at(npcs, &player.current_map_id, new_x, new_y) {
            return false;
        }

        player.move_by(dx, dy);
        true
    }

    fn npc_at(npcs: &[Npc], map_id: &str, x: usize, y: usize) -> bool {
        npcs.iter()
            .any(|npc| npc.map_id == map_id && npc.x == x && npc.y == y)
    }
}
