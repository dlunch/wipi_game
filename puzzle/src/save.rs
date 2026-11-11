use anyhow::{Result, anyhow};
use wipi::fs::{File, OpenMode};

const SAVE_FILE_NAME: &str = "save";

pub fn save_stage(stage: usize) -> Result<()> {
    let mut file = File::open(SAVE_FILE_NAME, OpenMode::WriteTruncate)
        .map_err(|e| anyhow!("failed to open save file: {:?}", e))?;
    file.write(&[stage as u8])
        .map_err(|e| anyhow!("failed to write save file: {:?}", e))?;
    Ok(())
}

pub fn load_stage() -> Result<usize> {
    // No save file yet on the first run.
    let Ok(mut file) = File::open(SAVE_FILE_NAME, OpenMode::ReadOnly) else {
        return Ok(0);
    };
    let mut stage = [0u8; 1];
    file.read(&mut stage)
        .map_err(|e| anyhow!("failed to read save file: {:?}", e))?;
    Ok(stage[0] as usize)
}
