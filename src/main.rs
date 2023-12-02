use bevy::{prelude::*, window::WindowResolution};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use charon::GamePlugin;

fn main() {
    App::new()
        .add_plugins((
            EmbeddedAssetPlugin {
                mode: PluginMode::ReplaceDefault,
            },
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Charon".to_string(),
                    resolution: WindowResolution::new(1000., 700.),
                    resizable: false, // Or use fit_canvas_to_parent: true for resizing on the web
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
