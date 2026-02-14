use alloc::string::String;
use alloc::vec::Vec;

use crate::data::{Dialog, DialogAction, DialogLine};

#[derive(Debug)]
pub struct DialogState {
    pub npc_name: String,
    pub lines: Vec<DialogLine>,
    pub current_line: usize,
}

impl DialogState {
    pub fn new(npc_name: String, dialog: &Dialog) -> Self {
        Self {
            npc_name,
            lines: dialog.lines.clone(),
            current_line: 0,
        }
    }

    pub fn current_text(&self) -> Option<&str> {
        self.lines.get(self.current_line).map(|l| l.text.as_str())
    }

    pub fn advance(&mut self) -> bool {
        if self.current_line + 1 < self.lines.len() {
            self.current_line += 1;
            true
        } else {
            false
        }
    }

    pub fn current_action(&self) -> Option<&DialogAction> {
        self.lines
            .get(self.current_line)
            .and_then(|l| l.action.as_ref())
    }
}
