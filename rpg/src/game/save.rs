use core::str;

use anyhow::{Result, anyhow};
use wipi::fs::{File, OpenMode};

use crate::game::{save_schema, world::WorldState};

const SAVE_FILE_NAME: &str = "save";

pub fn save_game(session: &WorldState) -> Result<()> {
    let data = save_schema::serialize(session);

    let mut file = File::open(SAVE_FILE_NAME, OpenMode::WriteTruncate)
        .map_err(|e| anyhow!("failed to open save file: {:?}", e))?;
    file.write(data.as_bytes())
        .map_err(|e| anyhow!("failed to write save file: {:?}", e))?;
    Ok(())
}

pub fn load_game(world: &mut WorldState) -> Result<()> {
    let mut file = File::open(SAVE_FILE_NAME, OpenMode::ReadOnly)
        .map_err(|e| anyhow!("failed to open save file: {:?}", e))?;
    let mut buf = [0u8; 8192];
    let len = file
        .read(&mut buf)
        .map_err(|e| anyhow!("failed to read save file: {:?}", e))?;
    if len >= buf.len() {
        return Err(anyhow!("save data too large ({} bytes)", len));
    }
    let data = str::from_utf8(&buf[..len])?;
    save_schema::deserialize(data, world)
}

pub fn has_save_data() -> Result<bool> {
    Ok(File::open(SAVE_FILE_NAME, OpenMode::ReadOnly).is_ok())
}
