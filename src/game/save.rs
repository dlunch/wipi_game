use anyhow::Result;
use wipi::database::{Database, OpenMode};

use super::PlayerState;
use crate::game::save_schema;

const SAVE_DB_NAME: &str = "save";

pub fn save_game(player: &PlayerState) -> Result<()> {
    let data = save_schema::serialize(player);

    let mut db = Database::open(SAVE_DB_NAME, OpenMode::ReadWrite)
        .map_err(|e| anyhow::anyhow!("failed to open save db: {:?}", e))?;
    db.write(data.as_bytes())
        .map_err(|e| anyhow::anyhow!("failed to write save db: {:?}", e))?;
    Ok(())
}

pub fn load_game(player: &mut PlayerState) -> Result<bool> {
    let db = Database::open(SAVE_DB_NAME, OpenMode::ReadOnly)
        .map_err(|e| anyhow::anyhow!("failed to open save db: {:?}", e))?;
    let mut buf = [0u8; 4096];
    let len = db
        .read(&mut buf)
        .map_err(|e| anyhow::anyhow!("failed to read save db: {:?}", e))?;
    let data = core::str::from_utf8(&buf[..len])?;
    Ok(save_schema::deserialize(data, player))
}

pub fn has_save_data() -> bool {
    Database::open(SAVE_DB_NAME, OpenMode::ReadOnly).is_ok()
}
