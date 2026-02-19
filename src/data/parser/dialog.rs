use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use anyhow::{Result, anyhow};

use super::parse_int;
use crate::data::types::{Dialog, DialogAction, DialogCondition, DialogLine};

pub fn parse_dialogs(data: &str) -> Result<Vec<Dialog>> {
    let mut dialogs = Vec::new();
    let mut current: Option<DialogBuilder> = None;

    for raw_line in data.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("@DIALOG:") {
            if let Some(builder) = current.take() {
                dialogs.push(builder.build());
            }
            current = Some(DialogBuilder::new(rest.to_string()));
        } else if line == "@END" {
            if let Some(builder) = current.take() {
                dialogs.push(builder.build());
            }
        } else if !line.is_empty()
            && !line.starts_with('#')
            && let Some(ref mut builder) = current
        {
            builder.add_line(line)?;
        }
    }

    if let Some(builder) = current {
        dialogs.push(builder.build());
    }

    Ok(dialogs)
}

struct DialogBuilder {
    id: String,
    lines: Vec<DialogLine>,
}

impl DialogBuilder {
    fn new(id: String) -> Self {
        Self {
            id,
            lines: Vec::new(),
        }
    }

    fn add_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.splitn(3, ':').collect();

        let (condition, action, text) = if parts.len() == 3 {
            (
                Self::parse_condition(parts[0])?,
                Self::parse_action(parts[1])?,
                parts[2].to_string(),
            )
        } else if parts.len() == 2 {
            (None, Self::parse_action(parts[0])?, parts[1].to_string())
        } else {
            (None, None, line.to_string())
        };

        self.lines.push(DialogLine {
            text,
            condition,
            action,
        });
        Ok(())
    }

    fn parse_condition(s: &str) -> Result<Option<DialogCondition>> {
        let parts: Vec<&str> = s.split('=').collect();
        if parts.len() != 2 {
            return Ok(None);
        }
        match parts[0] {
            "HAS_QUEST" => Ok(Some(DialogCondition::HasQuest(parts[1].to_string()))),
            "QUEST_DONE" => Ok(Some(DialogCondition::QuestComplete(parts[1].to_string()))),
            "HAS_ITEM" => Ok(Some(DialogCondition::HasItem(parts[1].to_string()))),
            "HAS_GOLD" => {
                let amount = parse_int(parts[1], "HAS_GOLD amount", s)?;
                Ok(Some(DialogCondition::HasGold(amount)))
            }
            _ => Ok(None),
        }
    }

    fn parse_action(s: &str) -> Result<Option<DialogAction>> {
        let parts: Vec<&str> = s.split('=').collect();
        if parts.is_empty() {
            return Ok(None);
        }
        match parts[0] {
            "GIVE_QUEST" => Ok(parts
                .get(1)
                .map(|id| DialogAction::GiveQuest(id.to_string()))),
            "COMPLETE_QUEST" => Ok(parts
                .get(1)
                .map(|id| DialogAction::CompleteQuest(id.to_string()))),
            "GIVE_ITEM" => Ok(parts
                .get(1)
                .map(|id| DialogAction::GiveItem(id.to_string()))),
            "TAKE_ITEM" => Ok(parts
                .get(1)
                .map(|id| DialogAction::TakeItem(id.to_string()))),
            "GIVE_GOLD" => {
                let val = parts
                    .get(1)
                    .ok_or_else(|| anyhow!("GIVE_GOLD missing amount in: {s}"))?;
                let amount = parse_int(val, "GIVE_GOLD amount", s)?;
                Ok(Some(DialogAction::GiveGold(amount)))
            }
            "TAKE_GOLD" => {
                let val = parts
                    .get(1)
                    .ok_or_else(|| anyhow!("TAKE_GOLD missing amount in: {s}"))?;
                let amount = parse_int(val, "TAKE_GOLD amount", s)?;
                Ok(Some(DialogAction::TakeGold(amount)))
            }
            "OPEN_SHOP" => Ok(parts
                .get(1)
                .map(|id| DialogAction::OpenShop(id.to_string()))),
            "HEAL" => Ok(Some(DialogAction::Heal)),
            _ => Ok(None),
        }
    }

    fn build(self) -> Dialog {
        Dialog {
            id: self.id,
            lines: self.lines,
        }
    }
}
