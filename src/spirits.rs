#![allow(clippy::type_complexity)]

use std::time::Duration;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_persistent::Persistent;
use rand::Rng;

use crate::{
    config::GameScore,
    load::{SpiritAssets, StartAssets},
    tilemap::{
        get_neighbours, pos_to_tile, tile_to_pos, EndTile, PathTile, StartTile, TilemapLayer,
    },
    GameState,
};

const SPIRIT_SPEED: f32 = 200.;
const SPIRIT_SIZE: f32 = 50.;
const MAX_SPIRITS_IN_TILE: u32 = 3;

pub const INITIAL_SPAWN_TIME: f32 = 1.2;
const LOSE_COUNT: f32 = 30.;

const FUN_A: f32 = 10.;

// ······
// Plugin
// ······

pub struct SpiritPlugin;

impl Plugin for SpiritPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EndTimer::default())
            .add_systems(
                Update,
                (
                    spawn_spirit,
                    check_lose_count,
                    next_tile_spirit,
                    spirit_collision,
                    move_spirit,
                    animate_spirit,
                    integrate,
                )
                    .run_if(in_state(GameState::Play)),
            )
            .add_systems(
                PostUpdate,
                clear_end_count.run_if(in_state(GameState::Play)),
            )
            .add_systems(OnEnter(GameState::End), reset_spirits);
    }
}

// ·········
// Resources
// ·········

#[derive(Resource)]
pub struct EndTimer(Timer);

impl Default for EndTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.25, TimerMode::Repeating))
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
    animate_timer: Timer,
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
            animate_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

#[derive(Component)]
pub struct LoseText;

// ·······
// Systems
// ·······

fn reset_spirits(
    mut cmd: Commands,
    mut spirits: Query<Entity, Or<(With<Spirit>, With<LoseText>)>>,
) {
    for entity in spirits.iter_mut() {
        cmd.entity(entity).despawn_recursive();
    }
}

fn spawn_spirit(
    mut cmd: Commands,
    time: Res<Time>,
    spirit_assets: Res<SpiritAssets>,
    mut start: Query<(&TilePos, &mut StartTile, &mut PathTile)>,
    tilemap: Query<(&TilemapLayer, &TilemapGridSize, &TilemapType, &Transform)>,
) {
    for (start_pos, mut start_tile, mut start_path) in start.iter_mut() {
        if start_tile.spawn_timer.tick(time.delta()).just_finished() {
            start_tile.lose_counter += 1.;

            for (layer, grid_size, map_type, trans) in tilemap.iter() {
                match layer {
                    TilemapLayer::RiverStix => {}
                    _ => continue,
                }

                // Don't spawn entities if the path is not complete
                if !start_tile.completed_once {
                    continue;
                }

                // Calculate the spawn position
                let pos = tile_to_pos(start_pos, grid_size, map_type, trans);

                // If there is already another entity there, don't spawn
                if start_path.count >= 2 {
                    continue;
                }
                start_path.count += 1;

                // Spawn the entity at the start of the path
                cmd.spawn((
                    SpriteSheetBundle {
                        sprite: TextureAtlasSprite::new((rand::random::<usize>() % 3) * 2),
                        texture_atlas: spirit_assets.stix.clone(),
                        transform: Transform::from_translation(pos.extend(5.))
                            .with_scale(Vec3::splat(0.75)),
                        ..default()
                    },
                    Spirit::new(*start_pos, pos),
                ));
                start_tile.lose_counter = (start_tile.lose_counter - 2.).max(0.);

                // Reduce timer 0.01 seconds until it is 0.5
                let duration = start_tile.spawn_timer.duration().as_millis();
                if duration > 500 {
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
    assets: Res<StartAssets>,
    mut start: Query<(&TilePos, &mut StartTile)>,
    mut text: Query<&mut Text, With<LoseText>>,
    tilemap: Query<(&TilemapLayer, &TilemapGridSize, &TilemapType, &Transform)>,
) {
    for (pos, mut start) in start.iter_mut() {
        let lose_text = start.lose_text;

        if lose_text.is_none() {
            for (layer, grid_size, map_type, trans) in tilemap.iter() {
                match layer {
                    TilemapLayer::RiverStix => {}
                    _ => continue,
                }

                let pos = tile_to_pos(pos, grid_size, map_type, trans);
                start.lose_text = Some(
                    cmd.spawn((
                        Text2dBundle {
                            text: Text::from_section(
                                "",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 48.,
                                    color: Color::rgb(0.9, 0.4, 0.6),
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
            let remainder = (LOSE_COUNT - start.lose_counter) / 2. - 3.;
            text.sections[0].value = if remainder <= 0. {
                "!!!".to_string()
            } else if remainder > 10. {
                "".to_string()
            } else {
                remainder.round().to_string()
            };
        }
        if start.lose_counter >= LOSE_COUNT {
            state.set(GameState::End);
        }
    }
}

fn next_tile_spirit(
    mut cmd: Commands,
    mut spirit: Query<(Entity, &Transform, &mut Spirit)>,
    mut paths: Query<(&TilePos, &mut PathTile)>,
    start: Query<Entity, With<StartTile>>,
    end: Query<Entity, With<EndTile>>,
    tilemap: Query<
        (
            &TilemapLayer,
            &TilemapSize,
            &TilemapGridSize,
            &TilemapType,
            &TileStorage,
            &Transform,
        ),
        Without<Spirit>,
    >,
) {
    for (layer, map_size, grid_size, map_type, storage, map_trans) in tilemap.iter() {
        match layer {
            TilemapLayer::RiverStix => {}
            _ => continue,
        }
        for (spirit_entity, trans, mut spirit) in spirit.iter_mut() {
            if let Some(tile_pos) = pos_to_tile(
                &trans.translation.xy(),
                map_size,
                grid_size,
                map_type,
                map_trans,
            ) {
                // Check if selected end tile is reachable
                if let Some(selected_end) = spirit.selected_end {
                    if let Some(entity) = storage.get(&tile_pos) {
                        if let Ok((_, path)) = paths.get(entity) {
                            if !path.distance.contains_key(&selected_end) {
                                spirit.selected_end = None;
                                spirit.next_tile = None;
                                spirit.curr_distance = std::f32::MAX;
                            }
                        }
                    }
                }

                if spirit.next_tile.is_some()
                    && spirit.selected_end.is_some()
                    && spirit.curr_tile == tile_pos
                {
                    continue;
                }
                spirit.curr_tile = tile_pos;

                // If it arrived at the next tile (or if there is no next tile)
                // Get the next tile
                if spirit.next_tile.is_none() || spirit.next_tile.unwrap() == tile_pos {
                    spirit.next_tile = None;
                    // If the spirit is on the end tile, despawn
                    if let Some(entity) = storage.get(&tile_pos) {
                        if end.get(entity).is_ok() {
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

                    // Update counts
                    if let Some(entity) = storage.get(&spirit.next_tile.unwrap()) {
                        if let Ok((_, mut path)) = paths.get_mut(entity) {
                            path.count += 1;
                        }
                    }
                    if let Some(entity) = storage.get(&spirit.prev_tile.unwrap()) {
                        if let Ok((_, mut path)) = paths.get_mut(entity) {
                            path.count = path.count.saturating_sub(1);
                        }
                    }
                }
            }
        }
    }
}

fn clear_end_count(
    mut cmd: Commands,
    time: Res<Time>,
    mut score: ResMut<Persistent<GameScore>>,
    mut end: Query<(&mut PathTile, &TilePos), With<EndTile>>,
    spirits: Query<(Entity, &Spirit)>,
    mut timer: ResMut<EndTimer>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    for (mut end, end_pos) in end.iter_mut() {
        for (entity, spirit) in spirits.iter() {
            if spirit.curr_tile == *end_pos {
                cmd.get_entity(entity).unwrap().despawn_recursive();
                end.count = end.count.saturating_sub(1);
                score.score += 1;
                break;
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

fn animate_spirit(time: Res<Time>, mut spirits: Query<(&mut Spirit, &mut TextureAtlasSprite)>) {
    for (mut spirit, mut tex) in spirits.iter_mut() {
        if !spirit.animate_timer.tick(time.delta()).just_finished() {
            continue;
        }
        tex.index = if tex.index % 2 == 0 {
            tex.index + 1
        } else {
            tex.index - 1
        };
    }
}
