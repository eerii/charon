use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    load::GameAssets,
    tilemap::{get_neighbours, pos_to_tile, tile_to_pos, PathTile, StartTile},
    GameState,
};

// ······
// Plugin
// ······

pub struct SpiritPlugin;

impl Plugin for SpiritPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnTimer::default())
            .insert_resource(MoveTimer::default())
            .add_systems(Update, move_spirit.run_if(in_state(GameState::Play)));
    }
}

// ·········
// Resources
// ·········

#[derive(Resource)]
struct SpawnTimer(Timer);

impl Default for SpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2.0, TimerMode::Repeating))
    }
}

#[derive(Resource)]
struct MoveTimer(Timer);

impl Default for MoveTimer {
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

fn _spawn_spirit(
    mut cmd: Commands,
    mut timer: ResMut<SpawnTimer>,
    time: Res<Time>,
    assets: Res<GameAssets>,
    start: Query<&TilePos, With<StartTile>>,
    tilemap: Query<(&TilemapGridSize, &TilemapType, &Transform)>,
    spirits: Query<&Transform, With<Spirit>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if let Ok((grid_size, map_type, trans)) = tilemap.get_single() {
            if let Ok(start) = start.get_single() {
                let pos = tile_to_pos(start, grid_size, map_type, trans).extend(1.);

                for trans in spirits.iter() {
                    if pos == trans.translation {
                        continue;
                    }
                }

                cmd.spawn((
                    SpriteBundle {
                        texture: assets.bevy_icon.clone(),
                        transform: Transform::from_translation(pos).with_scale(Vec3::splat(0.3)),
                        ..default()
                    },
                    Spirit,
                ));
            }
        }
    }
}

fn move_spirit(
    mut timer: ResMut<MoveTimer>,
    time: Res<Time>,
    mut spirit: Query<&mut Transform, With<Spirit>>,
    paths: Query<&TilePos, With<PathTile>>,
    tilemap: Query<
        (
            &TilemapSize,
            &TilemapGridSize,
            &TilemapType,
            &TileStorage,
            &Transform,
        ),
        Without<Spirit>,
    >,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if let Ok((map_size, grid_size, map_type, storage, map_trans)) = tilemap.get_single() {
            for mut trans in spirit.iter_mut() {
                if let Some(tile_pos) = pos_to_tile(
                    &trans.translation.xy(),
                    map_size,
                    grid_size,
                    map_type,
                    map_trans,
                ) {
                    for neighbour in get_neighbours(&tile_pos, map_size) {
                        if let Some(entity) = storage.get(&neighbour) {
                            // TODO: Get neighbours that are paths and get lower
                            // TODO: Check collisions
                            if let Ok(path) = paths.get(entity) {
                                let new_pos = tile_to_pos(&path, grid_size, map_type, map_trans);
                                trans.translation = new_pos.extend(1.);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
