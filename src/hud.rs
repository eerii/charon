use bevy::prelude::*;

use crate::{game::GameScore, load::GameAssets, ui::*, GameState};

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

// ·······
// Systems
// ·······

fn init_hud(mut cmd: Commands, assets: Res<GameAssets>, mut node: Query<Entity, With<UiNode>>) {
    // Main menu layout
    if let Ok(node) = node.get_single_mut() {
        if let Some(mut node) = cmd.get_entity(node) {
            node.with_children(|parent| {
                parent.spawn((
                    TextBundle::from_section(
                        "0",
                        TextStyle {
                            font: assets.font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_style(Style {
                        position_type: PositionType::Absolute,
                        right: Val::Px(5.0),
                        top: Val::Px(5.0),
                        ..default()
                    }),
                    ScoreText,
                ));
            });
        }
    }
}

fn update_hud(score: Res<GameScore>, mut text: Query<&mut Text, With<ScoreText>>) {
    for mut text in text.iter_mut() {
        text.sections[0].value = format!("{}", score.score);
    }
}

fn exit_hud(mut cmd: Commands, score: Query<Entity, With<ScoreText>>) {
    for score in score.iter() {
        cmd.entity(score).despawn_recursive();
    }
}
