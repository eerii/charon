use bevy::{asset::AssetMetaCheck, prelude::*, window::WindowResolution};
use charon::GamePlugin;

fn main() {
    App::new()
        .insert_resource(AssetMetaCheck::Never)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Charon".to_string(),
                    resolution: WindowResolution::new(1080., 720.),
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
