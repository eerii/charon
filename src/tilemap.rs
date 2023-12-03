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

// TODO: Astar algorithm to generate pathfinding
// TODO: Run astar only when some tile change
// TODO: Automatically select path sprite (corner, straight, intersection)

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
                    pathfinding.run_if(resource_changed::<TileChanged>()),
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

#[derive(Component)]
pub struct StartTile;

#[derive(Component)]
pub struct EndTile;

#[derive(Component, Clone)]
pub struct PathTile(pub Option<f32>);

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
                .spawn((TileBundle {
                    position: pos,
                    tilemap_id: TilemapId(tilemap),
                    ..default()
                },))
                .id();
            storage.set(&pos, tile);
        }
    }

    cmd.entity(storage.get(&TilePos { x: 0, y: 3 }).unwrap())
        .insert(StartTile)
        .remove::<PathTile>();
    cmd.entity(storage.get(&TilePos { x: 14, y: 7 }).unwrap())
        .insert(EndTile)
        .remove::<PathTile>();

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

                // Erase path
                if path.is_some() {
                    cmd.entity(entity).remove::<PathTile>();
                    changed.0 -= 1;
                }
                // Add paths
                else if start.is_none() && end.is_none() {
                    cmd.entity(entity).insert(PathTile(None));
                    changed.0 += 1;

                    // After first and second path
                    if two_ago.is_some() {
                        let one = one_ago.as_ref().unwrap();
                        let two = two_ago.as_ref().unwrap();

                        if let Ok((size, storage)) = tilemap.get_single() {
                            let prev_neighbours = get_neighbours(two, size);

                            // If the new tile is also a neighbour of the one two ago, delete the previous one
                            if prev_neighbours.iter().any(|p| p == pos) {
                                let entity = storage.get(&one).unwrap();
                                cmd.entity(entity).remove::<PathTile>();
                                one_ago.replace(pos.clone());
                                return;
                            }
                        }
                    }
                    *two_ago = one_ago.replace(pos.clone());
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
        Option<&SelectedTile>,
        Option<&PathTile>,
        Option<&StartTile>,
        Option<&EndTile>,
    )>,
) {
    for (mut tex, mut color, selected, path, start, end) in tiles.iter_mut() {
        *color = TileColor::default();
        if selected.is_some() {
            *tex = TileTextureIndex(3);
        } else if path.is_some() {
            *tex = TileTextureIndex(3);
            if let Some(value) = path.unwrap().0 {
                *color = TileColor(Color::rgb(value as f32 / 20., 1. - value as f32 / 20., 0.));
            }
        } else if start.is_some() || end.is_some() {
            *tex = TileTextureIndex(2);
        } else {
            *tex = TileTextureIndex(0);
        }
    }
}

fn pathfinding(
    tilemap: Query<(&TilemapSize, &TileStorage)>,
    start: Query<&TilePos, With<StartTile>>,
    end: Query<&TilePos, With<EndTile>>,
    mut paths: Query<&mut PathTile>,
) {
    if let Ok((size, storage)) = tilemap.get_single() {
        if let (Ok(start), Ok(end)) = (start.get_single(), end.get_single()) {
            let mut open = PriorityQueue::new(end.clone());
            let mut closed = Vec::new();
            open.push(start.clone(), 0.);

            // Iterate the pathfinding queue
            while !open.is_empty() {
                let (current, g) = open.pop().unwrap();

                // Get the neighbouring tiles
                let neighbours = get_neighbours(&current, size);

                // TODO: Order with heuristic
                for neighbour in neighbours {
                    // If the tile is already closed, skip it
                    if !closed.contains(&neighbour) {
                        if let Some(entity) = storage.get(&neighbour) {
                            // If the goal is reached, stop
                            if neighbour == *end {
                                break;
                            }

                            match paths.get_mut(entity) {
                                // If the tile is a path, update its value and add it to the queue
                                Ok(mut path) => {
                                    path.0 = Some(g + 1.);
                                    open.push(neighbour, path.0.unwrap());
                                }
                                // If it is not, skip it
                                Err(_) => continue,
                            }
                        }
                    }
                }

                // Mark the current tile as done
                closed.push(current);
            }
        }
    }
}

// ·····
// Extra
// ·····

const DIRECTIONS: [SquareDirection; 4] = [
    SquareDirection::East,
    SquareDirection::North,
    SquareDirection::South,
    SquareDirection::West,
];

struct PriorityQueue {
    heap: Vec<(TilePos, f32)>,
    end: TilePos,
}

impl PriorityQueue {
    fn new(end: TilePos) -> Self {
        Self {
            heap: Vec::new(),
            end,
        }
    }

    fn push(&mut self, pos: TilePos, g: f32) {
        // TODO: Fix multiple paths
        if let Some((_, old_g)) = self.heap.iter().find(|(p, _)| p == &pos) {
            if g < *old_g {
                self.heap.retain(|(p, _)| p != &pos);
            } else {
                return;
            }
        }
        self.heap.push((pos, g));
        self.heap.sort_by(|(a_pos, a_g), (b_pos, b_g)| {
            astar_f(a_pos, &self.end, *a_g)
                .partial_cmp(&astar_f(b_pos, &self.end, *b_g))
                .unwrap()
        });
        self.heap.reverse();
    }

    fn pop(&mut self) -> Option<(TilePos, f32)> {
        self.heap.pop()
    }

    fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

pub fn get_neighbours(pos: &TilePos, size: &TilemapSize) -> Vec<TilePos> {
    let mut neighbours = Vec::new();

    for direction in DIRECTIONS.iter() {
        if let Some(pos) = pos.diamond_offset(direction, size) {
            neighbours.push(pos);
        }
    }

    neighbours
}

fn heuristic(pos: &TilePos, end: &TilePos) -> f32 {
    let dx = pos.x as f32 - end.x as f32;
    let dy = pos.y as f32 - end.y as f32;
    (dx * dx + dy * dy).sqrt()
}

fn astar_f(pos: &TilePos, end: &TilePos, g: f32) -> f32 {
    g + heuristic(pos, end)
}

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
