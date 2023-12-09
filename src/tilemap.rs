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
        app.insert_resource(TileChanged(0))
            .add_plugins(TilemapPlugin)
            .add_systems(OnEnter(GameState::Play), init_tilemap.run_if(run_once()))
            .add_systems(
                Update,
                (select_tile, click_tile).run_if(in_state(GameState::Play)),
            )
            .add_systems(
                PostUpdate,
                (
                    highlight_tile,
                    (autotile, pathfinding).run_if(resource_changed::<TileChanged>()),
                )
                    .run_if(in_state(GameState::Play)),
            );
    }
}

// ·········
// Resources
// ·········

#[derive(Resource)]
pub struct TileChanged(u32);

// ··········
// Components
// ··········

#[derive(Component)]
pub struct SelectedTile;

#[derive(Component, Default)]
pub struct StartTile {
    pub completed_once: bool,
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
    pub distance: f32,
    pub count: u32,
    pub shape: PathShape,
    pub rot: u32,
}

impl Default for PathTile {
    fn default() -> Self {
        Self {
            distance: f32::INFINITY,
            count: 0,
            shape: PathShape::End,
            rot: 0,
        }
    }
}

// ·······
// Systems
// ·······

fn init_tilemap(mut cmd: Commands, tile_assets: Res<TilemapAssets>) {
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

    // Mark start and end tiles
    cmd.entity(storage.get(&TilePos { x: 0, y: 3 }).unwrap())
        .insert((StartTile::default(), PathTile::default()));
    cmd.entity(storage.get(&TilePos { x: 14, y: 7 }).unwrap())
        .insert((
            EndTile,
            PathTile {
                distance: -std::f32::INFINITY,
                ..default()
            },
        ));

    // Create tilemap
    let map_type = TilemapType::default();
    cmd.entity(tilemap).insert(TilemapBundle {
        size: MAP_SIZE,
        tile_size: TILE_SIZE,
        grid_size: GRID_SIZE,
        map_type,
        storage,
        texture: TilemapTexture::Single(tile_assets.tiles.clone()),
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

    for (map_size, grid_size, map_type, tile_storage, trans) in tilemap.iter() {
        if let Some(tile_pos) = pos_to_tile(&mouse.0, map_size, grid_size, map_type, trans) {
            if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                cmd.entity(tile_entity).insert(SelectedTile);
            }
        }
    }
}

fn click_tile(
    mut cmd: Commands,
    mut selected: Query<Entity, With<SelectedTile>>,
    tiles: Query<(
        &TilePos,
        Option<&PathTile>,
        Option<&StartTile>,
        Option<&EndTile>,
    )>,
    tilemap: Query<(&TilemapSize, &TileStorage)>,
    input: Res<Input<Bind>>,
    keybinds: Res<Persistent<Keybinds>>,
    mut changed: ResMut<TileChanged>,
    mut prev: Local<Option<(bool, Option<TilePos>, Option<TilePos>)>>,
) {
    let select = keybinds.interact.iter().any(|bind| {
        if prev.is_none() {
            input.just_pressed(*bind)
        } else {
            input.pressed(*bind)
        }
    });

    if select {
        if let Ok(entity) = selected.get_single_mut() {
            if let Ok((pos, path, start, end)) = tiles.get(entity) {
                if prev.is_none() {
                    *prev = Some((path.is_some(), None, None));
                }
                let (is_path, one_ago, two_ago) = prev.as_mut().unwrap();

                if path.is_some() != *is_path {
                    return;
                }

                if start.is_none() && end.is_none() {
                    // Erase path
                    if path.is_some() {
                        cmd.entity(entity).remove::<PathTile>();
                        changed.0 -= 1;
                        return;
                    }

                    // Add paths
                    cmd.entity(entity).insert(PathTile::default());
                    changed.0 += 1;

                    // After first and second path
                    if two_ago.is_some() {
                        let one = one_ago.as_ref().unwrap();
                        let two = two_ago.as_ref().unwrap();

                        if let Ok((size, storage)) = tilemap.get_single() {
                            let prev_neighbours = get_neighbours(two, size);

                            // If the new tile is also a neighbour of the one two ago, delete the previous one
                            if prev_neighbours.iter().any(|p| p == pos) {
                                let entity = storage.get(one).unwrap();
                                cmd.entity(entity).remove::<PathTile>();
                                one_ago.replace(*pos);
                                return;
                            }
                        }
                    }
                    *two_ago = one_ago.replace(*pos);
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
        Option<&SelectedTile>,
        Option<&PathTile>,
        Option<&StartTile>,
        Option<&EndTile>,
    )>,
) {
    for (mut tex, mut color, mut flip, selected, path, start, end) in tiles.iter_mut() {
        *color = TileColor::default();
        if selected.is_some() {
            *tex = TileTextureIndex(3);
        } else if start.is_some() {
            *tex = TileTextureIndex(2);
        } else if end.is_some() {
            *tex = TileTextureIndex(1);
        } else if path.is_some() {
            let dist = path.unwrap().distance;
            *color = if dist < std::f32::INFINITY {
                TileColor(Color::rgb(dist / 25., 1. - dist / 25., (dist - 20.) / 50.))
            } else {
                TileColor::default()
            };
            *tex = match path.unwrap().shape {
                PathShape::None => TileTextureIndex(3),
                PathShape::End => TileTextureIndex(4),
                PathShape::Straight => TileTextureIndex(5),
                PathShape::Turn => TileTextureIndex(6),
                PathShape::Junction => TileTextureIndex(7),
                PathShape::Crossing => TileTextureIndex(8),
            };
            *flip = flip_from_rotation(path.unwrap().rot);
        } else {
            *tex = TileTextureIndex(0);
        }
    }
}

fn pathfinding(
    tilemap: Query<(&TilemapSize, &TileStorage)>,
    mut start: Query<(&TilePos, &mut StartTile)>,
    end: Query<&TilePos, With<EndTile>>,
    mut paths: Query<&mut PathTile>,
) {
    if let Ok((size, storage)) = tilemap.get_single() {
        if let (Ok((start_pos, mut start_tile)), Ok(end_pos)) =
            (start.get_single_mut(), end.get_single())
        {
            let mut open = BinaryHeap::new();
            let mut distances = HashMap::new();

            open.push(PathfindingNode {
                pos: *end_pos,
                distance: 0.,
            });
            distances.insert(*end_pos, 0.);

            while let Some(PathfindingNode { pos, distance }) = open.pop() {
                // Get the neighbouring tiles
                let neighbours = get_neighbours(&pos, size);

                for neighbour in neighbours {
                    if let Some(entity) = storage.get(&neighbour) {
                        if let Ok(mut path) = paths.get_mut(entity) {
                            // Djikstra's algorithm to find the shortest path from each tile
                            let dist = distance + 1.;
                            if dist < *distances.get(&neighbour).unwrap_or(&f32::INFINITY) {
                                distances.insert(neighbour, dist);
                                open.push(PathfindingNode {
                                    pos: neighbour,
                                    distance: dist,
                                });
                                path.distance = dist * 2.;
                            }
                        }
                    }
                }
            }

            // Check if there is a path from the end to the start
            if distances.contains_key(start_pos) {
                start_tile.completed_once = true;

                // Then do the algorithm in reverse to favour paths away from the start
                open.push(PathfindingNode {
                    pos: *start_pos,
                    distance: 0.,
                });
                let mut distances = HashMap::new();
                distances.insert(*start_pos, 0.);

                while let Some(PathfindingNode { pos, distance }) = open.pop() {
                    let neighbours = get_neighbours(&pos, size);

                    for neighbour in neighbours {
                        if let Some(entity) = storage.get(&neighbour) {
                            if let Ok(mut path) = paths.get_mut(entity) {
                                let dist = distance + 1.;
                                if dist < *distances.get(&neighbour).unwrap_or(&f32::INFINITY) {
                                    distances.insert(neighbour, dist);
                                    open.push(PathfindingNode {
                                        pos: neighbour,
                                        distance: dist,
                                    });
                                    path.distance -= dist * 0.5;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn autotile(
    tilemap: Query<(&TilemapSize, &TileStorage)>,
    mut paths: Query<(&TilePos, &mut PathTile)>,
) {
    if let Ok((size, storage)) = tilemap.get_single() {
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
