use bevy::prelude::*;

use crate::{game::GameScore, load::GameAssets, tilemap::TilesAvailable, ui::*, GameState};

// ······
// Plugin
// ······

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Play), init_hud)
            .add_systems(Update, update_hud.run_if(in_state(GameState::Play)))
            .add_systems(OnExit(GameState::Play), exit_hud);
    }
}

// ··········
// Components
// ··········

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct TilesText;

// ·······
// Systems
// ·······

fn init_hud(mut cmd: Commands, assets: Res<GameAssets>, mut node: Query<Entity, With<UiNode>>) {
    // Main menu layout
    if let Ok(node) = node.get_single_mut() {
        if let Some(mut node) = cmd.get_entity(node) {
            node.with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(5.0),
                            top: Val::Px(5.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(4.),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|tiles| {
                        tiles.spawn(ImageBundle {
                            image: UiImage {
                                texture: assets.river_icon.clone(),
                                ..default()
                            },
                            style: Style {
                                width: Val::Px(24.),
                                ..default()
                            },
                            ..default()
                        });

                        tiles.spawn((
                            TextBundle::from_section(
                                "0",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TilesText,
                        ));
                    });

                parent
                    .spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right: Val::Px(5.0),
                            top: Val::Px(5.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(4.),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|score| {
                        score.spawn((
                            TextBundle::from_section(
                                "0",
                                TextStyle {
                                    font: assets.font.clone(),
                                    font_size: 24.0,
                                    color: Color::WHITE,
                                },
                            ),
                            ScoreText,
                        ));

                        score.spawn(ImageBundle {
                            image: UiImage {
                                texture: assets.coin_icon.clone(),
                                ..default()
                            },
                            style: Style {
                                width: Val::Px(24.),
                                ..default()
                            },
                            ..default()
                        });
                    });
            });
        }
    }
}

fn update_hud(
    score: Res<GameScore>,
    mut score_text: Query<&mut Text, (With<ScoreText>, Without<TilesText>)>,
    tiles: Res<TilesAvailable>,
    mut tiles_text: Query<&mut Text, (With<TilesText>, Without<ScoreText>)>,
) {
    for mut text in score_text.iter_mut() {
        text.sections[0].value = format!("{}", score.score);
    }
    for mut text in tiles_text.iter_mut() {
        text.sections[0].value = format!("{}", tiles.0);
    }
}

fn exit_hud(mut cmd: Commands, score: Query<Entity, With<ScoreText>>) {
    for score in score.iter() {
        cmd.entity(score).despawn_recursive();
    }
}
