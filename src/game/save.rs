use alloc::string::String;
use alloc::vec::Vec;
use core::str;

use anyhow::{Result, anyhow};
use wipi::database::{Database, OpenMode};

use super::{CharacterState, WorldState};
use crate::data::QuestProgress;
use crate::game::save_schema;

const SAVE_DB_NAME: &str = "save";

pub fn save_game(session: &WorldState) -> Result<()> {
    let data = save_schema::serialize(&session.leader, &session.quests, &session.opened_treasures);

    let mut db = Database::open(SAVE_DB_NAME, OpenMode::ReadWrite)
        .map_err(|e| anyhow!("failed to open save db: {:?}", e))?;
    db.write(data.as_bytes())
        .map_err(|e| anyhow!("failed to write save db: {:?}", e))?;
    Ok(())
}

pub fn load_game(
    character: &mut CharacterState,
    quests: &mut Vec<QuestProgress>,
    opened_treasures: &mut Vec<(String, usize, usize)>,
) -> Result<bool> {
    let db = Database::open(SAVE_DB_NAME, OpenMode::ReadOnly)
        .map_err(|e| anyhow!("failed to open save db: {:?}", e))?;
    let mut buf = [0u8; 8192];
    let len = db
        .read(&mut buf)
        .map_err(|e| anyhow!("failed to read save db: {:?}", e))?;
    if len >= buf.len() {
        return Err(anyhow!("save data too large ({} bytes)", len));
    }
    let data = str::from_utf8(&buf[..len])?;
    Ok(save_schema::deserialize(
        data,
        character,
        quests,
        opened_treasures,
    ))
}

pub fn has_save_data() -> bool {
    Database::open(SAVE_DB_NAME, OpenMode::ReadOnly).is_ok()
}
