use crate::{
    config::{GameOptions, GameScore},
    menu::MenuState,
    ui::*,
    GameState,
};
use bevy::prelude::*;
use bevy_persistent::Persistent;

// ······
// Plugin
// ······

pub struct EndScreenPlugin;

impl Plugin for EndScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::End), init_end_screen)
            .add_systems(Update, handle_buttons.run_if(in_state(GameState::End)))
            .add_systems(OnExit(GameState::End), exit_end_screen);
    }
}

// ·······
// Systems
// ·······

fn init_end_screen(
    mut cmd: Commands,
    style: Res<UIStyle>,
    mut node: Query<Entity, With<UiNode>>,
    score: Res<Persistent<GameScore>>,
) {
    if let Ok(node) = node.get_single_mut() {
        if let Some(mut node) = cmd.get_entity(node) {
            node.with_children(|parent| {
                UIText::simple(&style, "Your journey has ended").add(parent);
                UIText::simple(
                    &style,
                    &format!(
                        "You helped {} entities find their way home",
                        if score.score > 0 {
                            // Dirty hack to avoid dealing with system ordering
                            score.score
                        } else {
                            score.last_score
                        }
                    ),
                )
                .add(parent);
                UIText::simple(&style, "Thank you").add(parent);

                UIButton::<UiNone>::new(&style, "Try again", None).add(parent);
            });
        }
    }
}

fn handle_buttons(
    mut menu_state: ResMut<NextState<MenuState>>,
    mut game_state: ResMut<NextState<GameState>>,
    mut text: Query<&mut Text>,
    mut buttons: Query<(&Interaction, &Children, &mut BackgroundColor), Changed<Interaction>>,
    opts: Res<Persistent<GameOptions>>,
) {
    for (inter, child, mut bg) in &mut buttons {
        let child = child.iter().next();
        if let Some(mut text) = child.and_then(|child| text.get_mut(*child).ok()) {
            match inter {
                Interaction::Pressed => {
                    bg.0 = opts.color.dark;
                    text.sections[0].style.color = opts.color.light;
                    // Go to the main menu
                    menu_state.set(MenuState::Main);
                    game_state.set(GameState::Menu);
                }
                Interaction::Hovered => {
                    bg.0 = opts.color.mid;
                    text.sections[0].style.color = opts.color.dark;
                }
                Interaction::None => {
                    bg.0 = opts.color.light;
                    text.sections[0].style.color = opts.color.dark;
                }
            }
        }
    }
}

fn exit_end_screen(mut cmd: Commands, mut node: Query<Entity, With<UiNode>>) {
    if let Ok(node) = node.get_single_mut() {
        if let Some(mut entity) = cmd.get_entity(node) {
            entity.despawn_descendants();
        }
    }
}
