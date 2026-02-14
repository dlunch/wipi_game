use alloc::string::String;

use crate::data::Dialog;

#[derive(Debug)]
pub struct DialogState {
    pub npc_name: String,
    pub dialog_id: String,
    pub current_line: usize,
}

impl DialogState {
    pub fn new(npc_name: String, dialog: &Dialog) -> Self {
        Self {
            npc_name,
            dialog_id: dialog.id.clone(),
            current_line: 0,
        }
    }
}
