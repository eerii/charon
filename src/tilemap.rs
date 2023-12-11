#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use bevy::prelude::*;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::SquareDirection, prelude::*};
use bevy_persistent::Persistent;

use crate::{
    config::Keybinds,
    input::{Bind, MousePosition},
    load::TilemapAssets,
    spirits::INITIAL_SPAWN_TIME,
    GameState,
};

pub const MAP_SIZE: TilemapSize = TilemapSize { x: 24, y: 19 };
const TILE_SIZE: TilemapTileSize = TilemapTileSize { x: 128., y: 128. };
const GRID_SIZE: TilemapGridSize = TilemapGridSize { x: 127.5, y: 127.5 };
const INITIAL_TILES: u32 = 9;

// ······
// Plugin
// ······

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NeedsReset)
            .add_plugins(TilemapPlugin)
            .add_systems(
                OnEnter(GameState::Play),
                init_tilemap.run_if(resource_added::<NeedsReset>()),
            )
            .add_systems(
                Update,
                (select_tile, click_tile).run_if(in_state(GameState::Play)),
            )
            .add_systems(
                PostUpdate,
                (
                    highlight_tile,
                    (autotile, pathfinding).run_if(resource_exists_and_changed::<TilesAvailable>()),
                )
                    .run_if(in_state(GameState::Play)),
            )
            .add_systems(OnEnter(GameState::End), reset_tilemap);
    }
}

// ·········
// Resources
// ·········

#[derive(Resource)]
pub struct TilesAvailable(pub u32);

#[derive(Resource)]
pub struct SelectedPos(Option<TilePos>);

#[derive(Resource)]
pub struct LevelSize(pub TilemapSize);

#[derive(Resource)]
pub struct NeedsReset;

// ··········
// Components
// ··········

#[derive(Component)]
pub struct SelectedTile;

#[derive(Component)]
pub struct StartTile {
    pub completed_once: bool,
    pub spawn_timer: Timer,
    pub lose_counter: f32,
    pub lose_text: Option<Entity>,
}

impl Default for StartTile {
    fn default() -> Self {
        Self {
            completed_once: false,
            spawn_timer: Timer::from_seconds(INITIAL_SPAWN_TIME, TimerMode::Repeating),
            lose_counter: 0.,
            lose_text: None,
        }
    }
}

#[derive(Component)]
pub struct EndTile;

#[derive(Clone)]
pub enum PathShape {
    None,
    End,
    Straight,
    Turn,
    Junction,
    Crossing,
}

#[derive(Component, Clone)]
pub struct PathTile {
    pub distance: HashMap<TilePos, f32>,
    pub count: u32,
    pub shape: PathShape,
    pub rot: u32,
}

impl Default for PathTile {
    fn default() -> Self {
        Self {
            distance: HashMap::new(),
            count: 0,
            shape: PathShape::End,
            rot: 0,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub enum TilemapLayer {
    Background,
    RiverStix,
    RiverPhlege,
    Foreground,
}
const TILEMAP_LAYERS: [TilemapLayer; 4] = [
    TilemapLayer::Background,
    TilemapLayer::RiverStix,
    TilemapLayer::RiverPhlege,
    TilemapLayer::Foreground,
];

#[derive(Component)]
pub enum ForegroundTile {
    Start,
    End,
}

// ·······
// Systems
// ·······

fn init_tilemap(mut cmd: Commands, tile_assets: Res<TilemapAssets>) {
    for (i, &layer) in TILEMAP_LAYERS.iter().enumerate() {
        let tilemap = cmd.spawn_empty().id();

        // Spawn tiles
        let mut storage = TileStorage::empty(MAP_SIZE);
        fill_tilemap(
            TilemapId(tilemap),
            MAP_SIZE,
            &mut storage,
            TileVisible(i == 0),
            &mut cmd,
        );

        // Create tilemap
        let map_type = TilemapType::default();
        cmd.entity(tilemap).insert((
            TilemapBundle {
                size: MAP_SIZE,
                tile_size: TILE_SIZE,
                grid_size: GRID_SIZE,
                map_type,
                storage,
                texture: TilemapTexture::Single(tile_assets.stix.clone()),
                transform: get_tilemap_center_transform(&MAP_SIZE, &GRID_SIZE, &map_type, i as f32),
                ..default()
            },
            layer,
        ));
    }

    cmd.insert_resource(TilesAvailable(INITIAL_TILES));
    cmd.insert_resource(SelectedPos(None));
    cmd.insert_resource(LevelSize(TilemapSize { x: 8, y: 3 }));
    cmd.remove_resource::<NeedsReset>();
}

fn reset_tilemap(
    mut cmd: Commands,
    tiles: Query<Entity, Or<(With<TilemapSize>, With<TilePos>, With<TileStorage>)>>,
) {
    for entity in tiles.iter() {
        if let Some(mut entity) = cmd.get_entity(entity) {
            entity.despawn();
        }
    }
    cmd.insert_resource(NeedsReset);
}

fn select_tile(
    mut cmd: Commands,
    mouse: Res<MousePosition>,
    mut sel_pos: ResMut<SelectedPos>,
    level_size: Res<LevelSize>,
    tilemap: Query<(
        &TilemapLayer,
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

    for (layer, map_size, grid_size, map_type, tile_storage, trans) in tilemap.iter() {
        match layer {
            TilemapLayer::RiverStix => {}
            _ => continue,
        }

        if let Some(tile_pos) = pos_to_tile(&mouse.0, map_size, grid_size, map_type, trans) {
            if !tile_in_level(&tile_pos, &level_size) {
                sel_pos.0 = None;
                return;
            }
            if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                if let Some(mut entity) = cmd.get_entity(tile_entity) {
                    entity.insert(SelectedTile);
                }
                sel_pos.0 = Some(tile_pos);
            }
        }
    }
}

fn click_tile(
    mut cmd: Commands,
    mut selected: Query<(Entity, &mut TileVisible), With<SelectedTile>>,
    tiles: Query<(Option<&PathTile>, Option<&StartTile>, Option<&EndTile>)>,
    input: Res<Input<Bind>>,
    keybinds: Res<Persistent<Keybinds>>,
    mut available: ResMut<TilesAvailable>,
    mut prev: Local<Option<bool>>,
) {
    let select = keybinds.interact.iter().any(|bind| {
        if prev.is_none() {
            input.just_pressed(*bind)
        } else {
            input.pressed(*bind)
        }
    });

    if select {
        if let Ok((entity, mut visible)) = selected.get_single_mut() {
            if let Ok((path, start, end)) = tiles.get(entity) {
                if prev.is_none() {
                    *prev = Some(path.is_some());
                }
                let is_path = prev.as_mut().unwrap();

                if path.is_some() != *is_path {
                    return;
                }

                if start.is_none() && end.is_none() {
                    // Erase path
                    if path.is_some() {
                        cmd.entity(entity).remove::<PathTile>();
                        visible.0 = false;
                        available.0 += 1;
                        return;
                    }

                    // Add paths
                    if available.0 == 0 {
                        return;
                    }
                    cmd.entity(entity).insert(PathTile::default());
                    visible.0 = true;
                    available.0 -= 1;
                }
            }
            return;
        }
    }
    *prev = None;
}

fn highlight_tile(
    mut tiles: Query<(
        &mut TileTextureIndex,
        &mut TileColor,
        &mut TileFlip,
        &TilePos,
        Option<&PathTile>,
        Option<&StartTile>,
        Option<&ForegroundTile>,
    )>,
    level_size: Res<LevelSize>,
    sel_pos: Res<SelectedPos>,
    end_tiles: Query<(&TilePos, With<EndTile>)>,
) {
    let mut ends = Vec::new();
    for (pos, _) in end_tiles.iter() {
        ends.push(*pos);
    }

    for (mut tex, mut color, mut flip, pos, path, start, foreground) in tiles.iter_mut() {
        if let Some(sel_pos) = sel_pos.0 {
            if sel_pos == *pos && path.is_none() && foreground.is_none() {
                *tex = TileTextureIndex(1);
                continue;
            }
        }

        if !tile_in_level(pos, &level_size) {
            *color = TileColor(Color::rgb(0.2, 0.1, 0.35));
            continue;
        }

        if start.is_some() {
            *tex = TileTextureIndex(9);
            continue;
        }

        *color = TileColor::default();

        if let Some(fg) = foreground {
            match fg {
                ForegroundTile::Start => {
                    *tex = TileTextureIndex(12);
                }
                ForegroundTile::End => {
                    *tex = TileTextureIndex(11);
                }
            }
            continue;
        }

        if path.is_some() {
            *tex = match path.unwrap().shape {
                PathShape::None => TileTextureIndex(0),
                PathShape::End => TileTextureIndex(4),
                PathShape::Straight => TileTextureIndex(2),
                PathShape::Turn => TileTextureIndex(6),
                PathShape::Junction => TileTextureIndex(8),
                PathShape::Crossing => TileTextureIndex(9),
            };
            *flip = flip_from_rotation(path.unwrap().rot);
        } else {
            *tex = TileTextureIndex(0);
        }
    }
}

fn pathfinding(
    tilemap: Query<(&TilemapLayer, &TilemapSize, &TileStorage)>,
    mut start: Query<(&TilePos, &mut StartTile)>,
    end: Query<&TilePos, With<EndTile>>,
    mut paths: Query<(&TilePos, &mut PathTile)>,
) {
    // Clear all paths
    for (_, mut path) in paths.iter_mut() {
        path.distance.clear();
    }

    for (layer, size, storage) in tilemap.iter() {
        match layer {
            TilemapLayer::RiverStix => {}
            _ => continue,
        }

        for end_pos in end.iter() {
            let mut open = BinaryHeap::new();
            let mut distances = HashMap::new();

            // Add the end position to the queue
            distances.insert(*end_pos, 0.);
            open.push(PathfindingNode {
                pos: *end_pos,
                distance: 0.,
            });
            if let Some(entity) = storage.get(end_pos) {
                if let Ok((_, mut path)) = paths.get_mut(entity) {
                    path.distance.insert(*end_pos, 0.);
                }
            }

            // Start iterating through the queue
            while let Some(PathfindingNode { pos, distance }) = open.pop() {
                // If the path is a start, cut this branch
                if let Some(entity) = storage.get(&pos) {
                    if start.get_mut(entity).is_ok() {
                        continue;
                    }
                }

                // Get the neighbouring tiles
                let neighbours = get_neighbours(&pos, size);

                for neighbour in neighbours {
                    if let Some(entity) = storage.get(&neighbour) {
                        if let Ok((_, mut path)) = paths.get_mut(entity) {
                            // Djikstra's algorithm to find the shortest path from each tile
                            let dist = distance + 1.;
                            if dist < *distances.get(&neighbour).unwrap_or(&std::f32::INFINITY) {
                                distances.insert(neighbour, dist);
                                open.push(PathfindingNode {
                                    pos: neighbour,
                                    distance: dist,
                                });
                                path.distance.insert(*end_pos, dist);
                            }
                        }
                    }
                }
            }

            // Set the end distance to 0
            if let Some(entity) = storage.get(end_pos) {
                if let Ok((_, mut path)) = paths.get_mut(entity) {
                    path.distance.insert(*end_pos, 0.);
                }
            }

            // Check if there is a path from the end to the start
            for (start_pos, mut start_tile) in start.iter_mut() {
                if distances.contains_key(start_pos) {
                    start_tile.completed_once = true;
                    // Set begin distance to MAX
                    if let Some(entity) = storage.get(start_pos) {
                        if let Ok((_, mut path)) = paths.get_mut(entity) {
                            path.distance.insert(*start_pos, std::f32::INFINITY);
                        }
                    }
                }
            }
        }
    }
}

fn autotile(
    tilemap: Query<(&TilemapLayer, &TilemapSize, &TileStorage)>,
    mut paths: Query<(&TilePos, &mut PathTile)>,
) {
    for (layer, size, storage) in tilemap.iter() {
        match layer {
            TilemapLayer::RiverStix => {}
            _ => continue,
        }

        let mut path_shapes = HashMap::new();

        for (pos, _) in paths.iter() {
            let neighbours = get_neighbours(pos, size);

            // Get the neighbouring tiles
            let neighbours = neighbours
                .iter()
                .filter_map(|pos| storage.get(pos))
                .filter_map(|entity| {
                    if let Ok((pos, _)) = paths.get(entity) {
                        Some(pos)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            // Get the shape of the path
            let shape = match neighbours.len() {
                1 => PathShape::End,
                2 => {
                    if neighbours[0].x == neighbours[1].x || neighbours[0].y == neighbours[1].y {
                        PathShape::Straight
                    } else {
                        PathShape::Turn
                    }
                }
                3 => PathShape::Junction,
                4 => PathShape::Crossing,
                _ => PathShape::None,
            };

            // Get the rotation
            let rot = match shape {
                PathShape::End => {
                    if neighbours.is_empty() || neighbours[0].x < pos.x {
                        0
                    } else if neighbours[0].x > pos.x {
                        2
                    } else if neighbours[0].y < pos.y {
                        1
                    } else {
                        3
                    }
                }
                PathShape::Straight => {
                    if neighbours[0].x == neighbours[1].x {
                        1
                    } else {
                        0
                    }
                }
                PathShape::Turn => {
                    if neighbours[0].x < neighbours[1].x {
                        if neighbours[0].y > neighbours[1].y {
                            1
                        } else {
                            0
                        }
                    } else if neighbours[0].y > neighbours[1].y {
                        2
                    } else {
                        3
                    }
                }
                PathShape::Junction => {
                    if neighbours[0].y == neighbours[1].y {
                        if neighbours[0].y < neighbours[2].y {
                            0
                        } else {
                            2
                        }
                    } else if neighbours[0].x < neighbours[2].x {
                        1
                    } else {
                        3
                    }
                }
                _ => 0,
            };

            path_shapes.insert(*pos, (shape, rot));
        }

        for (pos, mut path) in paths.iter_mut() {
            let (shape, rot) = path_shapes.get(pos).unwrap();

            path.shape = shape.clone();
            path.rot = *rot;
        }
    }
}

// ·····
// Extra
// ·····

const DIRECTIONS: [SquareDirection; 4] = [
    // DONT CHANGE THE ORDER, BREAKS AUTOTILING
    SquareDirection::West,
    SquareDirection::East,
    SquareDirection::North,
    SquareDirection::South,
];

pub fn get_neighbours(pos: &TilePos, size: &TilemapSize) -> Vec<TilePos> {
    let mut neighbours = Vec::new();

    for direction in DIRECTIONS.iter() {
        if let Some(pos) = pos.diamond_offset(direction, size) {
            neighbours.push(pos);
        }
    }

    neighbours
}

struct PathfindingNode {
    pos: TilePos,
    distance: f32,
}

impl Ord for PathfindingNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.distance.partial_cmp(&self.distance).unwrap()
    }
}

impl PartialOrd for PathfindingNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PathfindingNode {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for PathfindingNode {}

pub fn pos_to_tile(
    pos: &Vec2,
    map_size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    trans: &Transform,
) -> Option<TilePos> {
    let pos = Vec4::new(pos.x, pos.y, 0., 1.);
    let pos_in_map = trans.compute_matrix().inverse() * pos;
    TilePos::from_world_pos(&pos_in_map.xy(), map_size, grid_size, map_type)
}

pub fn tile_to_pos(
    tile: &TilePos,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    trans: &Transform,
) -> Vec2 {
    let pos = tile
        .center_in_world(grid_size, map_type)
        .extend(0.)
        .extend(1.);
    let pos_in_map = trans.compute_matrix() * pos;
    pos_in_map.xy()
}

pub fn flip_from_rotation(rot: u32) -> TileFlip {
    match rot {
        1 => TileFlip {
            x: false,
            y: true,
            d: true,
        },
        2 => TileFlip {
            x: true,
            y: true,
            d: false,
        },
        3 => TileFlip {
            x: true,
            y: false,
            d: true,
        },
        _ => TileFlip {
            x: false,
            y: false,
            d: false,
        },
    }
}

pub fn play_to_real_size(play_size: &LevelSize) -> (TilemapSize, TilemapSize) {
    (
        TilemapSize {
            x: (MAP_SIZE.x - play_size.0.x) / 2,
            y: (MAP_SIZE.y - play_size.0.y) / 2,
        },
        play_size.0,
    )
}

pub fn tile_in_level(pos: &TilePos, level_size: &LevelSize) -> bool {
    let (offset, _) = play_to_real_size(level_size);
    pos.x >= offset.x
        && pos.x < offset.x + level_size.0.x
        && pos.y >= offset.y
        && pos.y < offset.y + level_size.0.y
}

pub fn fill_tilemap(
    tilemap_id: TilemapId,
    size: TilemapSize,
    storage: &mut TileStorage,
    visible: TileVisible,
    commands: &mut Commands,
) {
    commands.entity(tilemap_id.0).with_children(|parent| {
        for x in 0..size.x {
            for y in 0..size.y {
                let tile_pos = TilePos { x, y };
                let tile_entity = parent
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id,
                        visible,
                        ..Default::default()
                    })
                    .id();
                storage.set(&tile_pos, tile_entity);
            }
        }
    });
}
