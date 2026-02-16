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
    Tick,
    KeyDown(InputKey),
    KeyUp(InputKey),
}
