use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::input::Bind;

pub use bevy_persistent::prelude::*;

pub const FONT_MULTIPLIERS: [f32; 3] = [2.0, 1.0, 0.8];
pub const FONT_SIZES: [f32; 5] = [16.0, 20.0, 24.0, 28.0, 32.0];

// ······
// Plugin
// ······

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_persistence);
    }
}

// ·········
// Resources
// ·········

#[derive(Serialize, Deserialize, Reflect)]
pub struct FontSize {
    pub title: f32,
    pub text: f32,
    pub button_text: f32,
}

impl Default for FontSize {
    fn default() -> Self {
        Self {
            title: FONT_SIZES[2] * FONT_MULTIPLIERS[0],
            text: FONT_SIZES[2] * FONT_MULTIPLIERS[1],
            button_text: FONT_SIZES[2] * FONT_MULTIPLIERS[2],
        }
    }
}

#[derive(Serialize, Deserialize, Reflect)]
pub struct ColorPalette {
    pub light: Color,
    pub mid: Color,
    pub dark: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            light: Color::rgb(225.0 / 255.0, 224.0 / 255.0, 160.0 / 255.0),
            mid: Color::rgb(175.0 / 255.0, 224.0 / 255.0, 150.0 / 255.0),
            dark: Color::rgb(18.0 / 255.0, 13.0 / 255.0, 26.0 / 255.0),
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Reflect, Default)]
pub struct GameOptions {
    pub font_size: FontSize,
    pub color: ColorPalette,
}

// Keybinds

#[derive(Resource, Serialize, Deserialize, Reflect)]
pub struct Keybinds {
    pub interact: Vec<Bind>,
    pub pause: Vec<Bind>,
}

impl Keybinds {
    pub fn all(&self) -> Vec<&Bind> {
        self.iter_fields()
            .filter_map(|f| f.downcast_ref::<Vec<Bind>>())
            .flatten()
            .collect()
    }
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            interact: vec![
                Bind::Key(KeyCode::E),
                Bind::Mouse(MouseButton::Left),
                Bind::Gamepad(GamepadButtonType::East),
            ],
            pause: vec![
                Bind::Key(KeyCode::Escape),
                Bind::Gamepad(GamepadButtonType::Start),
            ],
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Reflect, Default)]
pub struct GameScore {
    #[serde(skip_serializing, skip_deserializing)]
    pub score: u32,
    #[serde(skip_serializing, skip_deserializing)]
    pub last_score: u32,
    pub best_score: u32,
}

// ·······
// Systems
// ·······

fn init_persistence(mut cmd: Commands) {
    #[cfg(not(target_arch = "wasm32"))]
    let config_dir = Path::new(".data");
    #[cfg(target_arch = "wasm32")]
    let config_dir = Path::new("session");

    cmd.insert_resource(
        Persistent::<GameOptions>::builder()
            .name("options")
            .format(StorageFormat::Toml)
            .path(config_dir.join("options.toml"))
            .default(GameOptions::default())
            .revertible(true)
            .revert_to_default_on_deserialization_errors(true)
            .build()
            .expect("Failed to initialize game options"),
    );

    cmd.insert_resource(
        Persistent::<Keybinds>::builder()
            .name("keybinds")
            .format(StorageFormat::Toml)
            .path(config_dir.join("keybinds.toml"))
            .default(Keybinds::default())
            .revertible(true)
            .revert_to_default_on_deserialization_errors(true)
            .build()
            .expect("Failed to initialize keybinds"),
    );

    cmd.insert_resource(
        Persistent::<GameScore>::builder()
            .name("score")
            .format(StorageFormat::Toml)
            .path(config_dir.join("score.toml"))
            .default(GameScore::default())
            .revertible(true)
            .revert_to_default_on_deserialization_errors(true)
            .build()
            .expect("Failed to initialize game score"),
    );
}
