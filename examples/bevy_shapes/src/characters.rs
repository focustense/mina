use bevy::prelude::*;
use enum_map::Enum;

const SPRITE_SHEET_COLUMNS: usize = 6;
const SPRITE_SHEET_ROWS: usize = 7;

#[derive(Clone, Component, Copy, Enum, Eq, PartialEq)]
pub enum Character {
    Caveman,
    Eggshell,
    Girl,
    Lion,
}

#[derive(Resource)]
pub struct CharacterSprites {
    caveman: Handle<TextureAtlas>,
    eggshell: Handle<TextureAtlas>,
    girl: Handle<TextureAtlas>,
    lion: Handle<TextureAtlas>,
}

impl CharacterSprites {
    pub fn create(&self, character: Character) -> CharacterBundle {
        let texture_atlas = match character {
            Character::Caveman => self.caveman.clone(),
            Character::Eggshell => self.eggshell.clone(),
            Character::Girl => self.girl.clone(),
            Character::Lion => self.lion.clone(),
        };
        CharacterBundle {
            character,
            sprite_sheet_bundle: SpriteSheetBundle {
                texture_atlas,
                sprite: TextureAtlasSprite {
                    index: 0,
                    color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                ..default()
            }
        }
    }
}

impl FromWorld for CharacterSprites {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        let caveman = load_texture_atlas(assets, "caverman.png", 97, 71);
        let eggshell = load_texture_atlas(assets, "egg-shell.png", 107, 71);
        let girl = load_texture_atlas(assets, "girl-2.png", 99, 74);
        let lion = load_texture_atlas(assets, "lion.png", 120, 83);
        let mut texture_atlases = world.resource_mut::<Assets<TextureAtlas>>();
        Self {
            caveman: texture_atlases.add(caveman),
            eggshell: texture_atlases.add(eggshell),
            girl: texture_atlases.add(girl),
            lion: texture_atlases.add(lion),
        }
    }
}

#[derive(Bundle)]
pub struct CharacterBundle {
    character: Character,
    sprite_sheet_bundle: SpriteSheetBundle,
}

fn load_texture_atlas(assets: &AssetServer, path: &str, width: u32, height: u32) -> TextureAtlas {
    TextureAtlas::from_grid(
        assets.load(path),
        Vec2::new(width as f32, height as f32),
        SPRITE_SHEET_COLUMNS,
        SPRITE_SHEET_ROWS,
        None,
        None,
    )
}
