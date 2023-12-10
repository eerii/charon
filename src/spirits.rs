use std::time::Duration;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;

use crate::{
    game::GameScore,
    load::GameAssets,
    tilemap::{get_neighbours, pos_to_tile, tile_to_pos, EndTile, PathTile, StartTile},
    GameState,
};

const SPIRIT_SPEED: f32 = 200.;
const SPIRIT_SIZE: f32 = 32.;
const MAX_SPIRITS_IN_TILE: u32 = 3;

pub const INITIAL_SPAWN_TIME: f32 = 1.0;
const LOSE_COUNT: f32 = 40.;

const FUN_A: f32 = 10.;

// ······
// Plugin
// ······

pub struct SpiritPlugin;

impl Plugin for SpiritPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_spirit,
                check_lose_count,
                next_tile_spirit,
                spirit_collision,
                move_spirit,
                integrate,
            )
                .run_if(in_state(GameState::Play)),
        );
    }
}

// ··········
// Components
// ··········

#[derive(Component)]
pub struct Spirit {
    prev_tile: Option<TilePos>,
    curr_tile: TilePos,
    curr_distance: f32,
    next_tile: Option<TilePos>,
    next_pos: Vec2,
    selected_end: Option<TilePos>,
    vel: Vec2,
}

impl Spirit {
    pub fn new(curr_tile: TilePos, curr_pos: Vec2) -> Self {
        Self {
            prev_tile: Some(curr_tile),
            curr_tile,
            curr_distance: std::f32::MAX,
            next_tile: None,
            next_pos: curr_pos,
            selected_end: None,
            vel: Vec2::ZERO,
        }
    }
}

#[derive(Component)]
pub struct LoseText;

// ·······
// Systems
// ·······

fn spawn_spirit(
    mut cmd: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut start: Query<(&TilePos, &mut StartTile, &mut PathTile)>,
    tilemap: Query<(&TilemapGridSize, &TilemapType, &Transform)>,
) {
    for (start_pos, mut start_tile, mut start_path) in start.iter_mut() {
        if start_tile.spawn_timer.tick(time.delta()).just_finished() {
            if let Ok((grid_size, map_type, trans)) = tilemap.get_single() {
                start_tile.lose_counter += 1.;

                // Don't spawn entities if the path is not complete
                if !start_tile.completed_once {
                    continue;
                }

                // Calculate the spawn position
                let pos = tile_to_pos(start_pos, grid_size, map_type, trans);

                // If there is already another entity there, don't spawn
                if start_path.count >= 1 {
                    continue;
                }
                start_path.count += 1;

                // Spawn the entity at the start of the path
                cmd.spawn((
                    SpriteBundle {
                        texture: assets.bevy_icon.clone(),
                        transform: Transform::from_translation(pos.extend(1.))
                            .with_scale(Vec3::splat(0.15)),
                        ..default()
                    },
                    Spirit::new(*start_pos, pos),
                ));
                start_tile.lose_counter = (start_tile.lose_counter - 1.5).max(0.);

                // Reduce timer 0.01 seconds until it is 0.5
                let duration = start_tile.spawn_timer.duration().as_millis();
                if duration > 600 {
                    start_tile
                        .spawn_timer
                        .set_duration(Duration::from_millis((duration - 5) as u64));
                }
            }
        }
    }
}

fn check_lose_count(
    mut cmd: Commands,
    mut state: ResMut<NextState<GameState>>,
    assets: Res<GameAssets>,
    mut start: Query<(&TilePos, &mut StartTile)>,
    mut text: Query<&mut Text, With<LoseText>>,
    tilemap: Query<(&TilemapGridSize, &TilemapType, &Transform)>,
) {
    for (pos, mut start) in start.iter_mut() {
        if start.lose_counter > 10. {
            let lose_text = start.lose_text;

            if lose_text.is_none() {
                if let Ok((grid_size, map_type, trans)) = tilemap.get_single() {
                    let pos = tile_to_pos(pos, grid_size, map_type, trans);
                    start.lose_text = Some(
                        cmd.spawn((
                            Text2dBundle {
                                text: Text::from_section(
                                    "",
                                    TextStyle {
                                        font: assets.font.clone(),
                                        font_size: 48.,
                                        color: Color::rgb(0.0, 0.2, 0.4),
                                    },
                                ),
                                transform: Transform::from_translation(pos.extend(10.)),
                                ..default()
                            },
                            LoseText,
                        ))
                        .id(),
                    );
                }
                continue;
            };

            if let Ok(mut text) = text.get_mut(lose_text.unwrap()) {
                let remainder = 16. - ((start.lose_counter - 10.) / 2.).min(15.);
                text.sections[0].value = if remainder <= 2. {
                    "!!!".to_string()
                } else if remainder > 15. {
                    "".to_string()
                } else {
                    remainder.round().to_string()
                };
            }
        }
        if start.lose_counter >= LOSE_COUNT {
            state.set(GameState::End);
        }
    }
}

fn next_tile_spirit(
    mut cmd: Commands,
    mut score: ResMut<GameScore>,
    mut spirit: Query<(Entity, &Transform, &mut Spirit)>,
    mut paths: Query<(&TilePos, &mut PathTile)>,
    start: Query<Entity, With<StartTile>>,
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
                if spirit.next_tile.is_some() && spirit.curr_tile == tile_pos {
                    continue;
                }

                // Update counts
                if let Some(entity) = storage.get(&tile_pos) {
                    if let Ok((_, mut path)) = paths.get_mut(entity) {
                        path.count += 1;
                    }
                }
                if let Some(entity) = storage.get(&spirit.curr_tile) {
                    if let Ok((_, mut path)) = paths.get_mut(entity) {
                        path.count = path.count.saturating_sub(1);
                    }
                }
                spirit.curr_tile = tile_pos;

                // If it arrived at the next tile (or if there is no next tile)
                // Get the next tile
                if spirit.next_tile.is_none() || spirit.next_tile.unwrap() == tile_pos {
                    spirit.next_tile = None;
                    // If the spirit is on the end tile, despawn
                    if let Some(entity) = storage.get(&tile_pos) {
                        if end.get(entity).is_ok() {
                            cmd.get_entity(spirit_entity).unwrap().despawn_recursive();
                            score.score += 1;
                            if let Ok((_, mut path)) = paths.get_mut(entity) {
                                path.count = 0;
                            }
                            continue;
                        }
                        if let Some(end) = spirit.selected_end {
                            if let Ok((_, path)) = paths.get(entity) {
                                spirit.curr_distance =
                                    *path.distance.get(&end).unwrap_or(&spirit.curr_distance);
                            }
                        }
                    }

                    // Get the possible next tiles (they must be paths)
                    let neighbour_list = get_neighbours(&tile_pos, map_size);
                    let neighbours = neighbour_list
                        .iter()
                        .filter_map(|pos| storage.get(pos))
                        .filter_map(|entity| {
                            if let Ok((pos, path)) = paths.get(entity) {
                                if start.get(entity).is_ok() {
                                    return None;
                                }
                                Some((pos, path))
                            } else {
                                None
                            }
                        });

                    let n = neighbours.clone().count();

                    // If there are no surrounding paths, check if the spirit is on a path
                    // If it is not, despawn
                    if n == 0 {
                        if let Some(entity) = storage.get(&tile_pos) {
                            if paths.get(entity).is_err() {
                                cmd.get_entity(spirit_entity).unwrap().despawn_recursive();
                            }
                        }
                    }

                    // If the spirit is not on a path, reset the current distance
                    if let Some(entity) = storage.get(&tile_pos) {
                        if paths.get(entity).is_err() {
                            spirit.curr_distance = std::f32::MAX;
                        }
                    }

                    // Choose the next tile to move to
                    // For this, it must have a path score less than the current one, or else it will stay put
                    // Also, we must check that there are not too many entities in this path
                    // From the possible next tiles, it chooses the one with the lowest score
                    let mut reset_distance = false;
                    let next = neighbours
                        .map(|(pos, path)| {
                            let min_dist = |a: &f32, b: &f32| {
                                let r = rand::thread_rng().gen_range(-FUN_A / 2.0..FUN_A / 2.0);
                                (a + r).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            };

                            // Get the end position
                            // If there is no selected end, calculate the closest one
                            let (end, dist) = if let Some(end) = spirit.selected_end {
                                (
                                    spirit.selected_end.as_ref().unwrap(),
                                    path.distance.get(&end).unwrap_or(&std::f32::MAX),
                                )
                            } else {
                                path.distance
                                    .iter()
                                    .min_by(|(_, a), (_, b)| min_dist(a, b))
                                    .unwrap_or((&tile_pos, &std::f32::MAX))
                            };

                            // If the selected end is different from the current one, reset the distance
                            if let Some(s_end) = spirit.selected_end {
                                if s_end != *end {
                                    reset_distance = true;
                                }
                            }

                            // Add a random offset to the distance
                            let r = rand::thread_rng().gen_range(0.0..0.1);

                            (*pos, *dist + r, Some(*end), path.count)
                        })
                        .filter(|(pos, dist, _, count)| {
                            let is_start = if let Some(entity) = storage.get(pos) {
                                start.get(entity).is_ok()
                            } else {
                                false
                            };
                            let is_prev = if let Some(prev) = spirit.prev_tile {
                                prev == *pos
                            } else {
                                false
                            };
                            let is_further = *dist < spirit.curr_distance;
                            *count < MAX_SPIRITS_IN_TILE && is_further && !is_start && !is_prev
                        })
                        .min_by(|(_, a, _, _), (_, b, _, _)| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        });

                    if reset_distance {
                        spirit.curr_distance = std::f32::MAX;
                    }

                    if next.is_none() {
                        spirit.prev_tile = None;
                        spirit.selected_end = None;
                        continue;
                    }

                    spirit.prev_tile = Some(spirit.curr_tile);
                    spirit.next_tile = Some(next.unwrap().0);
                    spirit.next_pos =
                        tile_to_pos(&spirit.next_tile.unwrap(), grid_size, map_type, map_trans);
                    spirit.selected_end = next.unwrap().2;
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
    let mut iter = spirits.iter_combinations_mut();
    while let Some([(mut sa, ta), (mut sb, tb)]) = iter.fetch_next() {
        let delta = ta.translation.xy() - tb.translation.xy();
        let dist = delta.length();
        if dist < SPIRIT_SIZE {
            let dir = delta.normalize_or_zero();
            // Add random offset
            let r = rand::thread_rng().gen_range(-1.0..1.0);
            let dir = (dir + Vec2::new(r, r)).normalize_or_zero();
            sa.vel = sa.vel.lerp(dir * SPIRIT_SPEED, 3. / dist.max(3.));
            sb.vel = sb.vel.lerp(-dir * SPIRIT_SPEED, 3. / dist.max(3.));
        }
    }
}

fn integrate(mut spirits: Query<(&Spirit, &mut Transform)>, time: Res<Time>) {
    for (spirit, mut trans) in spirits.iter_mut() {
        // Ondulating motion
        let offset = (time.elapsed_seconds() * 1.5).sin() * 0.05;
        let cross = spirit.vel.perp();

        // Update position
        trans.translation += (spirit.vel + cross * offset).extend(0.) * time.delta_seconds();
    }
}
