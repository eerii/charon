use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;

use crate::{
    game::GameScore,
    load::GameAssets,
    tilemap::{get_neighbours, pos_to_tile, tile_to_pos, EndTile, PathTile, StartTile},
    GameState,
};

const SPIRIT_SPEED: f32 = 150.;
const SPIRIT_SIZE: f32 = 40.;
const MAX_SPIRITS_IN_TILE: u32 = 2;

// ······
// Plugin
// ······

pub struct SpiritPlugin;

impl Plugin for SpiritPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnTimer::default()).add_systems(
            Update,
            (
                spawn_spirit,
                next_tile_spirit,
                spirit_collision,
                move_spirit,
                integrate,
            )
                .run_if(in_state(GameState::Play)),
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
        Self(Timer::from_seconds(0.5, TimerMode::Repeating))
    }
}

// ··········
// Components
// ··········

#[derive(Component)]
pub struct Spirit {
    curr_tile: TilePos,
    next_tile: Option<TilePos>,
    next_pos: Vec2,
    vel: Vec2,
}

impl Spirit {
    pub fn new(curr_tile: TilePos, curr_pos: Vec2) -> Self {
        Self {
            curr_tile,
            next_tile: None,
            next_pos: curr_pos,
            vel: Vec2::ZERO,
        }
    }
}

// ·······
// Systems
// ·······

fn spawn_spirit(
    mut cmd: Commands,
    mut timer: ResMut<SpawnTimer>,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut start: Query<(&TilePos, &StartTile, &mut PathTile)>,
    spirits: Query<&Transform, With<Spirit>>,
    tilemap: Query<(&TilemapGridSize, &TilemapType, &Transform)>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if let Ok((grid_size, map_type, trans)) = tilemap.get_single() {
            if let Ok((start_pos, start_tile, mut start_path)) = start.get_single_mut() {
                // Don't spawn entities if the path is not complete
                if !start_tile.completed_once {
                    return;
                }

                // Calculate the spawn position
                let pos = tile_to_pos(start_pos, grid_size, map_type, trans);

                // If there is already another entity there, don't spawn
                if start_path.count >= 1 {
                    // Check all spirits
                    for spirit in spirits.iter() {
                        if (spirit.translation.xy() - pos).length() < SPIRIT_SIZE {
                            return;
                        }
                    }
                }
                start_path.count += 1;

                // Spawn the entity at the start of the path
                cmd.spawn((
                    SpriteBundle {
                        texture: assets.bevy_icon.clone(),
                        transform: Transform::from_translation(pos.extend(1.))
                            .with_scale(Vec3::splat(0.16)),
                        ..default()
                    },
                    Spirit::new(*start_pos, pos),
                ));
            }
        }
    }
}

fn next_tile_spirit(
    mut cmd: Commands,
    mut score: ResMut<GameScore>,
    mut spirit: Query<(Entity, &Transform, &mut Spirit)>,
    mut paths: Query<(&TilePos, &mut PathTile)>,
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
    if let Ok((map_size, grid_size, map_type, storage, map_trans)) = tilemap.get_single() {
        for (spirit_entity, trans, mut spirit) in spirit.iter_mut() {
            if let Some(tile_pos) = pos_to_tile(
                &trans.translation.xy(),
                map_size,
                grid_size,
                map_type,
                map_trans,
            ) {
                // If it is on the current tile, continue
                if spirit.next_tile.is_some() && spirit.curr_tile == tile_pos {
                    continue;
                }
                spirit.curr_tile = tile_pos;

                // If it arrived at the next tile (or if there is no next tile)
                // Get the next tile
                if spirit.next_tile.is_none() || spirit.next_tile.unwrap() == tile_pos {
                    spirit.next_tile = None;
                    let mut tile_distance = std::f32::INFINITY;

                    // If the spirit is on the end tile, despawn
                    if let Some(entity) = storage.get(&tile_pos) {
                        if end.get(entity).is_ok() {
                            cmd.get_entity(spirit_entity).unwrap().despawn_recursive();
                            score.score += 1;
                            continue;
                        }
                        if let Ok((_, path)) = paths.get(entity) {
                            tile_distance = path.distance;
                        }
                    }

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
                    // Also, we must check that there are not too many entities in this path
                    // From the possible next tiles, it chooses the one with the lowest score
                    let mut rng = rand::thread_rng();
                    let next = neighbours
                        .filter(|(_, path)| {
                            path.distance <= tile_distance && path.count < MAX_SPIRITS_IN_TILE
                        })
                        .map(|(pos, path)| {
                            (
                                *pos,
                                PathTile {
                                    distance: path.distance + rng.gen_range(0.0..0.1),
                                    ..path.clone()
                                },
                            )
                        })
                        .min_by(|(_, a), (_, b)| {
                            a.distance
                                .partial_cmp(&b.distance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    if next.is_none() {
                        continue;
                    }

                    spirit.next_tile = Some(next.unwrap().0);
                    spirit.next_pos =
                        tile_to_pos(&spirit.next_tile.unwrap(), grid_size, map_type, map_trans);

                    // Update counts
                    if let Some(entity) = storage.get(&spirit.curr_tile) {
                        if let Ok((_, mut path)) = paths.get_mut(entity) {
                            path.count = path.count.saturating_sub(1);
                        }
                    }
                    if let Some(entity) = storage.get(&spirit.next_tile.unwrap()) {
                        if end.get(entity).is_err() {
                            if let Ok((_, mut path)) = paths.get_mut(entity) {
                                path.count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn move_spirit(mut spirits: Query<(&mut Spirit, &Transform)>) {
    for (mut spirit, trans) in spirits.iter_mut() {
        // Move towards next tile
        let delta = spirit.next_pos - trans.translation.xy();
        let dir = delta.normalize_or_zero();
        spirit.vel = spirit
            .vel
            .lerp(dir * SPIRIT_SPEED.min(delta.length_squared()), 0.1);
    }
}

fn spirit_collision(mut spirits: Query<(&mut Spirit, &Transform)>) {
    // TODO: Optimize this
    let mut iter = spirits.iter_combinations_mut();
    while let Some([(mut sa, ta), (mut sb, tb)]) = iter.fetch_next() {
        let delta = ta.translation.xy() - tb.translation.xy();
        let dist = delta.length();
        if dist < SPIRIT_SIZE {
            let dir = delta.normalize_or_zero();
            sa.vel = sa.vel.lerp(dir * SPIRIT_SPEED, 3. / dist.max(3.));
            sb.vel = sb.vel.lerp(-dir * SPIRIT_SPEED, 3. / dist.max(3.));
        }
    }
}

fn integrate(mut spirits: Query<(&Spirit, &mut Transform)>, time: Res<Time>) {
    for (spirit, mut trans) in spirits.iter_mut() {
        // Ondulating motion
        let offset = (time.elapsed_seconds() * 1.5).sin();
        let cross = spirit.vel.perp();

        // Update position
        trans.translation += (spirit.vel + cross * offset * 0.1).extend(0.) * time.delta_seconds();
    }
}
