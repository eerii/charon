use bevy::{prelude::*, render::view::RenderLayers};
use bevy_ecs_tilemap::prelude::*;
use rand::Rng;

use crate::{
    tilemap::{play_to_real_size, EndTile, LevelSize, PathTile, StartTile},
    GameState,
};

const START_SCORES: [u32; 17] = [
    0, 10, 30, 70, 100, 150, 300, 500, 700, 1000, 1500, 2000, 2500, 3500, 5000, 7000, 8500,
];

const END_SCORES: [u32; 7] = [0, 50, 200, 400, 900, 3000, 9000];

pub struct CharonPlugin;

impl Plugin for CharonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Play),
            (init_game.run_if(run_once()), resume_game),
        )
        .add_systems(
            Update,
            spawn_start_end.run_if(resource_exists_and_changed::<GameScore>()),
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

fn init_game(mut cmd: Commands) {
    cmd.spawn((Camera2dBundle::default(), RenderLayers::layer(0), GameCam));
    cmd.insert_resource(GameScore::default())
}

fn resume_game(mut cam: Query<&mut Camera, With<GameCam>>) {
    for mut cam in cam.iter_mut() {
        cam.is_active = true;
    }
}

fn pause_game(mut cam: Query<&mut Camera, With<GameCam>>) {
    for mut cam in cam.iter_mut() {
        cam.is_active = false;
    }
}

fn spawn_start_end(
    mut cmd: Commands,
    score: Res<GameScore>,
    mut level_size: ResMut<LevelSize>,
    tilemap: Query<&TileStorage>,
    starts: Query<&TilePos, With<StartTile>>,
    ends: Query<&TilePos, With<EndTile>>,
    mut start_spawned: Local<usize>,
    mut end_spawned: Local<usize>,
) {
    // Check if we need to spawn a start or end tile
    let next_start = if *start_spawned < START_SCORES.len() {
        START_SCORES[*start_spawned]
    } else {
        (*start_spawned + 1 - START_SCORES.len()) as u32 * 10000
    };

    let next_end = if *end_spawned < END_SCORES.len() {
        END_SCORES[*end_spawned]
    } else {
        (*end_spawned + 1 - END_SCORES.len()) as u32 * 25000
    };

    let mut is_start = false;
    let mut is_end = false;

    if score.score >= next_start {
        *start_spawned += 1;
        is_start = true;
    }

    if score.score >= next_end {
        *end_spawned += 1;
        is_end = true;
    }

    if !is_start && !is_end {
        return;
    };

    // Grow level size every 2 starts
    if is_start && (*start_spawned + 2) % 3 == 0 {
        level_size.0.x += 2;
        level_size.0.y += 2;
    }
    let (offset, size) = play_to_real_size(&*level_size);

    if let Ok(storage) = tilemap.get_single() {
        if is_start {
            let pos = if *start_spawned <= 1 {
                Some(TilePos {
                    x: offset.x + 1,
                    y: offset.y + size.y / 2,
                })
            } else {
                get_spawn_pos(&offset, &size, &starts, &ends)
            };
            if let Some(pos) = pos {
                cmd.entity(storage.get(&pos).unwrap())
                    .insert((StartTile::default(), PathTile::default()));
            }
        }

        if is_end {
            let pos = if *end_spawned <= 1 {
                Some(TilePos {
                    x: offset.x + size.x - 2,
                    y: offset.y + size.y / 2,
                })
            } else {
                get_spawn_pos(&offset, &size, &starts, &ends)
            };
            if let Some(pos) = pos {
                cmd.entity(storage.get(&pos).unwrap())
                    .insert((EndTile, PathTile::default()));
            }
        }
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
