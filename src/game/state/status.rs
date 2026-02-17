#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StatusState {
    pub poison_timer: u32,
    pub stun_timer: u32,
    pub armor_break_timer: u32,
}
