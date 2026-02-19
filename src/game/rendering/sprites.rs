use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::str;

use wipi::{image::Image, resource::Resource};

use crate::data::Direction;

#[derive(Clone, Copy)]
pub struct SpriteFrame {
    pub sx: i32,
    pub sy: i32,
    pub w: u32,
    pub h: u32,
}

struct SpriteClip {
    fps: u8,
    looped: bool,
    sheet_id: String,
    frames: Vec<SpriteFrame>,
}

struct AtlasData {
    sheets: BTreeMap<String, Image>,
    clips: BTreeMap<String, SpriteClip>,
}

pub struct SpriteAtlas {
    data: Option<AtlasData>,
}

impl SpriteAtlas {
    pub fn load_default() -> Self {
        let data = Self::load_atlas("images/atlas.meta");
        Self { data }
    }

    pub fn player_frame(
        &self,
        facing: Direction,
        moving: bool,
        tick: u32,
    ) -> Option<(&Image, SpriteFrame)> {
        let clip = match (facing, moving) {
            (Direction::Up, true) => "player_walk_up",
            (Direction::Down, true) => "player_walk_down",
            (Direction::Left, true) => "player_walk_left",
            (Direction::Right, true) => "player_walk_right",
            (Direction::Up, false) => "player_idle_up",
            (Direction::Down, false) => "player_idle_down",
            (Direction::Left, false) => "player_idle_left",
            (Direction::Right, false) => "player_idle_right",
        };
        self.frame_for_clip(clip, tick)
    }

    pub fn npc_frame(&self, tick: u32) -> Option<(&Image, SpriteFrame)> {
        self.frame_for_clip("npc_idle_down", tick)
            .or_else(|| self.frame_for_clip("npc_idle", tick))
    }

    pub fn enemy_frame(&self, tick: u32) -> Option<(&Image, SpriteFrame)> {
        self.frame_for_clip("enemy_idle", tick)
    }

    fn frame_for_clip(&self, name: &str, tick: u32) -> Option<(&Image, SpriteFrame)> {
        let atlas = self.data.as_ref()?;
        let clip = atlas.clips.get(name)?;
        if clip.frames.is_empty() {
            return None;
        }

        let frame = if clip.frames.len() == 1 {
            clip.frames[0]
        } else {
            let fps = clip.fps.max(1) as u32;
            let frame_step = tick.saturating_mul(fps) / 30;
            let idx = if clip.looped {
                (frame_step as usize) % clip.frames.len()
            } else {
                (frame_step as usize).min(clip.frames.len() - 1)
            };
            clip.frames[idx]
        };

        let image = atlas.sheets.get(clip.sheet_id.as_str())?;
        Some((image, frame))
    }

    fn load_atlas(meta_path: &str) -> Option<AtlasData> {
        // Atlas assets are optional until real sprite resources are added.
        // Any load/parse failure intentionally falls back to rectangle rendering.
        let resource = Resource::new(meta_path).ok()?;
        let text = str::from_utf8(resource.read()).ok()?;
        let parsed = parse_meta(text).ok()?;
        let mut sheets = BTreeMap::new();
        for (sheet_id, sheet_path) in parsed.sheets {
            let image = Image::new(sheet_path.as_str()).ok()?;
            sheets.insert(sheet_id, image);
        }
        Some(AtlasData {
            sheets,
            clips: parsed.clips,
        })
    }
}

struct ParsedMeta {
    sheets: BTreeMap<String, String>,
    clips: BTreeMap<String, SpriteClip>,
}

fn parse_meta(text: &str) -> Result<ParsedMeta, ()> {
    let mut sheets = BTreeMap::new();
    let mut default_sheet_id = None;
    let mut tile_w = 16;
    let mut tile_h = 16;
    let mut clips = BTreeMap::new();

    let mut current_name = None;
    let mut current_clip = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(cmd) = parts.next() else {
            continue;
        };

        match cmd {
            "sheet" => {
                let Some(first) = parts.next() else {
                    return Err(());
                };
                let second = parts.next();
                let (sheet_id, sheet_path) = if let Some(path) = second {
                    (String::from(first), String::from(path))
                } else {
                    (String::from("main"), String::from(first))
                };
                if default_sheet_id.is_none() {
                    default_sheet_id = Some(sheet_id.clone());
                }
                sheets.insert(sheet_id, sheet_path);
            }
            "tile" => {
                let Some(w) = parts.next() else {
                    return Err(());
                };
                let Some(h) = parts.next() else {
                    return Err(());
                };
                tile_w = w.parse().map_err(|_| ())?;
                tile_h = h.parse().map_err(|_| ())?;
            }
            "clip" => {
                if let (Some(name), Some(clip)) = (current_name.take(), current_clip.take()) {
                    clips.insert(name, clip);
                }

                let Some(name) = parts.next() else {
                    return Err(());
                };
                let mut sheet_id = default_sheet_id.clone().ok_or(())?;
                let mut fps = 8u8;
                let mut looped = true;
                for token in parts {
                    if let Some(raw_fps) = token.strip_prefix("fps=") {
                        fps = raw_fps.parse().map_err(|_| ())?;
                    } else if let Some(raw_loop) = token.strip_prefix("loop=") {
                        looped = !matches!(raw_loop, "0" | "false" | "False");
                    } else if let Some(raw_sheet) = token.strip_prefix("sheet=") {
                        sheet_id = String::from(raw_sheet);
                    }
                }

                current_name = Some(String::from(name));
                current_clip = Some(SpriteClip {
                    fps,
                    looped,
                    sheet_id,
                    frames: Vec::new(),
                });
            }
            "frame" => {
                let Some(clip) = current_clip.as_mut() else {
                    return Err(());
                };
                let Some(tx) = parts.next() else {
                    return Err(());
                };
                let Some(ty) = parts.next() else {
                    return Err(());
                };
                let tx = tx.parse::<i32>().map_err(|_| ())?;
                let ty = ty.parse::<i32>().map_err(|_| ())?;
                clip.frames.push(SpriteFrame {
                    sx: tx * tile_w as i32,
                    sy: ty * tile_h as i32,
                    w: tile_w,
                    h: tile_h,
                });
            }
            "endclip" => {
                if let (Some(name), Some(clip)) = (current_name.take(), current_clip.take()) {
                    clips.insert(name, clip);
                }
            }
            _ => return Err(()),
        }
    }

    if let (Some(name), Some(clip)) = (current_name.take(), current_clip.take()) {
        clips.insert(name, clip);
    }

    if sheets.is_empty() {
        return Err(());
    }

    Ok(ParsedMeta { sheets, clips })
}
