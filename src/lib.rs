pub mod audio;
pub mod config;
mod debug;
pub mod input;
pub mod load;
mod menu;
pub mod tilemap;
pub mod ui;

use bevy::{prelude::*, render::view::RenderLayers, sprite::MaterialMesh2dBundle};
use bevy_persistent::Persistent;
use config::GameOptions;

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
        ));

        #[cfg(debug_assertions)]
        {
            app.add_plugins(debug::DebugPlugin);
            debug::save_schedule(app);
        }

        app.add_systems(OnEnter(GameState::Play), init_game.run_if(run_once()));
    }
}

// TODO: Move this somewhere where it makes sense

#[derive(Component)]
pub struct GameCamera;

fn init_game(
    mut cmd: Commands,
    opts: Res<Persistent<GameOptions>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Camera
    cmd.spawn((
        Camera2dBundle::default(),
        RenderLayers::layer(0),
        GameCamera,
    ));

    // Background
    cmd.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
        transform: Transform::from_xyz(0., 0., -10.).with_scale(Vec3::new(1080., 720., 1.)),
        material: materials.add(ColorMaterial::from(opts.color.dark)),
        ..default()
    });
}
