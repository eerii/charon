use bevy::{asset::AssetMetaCheck, prelude::*};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use charon::{GamePlugin, INITIAL_RESOLUTION};

fn main() {
    App::new()
        .insert_resource(AssetMetaCheck::Never)
        .add_plugins((
            EmbeddedAssetPlugin {
                mode: PluginMode::ReplaceDefault,
            },
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Charon".to_string(),
                    resolution: INITIAL_RESOLUTION.into(),
                    resizable: true,
                    fit_canvas_to_parent: true,
                    canvas: Some("#bevy".to_string()),
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
            GamePlugin,
        ))
        // Run
        .run();
}
