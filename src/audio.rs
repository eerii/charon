use crate::{load::GameAssets, GameState};
use bevy::prelude::*;
use bevy_kira_audio::{prelude::AudioPlugin as KiraAudioPlugin, prelude::*};

// ······
// Plugin
// ······

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(KiraAudioPlugin)
            .add_systems(OnEnter(GameState::Play), init_music)
            .add_systems(OnExit(GameState::Play), pause_music)
            .init_resource::<MusicHandles>();
    }
}

// ·········
// Resources
// ·········

#[derive(Resource, Default)]
struct MusicHandles {
    ambient_music: Option<Handle<AudioInstance>>,
}

// ·······
// Systems
// ·······

fn init_music(
    assets: Res<GameAssets>,
    audio: Res<Audio>,
    mut handles: ResMut<MusicHandles>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    match handles.ambient_music.clone() {
        Some(h) => {
            if let Some(inst) = instances.get_mut(h) {
                inst.resume(default());
            }
        }
        None => {
            handles.ambient_music = Some(
                audio
                    .play(assets.music.clone())
                    .looped()
                    .with_volume(0.1)
                    .handle(),
            );
        }
    }
}

fn pause_music(handles: Res<MusicHandles>, mut instances: ResMut<Assets<AudioInstance>>) {
    if let Some(handle) = handles.ambient_music.clone() {
        if let Some(inst) = instances.get_mut(handle) {
            inst.pause(default());
        }
    }
}
