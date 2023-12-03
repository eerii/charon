#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::SquareDirection, prelude::*};
use bevy_persistent::Persistent;

use crate::{
    config::Keybinds,
    input::{Bind, MousePosition},
    load::TilemapAssets,
    GameState,
};

const MAP_SIZE: TilemapSize = TilemapSize { x: 15, y: 10 };
const TILE_SIZE: TilemapTileSize = TilemapTileSize { x: 64., y: 64. };
const GRID_SIZE: TilemapGridSize = TilemapGridSize { x: 72., y: 72. };

// ······
// Plugin
// ······

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .add_systems(OnEnter(GameState::Play), init_tilemap)
            .add_systems(
                Update,
                (
                    select_tile,
                    click_tile,
                    astar.run_if(not(resource_exists::<AstarMap>())),
                )
                    .run_if(in_state(GameState::Play)),
            )
            .add_systems(PostUpdate, highlight_tile.run_if(in_state(GameState::Play)));
    }
}

// ·········
// Resources
// ·········

#[derive(Resource)]
pub struct AstarMap;

// ··········
// Components
// ··········

#[derive(Component)]
pub struct SelectedTile;

#[derive(Component)]
pub struct StartTile;

#[derive(Component)]
pub struct EndTile;

#[derive(Component)]
pub struct PathTile(u32);

// ·······
// Systems
// ·······

fn init_tilemap(mut cmd: Commands, assets: Res<TilemapAssets>) {
    let tilemap = cmd.spawn_empty().id();

    // Spawn tiles
    let mut storage = TileStorage::empty(MAP_SIZE);
    for x in 0..MAP_SIZE.x {
        for y in 0..MAP_SIZE.y {
            let pos = TilePos { x, y };
            let tile = cmd
                .spawn(TileBundle {
                    position: pos,
                    tilemap_id: TilemapId(tilemap),
                    ..default()
                })
                .id();
            storage.set(&pos, tile);
        }
    }

    cmd.entity(storage.get(&TilePos { x: 0, y: 3 }).unwrap())
        .insert(StartTile);
    cmd.entity(storage.get(&TilePos { x: 14, y: 7 }).unwrap())
        .insert(EndTile);

    // Create tilemap
    let map_type = TilemapType::default();
    cmd.entity(tilemap).insert(TilemapBundle {
        size: MAP_SIZE,
        tile_size: TILE_SIZE,
        grid_size: GRID_SIZE,
        map_type,
        storage,
        texture: TilemapTexture::Single(assets.tiles.clone()),
        transform: get_tilemap_center_transform(&MAP_SIZE, &GRID_SIZE, &map_type, 0.0),
        ..default()
    });
}

fn select_tile(
    mut cmd: Commands,
    mouse: Res<MousePosition>,
    tilemap: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapType,
        &TileStorage,
        &Transform,
    )>,
    selected: Query<Entity, With<SelectedTile>>,
) {
    for entity in selected.iter() {
        cmd.entity(entity).remove::<SelectedTile>();
    }

    for (map_size, grid_size, map_type, tile_storage, map_transform) in tilemap.iter() {
        let mouse: Vec2 = mouse.0;

        let mouse_in_map_pos: Vec2 = {
            let mouse = Vec4::from((mouse, 0.0, 1.0));
            let mouse_in_map_pos = map_transform.compute_matrix().inverse() * mouse;
            mouse_in_map_pos.xy()
        };

        if let Some(tile_pos) =
            TilePos::from_world_pos(&mouse_in_map_pos, map_size, grid_size, map_type)
        {
            if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                cmd.entity(tile_entity).insert(SelectedTile);
            }
        }
    }
}

fn click_tile(
    mut cmd: Commands,
    mut selected: Query<Entity, With<SelectedTile>>,
    path: Query<Entity, With<PathTile>>,
    start_finish: Query<Entity, Or<(With<StartTile>, With<EndTile>)>>,
    input: Res<Input<Bind>>,
    keybinds: Res<Persistent<Keybinds>>,
    mut is_selecting: Local<Option<bool>>,
) {
    let select = keybinds.interact.iter().any(|bind| {
        if is_selecting.is_none() {
            input.just_pressed(*bind)
        } else {
            input.pressed(*bind)
        }
    });

    if select {
        if let Ok(entity) = selected.get_single_mut() {
            let is_path = path.get(entity).is_ok();
            let is_start_finish = start_finish.get(entity).is_ok();

            if is_selecting.is_none() {
                *is_selecting = Some(is_path);
            }

            if is_path != *is_selecting.as_ref().unwrap() {
                return;
            }

            if is_path {
                cmd.entity(entity).remove::<PathTile>();
            } else if !is_start_finish {
                cmd.entity(entity).insert(PathTile(0));
            }

            return;
        }
    }

    *is_selecting = None;
}

fn highlight_tile(
    mut tiles: Query<(
        &mut TileTextureIndex,
        Option<&SelectedTile>,
        Option<&PathTile>,
        Option<&StartTile>,
        Option<&EndTile>,
    )>,
) {
    for (mut tex, selected, path, start, end) in tiles.iter_mut() {
        if selected.is_some() {
            *tex = TileTextureIndex(0);
        } else if path.is_some() {
            *tex = TileTextureIndex(1);
        } else if start.is_some() || end.is_some() {
            *tex = TileTextureIndex(2);
        } else {
            *tex = TileTextureIndex(0);
        }
    }
}

fn astar(
    mut cmd: Commands,
    tilemap: Query<(&TilemapSize, &TileStorage)>,
    start: Query<&TilePos, With<StartTile>>,
    end: Query<&TilePos, With<EndTile>>,
    mut paths: Query<(&mut PathTile, &TilePos)>,
) {
    if let Ok((size, storage)) = tilemap.get_single() {
        if let (Ok(start), Ok(end)) = (start.get_single(), end.get_single()) {
            info!("Start: {:?}, End: {:?}", start, end);

            if let Some(next_pos) = start.diamond_offset(&SquareDirection::East, size) {
                if let Some(next) = storage.checked_get(&next_pos) {
                    if let Ok((mut path, pos)) = paths.get_mut(next) {
                        *path = PathTile(1);
                        info!("Next: {:?}", pos);
                    }
                }
            }
        }
    }

    cmd.insert_resource(AstarMap);
}
