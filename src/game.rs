use bevy::{prelude::*, render::view::RenderLayers};

use crate::GameState;

// TODO: End screen (win/lose)
// TODO: Dialogues
// TODO: Add sprites
// TODO: Add animations

pub struct CharonPlugin;

impl Plugin for CharonPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameScore::default())
            .add_systems(
                OnEnter(GameState::Play),
                (init_game.run_if(run_once()), resume_game),
            )
            .add_systems(OnExit(GameState::Play), pause_game);
    }
}

// ·········
// Resources
// ·········

#[derive(Resource, Default)]
pub struct GameScore {
    pub score: u32,
}

// ··········
// Components
// ··········

#[derive(Component)]
pub struct GameCam;

// ·······
// Systems
// ·······

pub fn init_game(mut cmd: Commands) {
    // Camera
    cmd.spawn((Camera2dBundle::default(), RenderLayers::layer(0), GameCam));
}

pub fn resume_game(mut cam: Query<&mut Camera, With<GameCam>>) {
    for mut cam in cam.iter_mut() {
        cam.is_active = true;
    }
}

pub fn pause_game(mut cam: Query<&mut Camera, With<GameCam>>) {
    for mut cam in cam.iter_mut() {
        cam.is_active = false;
    }
}
