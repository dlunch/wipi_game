use alloc::{string::ToString, vec::Vec};

use anyhow::{Result, anyhow};

use super::{parse_int, parse_u32};
use crate::data::types::{Dialog, DialogAction, DialogCondition, DialogLine};

pub fn parse_dialogs(data: &str) -> Result<Vec<Dialog>> {
    let mut dialogs = Vec::new();
    let mut current = None::<DialogBuilder>;

    for raw_line in data.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("@DIALOG:") {
            if let Some(builder) = current.take() {
                dialogs.push(builder.build());
            }
            current = Some(DialogBuilder::new(parse_u32(rest, "dialog_id", line)?));
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
    id: u32,
    lines: Vec<DialogLine>,
}

impl DialogBuilder {
    fn new(id: u32) -> Self {
        Self {
            id,
            lines: Vec::new(),
        }
    }

    fn add_line(&mut self, line: &str) -> Result<()> {
        let parts = line.splitn(3, ':').collect::<Vec<_>>();

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
        let parts = s.split('=').collect::<Vec<_>>();
        if parts.len() != 2 {
            return Ok(None);
        }
        match parts[0] {
            "HAS_QUEST" => Ok(Some(DialogCondition::HasQuest(parse_u32(
                parts[1],
                "HAS_QUEST.quest_id",
                s,
            )?))),
            "QUEST_DONE" => Ok(Some(DialogCondition::QuestComplete(parse_u32(
                parts[1],
                "QUEST_DONE.quest_id",
                s,
            )?))),
            "HAS_ITEM" => Ok(Some(DialogCondition::HasItem(parse_u32(
                parts[1],
                "HAS_ITEM.item_id",
                s,
            )?))),
            "HAS_GOLD" => {
                let amount = parse_int(parts[1], "HAS_GOLD amount", s)?;
                Ok(Some(DialogCondition::HasGold(amount)))
            }
            _ => Ok(None),
        }
    }

    fn parse_action(s: &str) -> Result<Option<DialogAction>> {
        let parts = s.split('=').collect::<Vec<_>>();
        if parts.is_empty() {
            return Ok(None);
        }
        match parts[0] {
            "GIVE_QUEST" => Ok(parts
                .get(1)
                .map(|id| parse_u32(id, "GIVE_QUEST.quest_id", s).map(DialogAction::GiveQuest))
                .transpose()?),
            "COMPLETE_QUEST" => Ok(parts
                .get(1)
                .map(|id| {
                    parse_u32(id, "COMPLETE_QUEST.quest_id", s).map(DialogAction::CompleteQuest)
                })
                .transpose()?),
            "GIVE_ITEM" => Ok(parts
                .get(1)
                .map(|id| parse_u32(id, "GIVE_ITEM.item_id", s).map(DialogAction::GiveItem))
                .transpose()?),
            "TAKE_ITEM" => Ok(parts
                .get(1)
                .map(|id| parse_u32(id, "TAKE_ITEM.item_id", s).map(DialogAction::TakeItem))
                .transpose()?),
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
                .map(|id| parse_u32(id, "OPEN_SHOP.shop_id", s).map(DialogAction::OpenShop))
                .transpose()?),
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
