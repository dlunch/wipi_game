use core::str;

use anyhow::{Result, anyhow};
use wipi::database::{Database, OpenMode};

use super::WorldState;
use crate::game::save_schema;

const SAVE_DB_NAME: &str = "save";

pub fn save_game(session: &WorldState) -> Result<()> {
    let data = save_schema::serialize(session);

    let mut db = Database::open(SAVE_DB_NAME, OpenMode::ReadWrite)
        .map_err(|e| anyhow!("failed to open save db: {:?}", e))?;
    db.write(data.as_bytes())
        .map_err(|e| anyhow!("failed to write save db: {:?}", e))?;
    Ok(())
}

pub fn load_game(world: &mut WorldState) -> Result<bool> {
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
    Ok(save_schema::deserialize(data, world))
}

pub fn has_save_data() -> Result<bool> {
    Database::open(SAVE_DB_NAME, OpenMode::ReadOnly)
        .map(|_| true)
        .map_err(|e| anyhow!("failed to open save db: {:?}", e))
}
