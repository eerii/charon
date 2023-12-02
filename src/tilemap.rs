use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{input::MousePosition, load::TilemapAssets, GameState};

const MAP_SIZE: TilemapSize = TilemapSize { x: 14, y: 10 };
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
            .add_systems(Update, select_tile)
            .add_systems(PostUpdate, highlight_selected_tile);
    }
}

// ··········
// Components
// ··········

#[derive(Component)]
pub struct SelectedTile;

// ·······
// Systems
// ·······

fn init_tilemap(mut cmd: Commands, tiles: Res<TilemapAssets>) {
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

    // Create tilemap
    let map_type = TilemapType::default();
    cmd.entity(tilemap).insert(TilemapBundle {
        size: MAP_SIZE,
        tile_size: TILE_SIZE,
        grid_size: GRID_SIZE,
        map_type,
        storage,
        texture: TilemapTexture::Single(tiles.temp.clone()),
        transform: get_tilemap_center_transform(&MAP_SIZE, &GRID_SIZE, &map_type, 0.0),
        ..default()
    });

    // Create atlas (if needed)
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

fn highlight_selected_tile(mut tile: Query<(&mut TileColor, Option<&SelectedTile>)>) {
    for (mut color, selected) in tile.iter_mut() {
        if selected.is_some() {
            *color = TileColor(Color::GREEN);
        } else {
            *color = TileColor(Color::WHITE);
        }
    }
}
