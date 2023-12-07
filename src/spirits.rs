use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::seq::IteratorRandom;

use crate::{
    load::GameAssets,
    tilemap::{get_neighbours, pos_to_tile, tile_to_pos, EndTile, PathTile, StartTile},
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
            .add_systems(
                Update,
                (spawn_spirit, move_spirit).run_if(in_state(GameState::Play)),
            );
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
        Self(Timer::from_seconds(0.5, TimerMode::Repeating))
    }
}

// ··········
// Components
// ··········

#[derive(Component, Default)]
pub struct Spirit {
    prev: Option<TilePos>,
}

// ·······
// Systems
// ·······

fn spawn_spirit(
    mut cmd: Commands,
    mut timer: ResMut<SpawnTimer>,
    time: Res<Time>,
    assets: Res<GameAssets>,
    start: Query<(&TilePos, &StartTile)>,
    tilemap: Query<(&TilemapGridSize, &TilemapType, &Transform)>,
    spirits: Query<&Transform, With<Spirit>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if let Ok((grid_size, map_type, trans)) = tilemap.get_single() {
            if let Ok((start_pos, start_tile)) = start.get_single() {
                // Don't spawn entities if the path is not complete
                if !start_tile.complete {
                    return;
                }

                // Calculate the spawn position
                // If there is already another entity, don't spawn
                let pos = tile_to_pos(start_pos, grid_size, map_type, trans).extend(1.);
                for trans in spirits.iter() {
                    if pos == trans.translation {
                        return;
                    }
                }

                // Spawn the entity at the start of the path
                cmd.spawn((
                    SpriteBundle {
                        texture: assets.bevy_icon.clone(),
                        transform: Transform::from_translation(pos).with_scale(Vec3::splat(0.3)),
                        ..default()
                    },
                    Spirit::default(),
                ));
            }
        }
    }
}

fn move_spirit(
    mut cmd: Commands,
    mut timer: ResMut<MoveTimer>,
    time: Res<Time>,
    mut spirit: Query<(Entity, &mut Transform, &mut Spirit)>,
    paths: Query<(&TilePos, &PathTile)>,
    end: Query<Entity, With<EndTile>>,
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
            for (spirit_entity, mut trans, spirit) in spirit.iter_mut() {
                if let Some(tile_pos) = pos_to_tile(
                    &trans.translation.xy(),
                    map_size,
                    grid_size,
                    map_type,
                    map_trans,
                ) {
                    // Get the score of the current path
                    let mut curr = std::f32::MAX;
                    if let Some(entity) = storage.get(&tile_pos) {
                        if let Ok((_, path)) = paths.get(entity) {
                            curr = path.distance;
                        }
                    };

                    // Get and update the path the entity was the previous frame
                    let prev = spirit.prev.unwrap_or(tile_pos);

                    // Get the possible next tiles (they must be paths)
                    let neighbour_list = get_neighbours(&tile_pos, map_size);
                    let mut neighbours = neighbour_list
                        .iter()
                        .filter_map(|pos| storage.get(pos))
                        .filter_map(|entity| {
                            if let Ok((pos, path)) = paths.get(entity) {
                                Some((pos, path))
                            } else {
                                None
                            }
                        })
                        .peekable();

                    // If there are no surrounding paths, check if the spirit is on a path
                    // If it is not, despawn
                    if neighbours.peek().is_none() {
                        if let Some(entity) = storage.get(&tile_pos) {
                            if paths.get(entity).is_err() {
                                cmd.get_entity(spirit_entity).unwrap().despawn_recursive();
                            }
                        }
                    }

                    // Choose the next tile to move to
                    // For this, it must have a path score less than the current one, or else it will stay put
                    let mut next = neighbours
                        .clone()
                        .filter(|(pos, path)| **pos != prev && path.distance < curr)
                        .peekable();

                    // It favours that the tile is not the previous one, but if there is no other option, it will move back
                    // TODO:

                    // From the possible next tiles, it chooses the one with the lowest score
                    if next.peek().is_none() {
                        continue;
                    }
                    let min_score = next
                        .peek()
                        .iter()
                        .min_by(|(_, a), (_, b)| {
                            a.distance
                                .partial_cmp(&b.distance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .unwrap()
                        .1
                        .distance;
                    let next = next
                        .filter(|(_, path)| path.distance == min_score)
                        .map(|(pos, _)| pos)
                        .collect::<Vec<_>>();

                    // If two tiles have the same score, it chooses one at random
                    let next = next.iter().choose(&mut rand::thread_rng());

                    // If there is no next tile, you have been stuck, despawn
                    if next.is_none() && prev == tile_pos {
                        cmd.get_entity(spirit_entity).unwrap().despawn_recursive();
                    }

                    // Move to next tile
                    if let Some(next) = next {
                        let new_pos = tile_to_pos(next, grid_size, map_type, map_trans);

                        // TODO: Check if there is a spirit on the next tile
                        // I'm going to need to break this down into subsistems to be able to
                        // borrow the spirit list
                        // That or register the spirit in the tile is in (but that doesn't let me
                        // do fancy collisions later
                        // This needs reworking anyways so that the movement is not tile by tile

                        trans.translation = new_pos.extend(1.);

                        // If the next tile has the end as a neighbour, despawn
                        for neighbour in get_neighbours(next, map_size) {
                            if let Some(entity) = storage.get(&neighbour) {
                                if end.get(entity).is_ok() {
                                    cmd.get_entity(spirit_entity).unwrap().despawn_recursive();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
