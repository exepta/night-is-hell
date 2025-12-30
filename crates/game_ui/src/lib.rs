#![feature(coverage_attribute)]

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_extended_ui::{ExtendedUiConfiguration, ExtendedUiPlugin};
use bevy_extended_ui::html::HtmlSource;
use bevy_extended_ui::io::HtmlAsset;
use bevy_extended_ui::registry::UiRegistry;

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {

    #[coverage(off)]
    fn build(&self, app: &mut App) {
        app.insert_resource(ExtendedUiConfiguration {
            enable_default_camera: false,
            ..default()
        });
        app.add_plugins(ExtendedUiPlugin);
        app.add_systems(Startup, (spawn_my_ui_camera, test_ui));
    }
}

pub fn spawn_my_ui_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("My UI Camera (Layers 1,2)"),
        Camera2d,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::from_layers(&[1, 2]),
    ));
}

fn test_ui(mut reg: ResMut<UiRegistry>, asset_server: Res<AssetServer>) {
    let handle: Handle<HtmlAsset> = asset_server.load("html/debug_ui.html");
    reg.add_and_use("debug-ui".to_string(), HtmlSource::from_handle(handle));
}