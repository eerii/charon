#![allow(clippy::too_many_arguments)]

use bevy::{prelude::*, render::view::RenderLayers, window::WindowResized};
use bevy_ecs_tilemap::prelude::*;
use bevy_persistent::Persistent;
use rand::Rng;

use crate::{
    config::GameScore,
    tilemap::{
        play_to_real_size, EndTile, ForegroundTile, LevelSize, PathTile, StartTile, TilemapLayer,
        TilesAvailable, MAP_SIZE,
    },
    GameState, INITIAL_RESOLUTION,
};

//0, 1, 2, 3, 4, 5, 130, 160, 250, 300, 400, 500, 700, 900, 1200, 1500, 2000, 2500, 3500,
const START_SCORES: [u32; 20] = [
    0, 5, 30, 50, 100, 130, 160, 220, 260, 300, 400, 500, 700, 900, 1200, 1500, 2000, 2500, 3500,
    5000,
];

const END_SCORES: [u32; 6] = [0, 70, 200, 350, 1000, 3000];

pub struct CharonPlugin;

impl Plugin for CharonPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SpawnedCount::default())
            .add_systems(OnEnter(GameState::Play), init_game)
            .add_systems(
                Update,
                (
                    zoom_camera,
                    spawn_start_end.run_if(
                        resource_exists::<TilesAvailable>()
                            .and_then(resource_exists_and_changed::<Persistent<GameScore>>()),
                    ),
                )
                    .run_if(in_state(GameState::Play)),
            )
            .add_systems(OnExit(GameState::Play), pause_game)
            .add_systems(OnEnter(GameState::End), reset_score);
    }
}

// ·········
// Resources
// ·········

#[derive(Resource, Default)]
struct SpawnedCount {
    start: usize,
    end: usize,
}

// ··········
// Components
// ··········

#[derive(Component, Default)]
pub struct GameCam {
    target_zoom: f32,
}

// ·······
// Systems
// ·······

fn init_game(
    mut cmd: Commands,
    mut score: ResMut<Persistent<GameScore>>,
    mut cam: Query<&mut Camera, With<GameCam>>,
) {
    if cam.iter().count() == 0 {
        cmd.spawn((
            Camera2dBundle::default(),
            RenderLayers::layer(0),
            GameCam::default(),
        ));
    }

    for mut cam in cam.iter_mut() {
        cam.is_active = true;
    }

    score.score = 0;
}

fn pause_game(mut cam: Query<&mut Camera, With<GameCam>>) {
    for mut cam in cam.iter_mut() {
        cam.is_active = false;
    }
}

fn reset_score(
    mut score: ResMut<Persistent<GameScore>>,
    mut count: ResMut<SpawnedCount>,
    mut cam: Query<&mut GameCam>,
) {
    score
        .update(|score| {
            score.last_score = score.score;
            score.best_score = score.score.max(score.best_score);
            score.score = 0;
        })
        .expect("Failed to update score");

    *count = SpawnedCount::default();

    for mut cam in cam.iter_mut() {
        cam.target_zoom = 0.;
    }
}

fn spawn_start_end(
    mut cmd: Commands,
    score: Res<Persistent<GameScore>>,
    mut level_size: ResMut<LevelSize>,
    mut available: ResMut<TilesAvailable>,
    tilemap: Query<(&TilemapLayer, &TileStorage)>,
    starts: Query<&TilePos, With<StartTile>>,
    ends: Query<&TilePos, With<EndTile>>,
    mut visible: Query<&mut TileVisible>,
    mut cam: Query<&mut GameCam>,
    mut count: ResMut<SpawnedCount>,
) {
    // Check if we need to spawn a start or end tile
    let next_start = if count.start < START_SCORES.len() {
        START_SCORES[count.start]
    } else {
        5000 + (count.start + 1 - START_SCORES.len()) as u32 * 1000
    };

    let next_end = if count.end < END_SCORES.len() {
        END_SCORES[count.end]
    } else {
        (count.end + 1 - END_SCORES.len()) as u32 * 10000
    };

    let mut is_start = false;
    let mut is_end = false;

    if score.score >= next_start {
        count.start += 1;
        is_start = true;
    }

    if score.score >= next_end {
        count.end += 1;
        is_end = true;
    }

    if !is_start && !is_end {
        return;
    };

    // Grow level size every 2 starts (only if we are not at the max size)
    if is_start && (count.start + 3) % 4 == 0 && level_size.0.x < MAP_SIZE.x {
        level_size.0.x += 2;
        level_size.0.y += 2;
        if let Ok(mut cam) = cam.get_single_mut() {
            cam.target_zoom += 0.3;
        }
    }
    let (offset, size) = play_to_real_size(&level_size);

    let mut spawn_fun = |is_start: bool| {
        // Get spawn position
        let spawn_pos = {
            if is_start {
                if count.start <= 1 {
                    Some(TilePos {
                        x: offset.x + 1,
                        y: offset.y + size.y / 2,
                    })
                } else {
                    get_spawn_pos(&offset, &size, &starts, &ends)
                }
            } else if count.end <= 1 {
                Some(TilePos {
                    x: offset.x + size.x - 2,
                    y: offset.y + size.y / 2,
                })
            } else {
                get_spawn_pos(&offset, &size, &starts, &ends)
            }
        };

        if let Some(pos) = spawn_pos {
            for (layer, storage) in tilemap.iter() {
                match layer {
                    // Insert the logical tile in the river
                    TilemapLayer::RiverStix => {
                        if let Some(entity) = storage.get(&pos) {
                            if is_start {
                                cmd.entity(entity)
                                    .insert((StartTile::default(), PathTile::default()));
                            } else {
                                cmd.entity(entity).insert((EndTile, PathTile::default()));
                            }
                            if let Ok(mut visible) = visible.get_mut(entity) {
                                visible.0 = true;
                            }
                        }
                    }
                    // Add the graphics element to the foreground
                    TilemapLayer::Foreground => {
                        if let Some(entity) = storage.get(&pos) {
                            cmd.entity(entity).insert(if is_start {
                                ForegroundTile::Start
                            } else {
                                ForegroundTile::End
                            });
                            if let Ok(mut visible) = visible.get_mut(entity) {
                                visible.0 = true;
                            }
                        }
                    }
                    _ => continue,
                }
            }
            available.0 += if is_start { 2 } else { 4 };
        }
    };

    if is_start {
        spawn_fun(true);
    }
    if is_end {
        spawn_fun(false);
    }
}

fn zoom_camera(
    mut cam: Query<(&mut OrthographicProjection, &GameCam)>,
    mut win: Query<&mut Window>,
    mut on_resize: EventReader<WindowResized>,
    mut base_scale: Local<f32>,
) {
    if *base_scale == 0. {
        *base_scale = 0.9;
    }

    for e in on_resize.read() {
        *base_scale = (INITIAL_RESOLUTION.x / e.width) * 0.9;
        if let Ok(mut win) = win.get_single_mut() {
            win.resolution.set(e.width, e.height);
        }
    }

    if let Ok((mut proj, cam)) = cam.get_single_mut() {
        proj.scale = lerp(proj.scale, *base_scale + cam.target_zoom, 0.01);
    }
}

// ·····
// Extra
// ·····

fn get_spawn_pos(
    offset: &TilemapSize,
    size: &TilemapSize,
    starts: &Query<&TilePos, With<StartTile>>,
    ends: &Query<&TilePos, With<EndTile>>,
) -> Option<TilePos> {
    // Calculate possible positions (along the border)
    let mut possible = Vec::new();
    for i in 0..size.x {
        possible.push(TilePos { x: i, y: 0 });
        possible.push(TilePos {
            x: i,
            y: size.y - 1,
        });
    }
    for i in 0..size.y {
        possible.push(TilePos { x: 0, y: i });
        possible.push(TilePos {
            x: size.x - 1,
            y: i,
        });
    }

    // Remove occupied starts and ends and their neighbours
    for start in starts.iter() {
        let pos = TilePos {
            x: start.x - offset.x,
            y: start.y - offset.y,
        };
        possible.retain(|p| tile_distance(p, &pos) > 2);
    }
    for end in ends.iter() {
        let pos = TilePos {
            x: end.x - offset.x,
            y: end.y - offset.y,
        };
        possible.retain(|p| tile_distance(p, &pos) > 2);
    }
    if possible.is_empty() {
        return None;
    }

    // Select random position
    let selected = possible[rand::thread_rng().gen_range(0..possible.len())];
    Some(TilePos {
        x: selected.x + offset.x,
        y: selected.y + offset.y,
    })
}

fn tile_distance(a: &TilePos, b: &TilePos) -> u32 {
    ((a.x as i32 - b.x as i32).abs() + (a.y as i32 - b.y as i32).abs()) as u32
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
