mod audio;
mod config;
mod debug;
mod game;
mod input;
mod load;
mod menu;
mod spirits;
mod tilemap;
mod ui;

use bevy::prelude::*;

// Game state
#[derive(States, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Play,
}

// Main game plugin
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>().add_plugins((
            load::LoadPlugin,
            ui::UIPlugin,
            menu::MenuPlugin,
            config::ConfigPlugin,
            input::InputPlugin,
            audio::AudioPlugin,
            tilemap::TilePlugin,
            game::CharonPlugin,
            spirits::SpiritPlugin,
        ));

        #[cfg(debug_assertions)]
        {
            app.add_plugins(debug::DebugPlugin);
            debug::save_schedule(app);
        }
    }
}
