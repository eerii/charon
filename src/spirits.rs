use bevy::prelude::*;

use crate::GameState;

// ······
// Plugin
// ······

pub struct SpiritPlugin;

impl Plugin for SpiritPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnTimer::default())
            .add_systems(Update, spawn_spirit.run_if(in_state(GameState::Play)));
    }
}

// ·········
// Resources
// ·········

#[derive(Resource)]
struct SpawnTimer(Timer);

impl Default for SpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

// ··········
// Components
// ··········

#[derive(Component)]
pub struct Spirit;

// ·······
// Systems
// ·······

fn spawn_spirit(mut _cmd: Commands, mut timer: ResMut<SpawnTimer>, time: Res<Time>) {
    if timer.0.tick(time.delta()).just_finished() {
        info!("Spawn spirit");
        //cmd.spawn((Spirit, Transform::default()));
    }
}
