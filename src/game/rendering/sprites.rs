use wipi::image::Image;

pub struct SpriteAtlas {
    pub player: Option<Image>,
    pub npc: Option<Image>,
    pub enemy: Option<Image>,
}

impl SpriteAtlas {
    pub fn load_default() -> Self {
        Self {
            player: Image::new("images/player.png").ok(),
            npc: Image::new("images/npc.png").ok(),
            enemy: Image::new("images/enemy.png").ok(),
        }
    }
}
